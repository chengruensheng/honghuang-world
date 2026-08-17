//! 调用 - 落回 - 园：调用模型，解析落回内容。

use crate::类型_定义_殿::{对话消息, 工具定义, 工具调用, 模型回复, 用量};
use peizhi_fu::模型配置;
use rizhi_fu::{error, info, warn};

/// 输出上限分级（M4 实测：落盘大文件 16384 会被 think 吃光致空输出，须 ≥32768，取 65536 稳；
/// 小任务 131072 一刀切会慢 3 倍，分级后各取所需）。
/// 落盘上限：派遣执行写文件（多文件 + think 兜底）。
pub const 落盘上限: u32 = 65536;
/// 常规上限：解析想法等结构化 JSON 输出。
pub const 常规上限: u32 = 16384;
/// 精简上限：读现状 / 播种归纳 / 补解释等短输出。
pub const 精简上限: u32 = 4096;

/// 请求超时：单次 HTTP 请求 120 秒上限（ureq 默认无超时，网络挂起会无限卡死整轮任务——
/// 2026-08-17 实测：任务线驱动在模型设计阶段挂起 5 分钟无进展，CPU 0.08 静止，事件流停滞）。
pub const 请求超时: std::time::Duration = std::time::Duration::from_secs(120);

/// DNS 预检：请求前解析 host，快速失败并给出明确错误。
/// ureq 的 DNS 失败信息含糊且慢（实测 os error 11001 把任务整轮打挂 3 次）；预检在发请求前
/// 用系统解析器确认 host 可达，失败直接返回（DNS 失败重试无益，不进入指数退避）。
pub fn 预检地址(地址: &str) -> Result<(), String> {
    let 无协议 = 地址.trim_start_matches("https://").trim_start_matches("http://");
    let 主机端口 = 无协议.split('/').next().unwrap_or(无协议);
    let (主机, 端口) = match 主机端口.rsplit_once(':') {
        Some((主机, 端口)) if 端口.chars().all(|字| 字.is_ascii_digit()) => {
            (主机, 端口.parse::<u16>().unwrap_or(443))
        }
        _ => (主机端口, 443),
    };
    use std::net::ToSocketAddrs;
    (主机, 端口)
        .to_socket_addrs()
        .map_err(|错误| format!("DNS 解析失败：{主机}:{端口}（{错误}）——请检查网络连接与域名可用性"))?;
    Ok(())
}

/// 瞬时故障判定：HTTP 5xx（服务端错误）与 429/529（过载）可重试；
/// 4xx 与其他错误不重试（重试无益，直接失败更快暴露问题）。
pub fn 是瞬时故障(状态码: u16) -> bool {
    状态码 == 429 || 状态码 == 529 || (500..=599).contains(&状态码)
}

/// 指数退避间隔（秒）：第 n 次重试前等待，n=0→1s、n=1→2s、n=2→4s，超出按 8s 兜底。
pub fn 退避间隔(重试次数: u32) -> u64 {
    [1, 2, 4].get(重试次数 as usize).copied().unwrap_or(8)
}

/// 瞬时故障重试上限（最多 3 次重试，即最多 4 次尝试）。
pub const 最大重试次数: u32 = 3;

/// 发送请求并做指数退避重试：仅对瞬时故障（5xx/429/529）重试，
/// 每次失败前 sleep 退避间隔；其余错误直接返回。防止 API 过载把整轮任务拖垮。
fn 发送并重试<F>(发请求: F, 模型: &str) -> Result<ureq::Response, String>
where
    F: Fn() -> Result<ureq::Response, ureq::Error>,
{
    let mut 重试次数 = 0u32;
    loop {
        match 发请求() {
            Ok(响应) => return Ok(响应),
            Err(错误) => {
                let 可重试 = match &错误 {
                    ureq::Error::Status(状态码, _) => 是瞬时故障(*状态码),
                    ureq::Error::Transport(_) => false,
                };
                if !可重试 {
                    let 详情 = 请求错误详情(错误);
                    error!(模型 = %模型, "模型请求失败：{详情}");
                    return Err(format!("模型请求失败: {详情}"));
                }
                if 重试次数 >= 最大重试次数 {
                    let 详情 = 请求错误详情(错误);
                    error!(模型 = %模型, 重试次数, "瞬时故障重试耗尽仍失败：{详情}");
                    return Err(format!("模型请求失败(重试{最大重试次数}次后仍瞬时故障): {详情}"));
                }
                let 间隔 = 退避间隔(重试次数);
                重试次数 += 1;
                warn!(模型 = %模型, 重试次数, 间隔, "模型瞬时故障，退避后重试");
                std::thread::sleep(std::time::Duration::from_secs(间隔));
            }
        }
    }
}

/// 模型请求错误转文本：状态概要 + 响应体（4xx/5xx 具体原因），便于诊断。
fn 请求错误详情(错误: ureq::Error) -> String {
    let 概要 = 错误.to_string();
    let 响应体 = match 错误.into_response() {
        Some(响应) => 响应.into_string().unwrap_or_default(),
        None => String::new(),
    };
    if 响应体.is_empty() {
        概要
    } else {
        format!("{概要}；响应体：{响应体}")
    }
}

/// 一次模型调用：发送消息，取回文本内容与用量。
pub fn 调用模型(配置: &模型配置, 消息们: &[对话消息], 输出上限: u32) -> Result<(String, 用量), String> {
    预检地址(&配置.地址)?;
    let 请求体 = 构造请求体(配置, 消息们, 输出上限);

    let 响应 = 发送并重试(
        || {
            ureq::post(&配置.地址)
                .timeout(请求超时)
                .set("Authorization", &format!("Bearer {}", 配置.密钥))
                .set("Content-Type", "application/json")
                .send_string(&请求体)
        },
        &配置.模型,
    )?;

    let 文本 = 响应.into_string().map_err(|错误| {
        error!("读取响应失败：{错误}");
        format!("读取响应失败: {错误}")
    })?;

    let 内容 = 解析回复(&文本)?;
    let 用量 = 解析用量(&文本);
    落盘收发(消息们, &内容, &用量);
    info!(
        模型 = %配置.模型, 内容长度 = 内容.len(),
        提示词 = 用量.提示词, 输出 = 用量.输出, 缓存命中 = 用量.缓存命中,
        "模型返回正常"
    );
    Ok((内容, 用量))
}

/// 调用模型并允许工具调用（function calling）：发送消息与工具定义，落回文本或工具调用（附用量）。
pub fn 调用模型带工具(
    配置: &模型配置,
    消息们: &[对话消息],
    工具们: &[工具定义],
    输出上限: u32,
) -> Result<(模型回复, 用量), String> {
    预检地址(&配置.地址)?;
    let 请求体 = 构造工具请求体(配置, 消息们, 工具们, 输出上限);

    let 响应 = 发送并重试(
        || {
            ureq::post(&配置.地址)
                .timeout(请求超时)
                .set("Authorization", &format!("Bearer {}", 配置.密钥))
                .set("Content-Type", "application/json")
                .send_string(&请求体)
        },
        &配置.模型,
    )?;

    let 文本 = 响应.into_string().map_err(|错误| {
        error!("读取响应失败：{错误}");
        format!("读取响应失败: {错误}")
    })?;

    let 回复 = 解析工具回复(&文本)?;
    let 用量 = 解析用量(&文本);
    落盘收发(消息们, &文本, &用量);
    info!(
        模型 = %配置.模型, 内容长度 = 文本.len(),
        提示词 = 用量.提示词, 输出 = 用量.输出, 缓存命中 = 用量.缓存命中,
        "模型返回（工具模式）"
    );
    Ok((回复, 用量))
}

/// 从 OpenAI 兼容响应解析 usage（提示词 / 输出 / 缓存命中）。
/// MiniMax 返回 usage.cache_read_input_tokens；OpenAI 兼容返回 usage.prompt_tokens_details.cached_tokens。
pub fn 解析用量(文本: &str) -> 用量 {
    let Ok(解析) = serde_json::from_str::<serde_json::Value>(文本) else {
        return 用量::default();
    };
    let 用量值 = &解析["usage"];
    let 提示词 = 用量值["prompt_tokens"].as_u64().unwrap_or(0);
    let 输出 = 用量值["completion_tokens"].as_u64().unwrap_or(0);
    let 缓存命中 = 用量值["cache_read_input_tokens"]
        .as_u64()
        .or_else(|| 用量值["prompt_tokens_details"]["cached_tokens"].as_u64())
        .unwrap_or(0);
    let 总计 = 用量值["total_tokens"].as_u64().unwrap_or(提示词 + 输出);
    用量 { 提示词, 输出, 缓存命中, 总计 }
}

/// 构造 OpenAI 兼容请求体（独立出便于测试）。
pub fn 构造请求体(配置: &模型配置, 消息们: &[对话消息], 输出上限: u32) -> String {
    serde_json::json!({
        "model": 配置.模型,
        "messages": 消息们.iter().map(|消息| serde_json::json!({
            "role": 消息.角色,
            "content": 消息.内容,
        })).collect::<Vec<_>>(),
        "max_tokens": 输出上限,
    })
    .to_string()
}

/// 构造带工具定义的 OpenAI 兼容请求体（独立出便于测试）。
/// 显式声明 temperature 与 thinking：服务端默认值漂移曾致 MiniMax-M3 工具参数退化
/// （arguments={} 缺必填字段，见设计稿 4.1-4），显式固定可复现、可收敛。
pub fn 构造工具请求体(配置: &模型配置, 消息们: &[对话消息], 工具们: &[工具定义], 输出上限: u32) -> String {
    serde_json::json!({
        "model": 配置.模型,
        "messages": 消息们_带工具(消息们),
        "tools": 工具们.iter().map(|工具| serde_json::json!({
            "type": "function",
            "function": {
                "name": 工具.名字,
                "description": 工具.描述,
                "parameters": 工具.参数,
            }
        })).collect::<Vec<_>>(),
        "max_tokens": 输出上限,
        "temperature": 0.2,
        "thinking": {"type": "adaptive"},
    })
    .to_string()
}

/// 从 OpenAI 兼容响应解析 choices[0].message.content。
pub fn 解析回复(文本: &str) -> Result<String, String> {
    let 解析: serde_json::Value =
        serde_json::from_str(文本).map_err(|错误| format!("解析响应失败: {错误}"))?;
    match 解析["choices"][0]["message"]["content"].as_str() {
        Some(段) if !段.trim().is_empty() => Ok(段.to_string()),
        // 空内容兜底：content 存在但为空白串（think 吃光输出 / 模型抽风）同样视为失败，
        // 否则各调用方会把空串当正常回复：静默写空记忆、空 JSON 报错、空文本收敛。
        Some(_) => {
            warn!("响应内容为空，原响应长度：{}", 文本.len());
            Err("模型返回空内容".to_string())
        }
        None => {
            warn!("响应缺少内容字段，原响应长度：{}", 文本.len());
            Err("响应缺少 choices[0].message.content".to_string())
        }
    }
}

/// 从 OpenAI 兼容响应解析 choices[0].message：优先取工具调用，否则取文本内容。
pub fn 解析工具回复(文本: &str) -> Result<模型回复, String> {
    let 解析: serde_json::Value =
        serde_json::from_str(文本).map_err(|错误| format!("解析响应失败: {错误}"))?;
    let 消息 = &解析["choices"][0]["message"];

    if let Some(调用们) = 消息["tool_calls"].as_array() {
        let 工具调用们 = 调用们
            .iter()
            .filter_map(|调用| {
                Some(工具调用 {
                    标识: 调用["id"].as_str().unwrap_or("").to_string(),
                    名字: 调用["function"]["name"].as_str()?.to_string(),
                    参数: 调用["function"]["arguments"].as_str()?.to_string(),
                })
            })
            .collect::<Vec<_>>();
        if 工具调用们.is_empty() {
            warn!("响应含 tool_calls 但无有效调用");
        }
        // 空参兜底：arguments 为空、{} 空对象或非法 JSON 的调用不执行，返回 参数缺失 引导模型重发完整参数。
        let 缺失名字们: Vec<String> = 工具调用们
            .iter()
            .filter(|调用| 参数缺失(&调用.参数))
            .map(|调用| 调用.名字.clone())
            .collect();
        if !缺失名字们.is_empty() {
            warn!(工具们 = ?缺失名字们, "工具调用参数缺失，返回参数缺失反馈");
            return Ok(模型回复::参数缺失(缺失名字们));
        }
        // 内容保留模型原回复（含 think），多轮回传须随 assistant 消息带回。
        let 内容 = 消息["content"].as_str().unwrap_or("").to_string();
        return Ok(模型回复::工具调用(内容, 工具调用们));
    }

    match 消息["content"].as_str() {
        Some(段) => Ok(模型回复::文本(段.to_string())),
        None => {
            warn!("响应缺少内容与工具调用，原响应长度：{}", 文本.len());
            Err("响应缺少 choices[0].message.content".to_string())
        }
    }
}

/// 判定工具参数字符串是否缺失：空串、`{}` 空对象、非法 JSON。
fn 参数缺失(参数: &str) -> bool {
    let 修剪 = 参数.trim();
    if 修剪.is_empty() || 修剪 == "{}" {
        return true;
    }
    serde_json::from_str::<serde_json::Value>(修剪).is_err()
}

/// 从模型回复中提取 JSON：
/// 优先取 ```json 围栏块；否则剥掉 <think>...</think> 思维链（防其内部的示例 JSON 抢先命中），再括号配平首个对象。
/// 供 解析想法 / 读现状 等结构化落回共用（跨府经 lib 根引用，勿在各府重复实现）。
pub fn 提取对象(文本: &str) -> Result<String, String> {
    let 文本 = 文本.trim();
    // 1) 有围栏块时直接取围栏内内容
    if let Some(围栏) = 文本.find("```") {
        let 余 = &文本[围栏 + 3..];
        let 余 = 余.strip_prefix("json").unwrap_or(余).trim_start();
        if let Some(对象) = 配平首个对象(余) {
            return Ok(对象.to_string());
        }
    }
    // 2) 剥掉 <think>...</think> 思维链再找首个对象
    let 去think = if let (Some(开), Some(闭)) = (文本.find("<think>"), 文本.find("</think>")) {
        let mut 拼接 = String::from(&文本[..开]);
        拼接.push_str(&文本[闭 + "</think>".len()..]);
        拼接
    } else {
        文本.to_string()
    };
    配平首个对象(&去think)
        .map(|对象| 对象.to_string())
        .ok_or_else(|| format!("模型未返回 JSON：{文本}"))
}

/// 从第一个 { 起括号配平（跳过字符串），返回首个完整 JSON 对象切片。
fn 配平首个对象(文本: &str) -> Option<&str> {
    let 开始 = 文本.find('{')?;
    let 字节 = 文本.as_bytes();
    let mut 深度 = 0i32;
    let mut 在字符串 = false;
    let mut 转义 = false;
    for (序号, &字符) in 字节.iter().enumerate().skip(开始) {
        if 在字符串 {
            if 转义 {
                转义 = false;
            } else if 字符 == b'\\' {
                转义 = true;
            } else if 字符 == b'"' {
                在字符串 = false;
            }
            continue;
        }
        match 字符 {
            b'"' => 在字符串 = true,
            b'{' => 深度 += 1,
            b'}' => {
                深度 -= 1;
                if 深度 == 0 {
                    return Some(&文本[开始..=序号]);
                }
            }
            _ => {}
        }
    }
    None
}

/// 观测：把一次模型调用的完整提示词、回复原文与 token 用量追加落盘到 临时文件夹/模型流水-观测.log。
/// 仅用于投递过程复盘（界主要看完整收发流）；写失败静默，不阻断主流程。
fn 落盘收发(消息们: &[对话消息], 回复: &str, 用量: &用量) {
    let 根 = std::env::var("WORLD_WORKSPACE_ROOT").unwrap_or_default();
    if 根.is_empty() {
        return;
    }
    let 路径 = std::path::Path::new(&根).join("临时文件夹").join("模型流水-观测.log");
    let 时刻 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|时距| 时距.as_millis())
        .unwrap_or(0);
    let mut 块 = format!(
        "\n========== 模型调用 @ {时刻} ==========\n【用量】提示词={} 输出={} 缓存命中={} 总计={}\n",
        用量.提示词, 用量.输出, 用量.缓存命中, 用量.总计
    );
    for 消息 in 消息们 {
        块.push_str(&format!("【{}】\n{}\n\n", 消息.角色, 消息.内容));
        if let Some(调用们) = &消息.工具调用们 {
            块.push_str(&format!("【工具调用】{:?}\n\n", 调用们));
        }
    }
    块.push_str("【回复】\n");
    块.push_str(回复);
    块.push('\n');
    if let Ok(mut 文件) = std::fs::OpenOptions::new().create(true).append(true).open(&路径) {
        use std::io::Write;
        let _ = 文件.write_all(块.as_bytes());
    }
}

/// 消息序列化：工具链路用（额外携带 tool_calls / tool_call_id 回传）。
fn 消息们_带工具(消息们: &[对话消息]) -> Vec<serde_json::Value> {
    消息们.iter().map(|消息| {
        let mut 对象 = serde_json::json!({
            "role": 消息.角色,
            "content": 消息.内容,
        });
        if let Some(调用们) = &消息.工具调用们 {
            对象["tool_calls"] = serde_json::json!(调用们.iter().map(|调用| {
                serde_json::json!({
                    "id": 调用.标识,
                    "type": "function",
                    "function": { "name": 调用.名字, "arguments": 调用.参数 }
                })
            }).collect::<Vec<_>>());
        }
        if let Some(标识) = &消息.工具调用标识 {
            对象["tool_call_id"] = serde_json::json!(标识);
        }
        对象
    }).collect()
}

/// 模型提供者：换模型不改主循环的抽象契约（对齐 DeepSeek「模型无关」）。
/// 本质：任何 LLM 提供者的通用协议——发消息（可带工具）→ 回复 + 用量。
/// HTTP 走 OpenAI 兼容；模拟提供者供测试/离线演练，不耗真实 API。
pub trait 模型提供者 {
    fn 调用(&self, 配置: &模型配置, 消息们: &[对话消息], 输出上限: u32) -> Result<(String, 用量), String>;
    fn 调用带工具(&self, 配置: &模型配置, 消息们: &[对话消息], 工具们: &[工具定义], 输出上限: u32) -> Result<(模型回复, 用量), String>;
}

/// HTTP 模型提供者：走 OpenAI 兼容 HTTP（当前 MiniMax-M3），退避重试 + 解析。
pub struct HTTP模型提供者;

impl 模型提供者 for HTTP模型提供者 {
    fn 调用(&self, 配置: &模型配置, 消息们: &[对话消息], 输出上限: u32) -> Result<(String, 用量), String> {
        调用模型(配置, 消息们, 输出上限)
    }
    fn 调用带工具(&self, 配置: &模型配置, 消息们: &[对话消息], 工具们: &[工具定义], 输出上限: u32) -> Result<(模型回复, 用量), String> {
        调用模型带工具(配置, 消息们, 工具们, 输出上限)
    }
}

/// 模拟模型提供者：返回固定文本，供测试/离线演练，不耗真实 API。
pub struct 模拟模型提供者 {
    pub 回复文本: String,
}

impl 模型提供者 for 模拟模型提供者 {
    fn 调用(&self, _配置: &模型配置, _消息们: &[对话消息], _输出上限: u32) -> Result<(String, 用量), String> {
        Ok((self.回复文本.clone(), 用量::default()))
    }
    fn 调用带工具(&self, _配置: &模型配置, _消息们: &[对话消息], _工具们: &[工具定义], _输出上限: u32) -> Result<(模型回复, 用量), String> {
        Ok((模型回复::文本(self.回复文本.clone()), 用量::default()))
    }
}

#[cfg(test)]
mod 测试 {
    use super::{退避间隔, 提取对象, 解析用量, 最大重试次数, 是瞬时故障, 预检地址};

    #[test]
    fn 预检地址_本机地址通过() {
        assert!(预检地址("http://127.0.0.1:8080/v1/chat/completions").is_ok());
        assert!(预检地址("https://localhost:443/x").is_ok());
    }

    #[test]
    fn 预检地址_非法域名快速失败() {
        // 不可解析域名（.invalid 保留后缀）应快速报错，不进入 HTTP 层。
        let 错误 = 预检地址("https://肯定不存在的域名-abc123.invalid/v1/chat").unwrap_err();
        assert!(错误.contains("DNS 解析失败"), "应给出明确 DNS 错误：{错误}");
    }

    #[test]
    fn 解析用量兼容两种缓存字段() {
        // MiniMax 风格：usage.cache_read_input_tokens
        let minimax = r#"{"usage":{"prompt_tokens":1000,"completion_tokens":200,"total_tokens":1200,"cache_read_input_tokens":800}}"#;
        let 用量 = 解析用量(minimax);
        assert_eq!(用量.提示词, 1000);
        assert_eq!(用量.输出, 200);
        assert_eq!(用量.缓存命中, 800);
        assert_eq!(用量.总计, 1200);
        // OpenAI 兼容风格：usage.prompt_tokens_details.cached_tokens
        let openai = r#"{"usage":{"prompt_tokens":500,"completion_tokens":50,"total_tokens":550,"prompt_tokens_details":{"cached_tokens":300}}}"#;
        let 用量 = 解析用量(openai);
        assert_eq!(用量.提示词, 500);
        assert_eq!(用量.缓存命中, 300);
        assert_eq!(用量.总计, 550);
    }

    #[test]
    fn 缺usage或缺total_tokens时兜底() {
        let 无total = r#"{"usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
        let 用量 = 解析用量(无total);
        assert_eq!(用量.总计, 15);
        let 无usage = r#"{"choices":[]}"#;
        assert_eq!(解析用量(无usage).总计, 0);
    }

    #[test]
    fn think块内示例_json不抢先命中() {
        // 复现真实事故：LLM 回复带 <think>，思维链里复述了提示词模板的示例 JSON。
        let 回复 = r#"<think>Let me structure this as JSON:
{
  "方向": "一句话目标",
  "类别": "功能|性能",
  "验收标准": "可核对的完成判据",
  "涉及路径": ["要改的符号名"]
}
Wait, let me output the real JSON.</think>

```json
{
  "方向": "优化扫描链路性能，保持外部行为一致",
  "类别": "性能",
  "验收标准": "cargo build 通过",
  "涉及路径": ["扫描-落格位-园", "依赖-边-园"]
}
```"#;
        let 对象 = 提取对象(回复).expect("应能提取到围栏内的真实 JSON");
        let 解析: serde_json::Value = serde_json::from_str(&对象).expect("提取结果应可解析");
        assert_eq!(解析["方向"], "优化扫描链路性能，保持外部行为一致");
        assert_eq!(解析["类别"], "性能");
    }

    #[test]
    fn 无围栏无think直接取首个对象() {
        let 回复 = r#"{"方向":"直接JSON","类别":"功能"}"#;
        let 对象 = 提取对象(回复).expect("应能提取 JSON");
        let 解析: serde_json::Value = serde_json::from_str(&对象).expect("应可解析");
        assert_eq!(解析["方向"], "直接JSON");
    }

    #[test]
    fn 瞬时故障判定只认5xx与429_529() {
        assert!(是瞬时故障(500));
        assert!(是瞬时故障(502));
        assert!(是瞬时故障(503));
        assert!(是瞬时故障(429));
        assert!(是瞬时故障(529));
        assert!(!是瞬时故障(400));
        assert!(!是瞬时故障(401));
        assert!(!是瞬时故障(404));
        assert!(!是瞬时故障(200));
        assert!(!是瞬时故障(408));
    }

    #[test]
    fn 退避间隔指数递增() {
        assert_eq!(退避间隔(0), 1);
        assert_eq!(退避间隔(1), 2);
        assert_eq!(退避间隔(2), 4);
        assert_eq!(退避间隔(3), 8, "超出内置间隔按 8s 兜底");
        assert_eq!(退避间隔(100), 8);
    }

    #[test]
    fn 瞬时故障重试上限为三次() {
        assert_eq!(最大重试次数, 3, "最多 3 次重试，即最多 4 次尝试");
    }

    #[test]
    fn 模拟提供者返回固定文本不耗api() {
        use super::{模型提供者, 模拟模型提供者, 模型回复};
        let 模拟 = 模拟模型提供者 { 回复文本: "固定回复".to_string() };
        let 配置 = peizhi_fu::模型配置 {
            密钥: String::new(),
            地址: String::new(),
            模型: String::new(),
        };
        // 纯文本调用返回固定文本 + 空用量。
        let (文本, 用量) = 模拟.调用(&配置, &[], 100).unwrap();
        assert_eq!(文本, "固定回复");
        assert_eq!(用量, super::用量::default());
        // 带工具调用也返回固定文本。
        let (回复, _) = 模拟.调用带工具(&配置, &[], &[], 100).unwrap();
        assert!(matches!(回复, 模型回复::文本(内容) if 内容 == "固定回复"));
    }
}
