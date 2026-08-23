//! 调用 - 落回 - 园：调用模型，解析落回内容。

use crate::类型_定义_殿::{对话消息, 工具定义, 工具调用, 模型回复, 用量};
use jiance_fu::{当前观测, 记回复, 记请求};
use peizhi_fu::模型配置;
use rizhi_fu::{debug, error, warn};

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
    let 无协议 = 地址
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let 主机端口 = 无协议.split('/').next().unwrap_or(无协议);
    let (主机, 端口) = match 主机端口.rsplit_once(':') {
        Some((主机, 端口)) if 端口.chars().all(|字| 字.is_ascii_digit()) => {
            (主机, 端口.parse::<u16>().unwrap_or(443))
        }
        _ => (主机端口, 443),
    };
    use std::net::ToSocketAddrs;
    (主机, 端口).to_socket_addrs().map_err(|错误| {
        format!("DNS 解析失败：{主机}:{端口}（{错误}）——请检查网络连接与域名可用性")
    })?;
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
///
/// §13.f.3 重试信息采集：返回 (响应, 重试次数, 累计退避毫秒) 供调用方记入观测附加。
fn 发送并重试<F>(发请求: F, 模型: &str) -> Result<(ureq::Response, u32, u64), String>
where
    F: Fn() -> Result<ureq::Response, ureq::Error>,
{
    let mut 重试次数 = 0u32;
    let mut 累计退避ms: u64 = 0;
    loop {
        match 发请求() {
            Ok(响应) => return Ok((响应, 重试次数, 累计退避ms)),
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
                    return Err(format!(
                        "模型请求失败(重试{最大重试次数}次后仍瞬时故障): {详情}"
                    ));
                }
                let 间隔 = 退避间隔(重试次数);
                重试次数 += 1;
                累计退避ms += 间隔 * 1000;
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
/// clippy result_large_err：闭包返回 ureq::Result，ureq::Error 272 字节（第三方库类型，
/// 无法通过本仓改小；Box 需动 发送并重试 泛型签名，收益低于成本，故允许）。
#[allow(clippy::result_large_err)]
pub fn 调用模型(
    配置: &模型配置,
    消息们: &[对话消息],
    输出上限: u32,
) -> Result<(String, 用量), String> {
    预检地址(&配置.地址)?;
    let 请求体 = 构造请求体(配置, 消息们, 输出上限);
    let (角色, 关联) = 当前观测();
    记请求(角色, "模型连接-府::调用模型", &请求体, 关联.clone());

    let 调用起 = std::time::Instant::now();
    let (响应, 重试次数, 累计退避ms) = 发送并重试(
        || {
            ureq::post(&配置.地址)
                .timeout(请求超时)
                .set("Authorization", &format!("Bearer {}", 配置.密钥))
                .set("Content-Type", "application/json")
                .send_string(&请求体)
        },
        &配置.模型,
    )?;
    let 首token时延 = 调用起.elapsed().as_millis() as u64;

    let 文本 = 响应.into_string().map_err(|错误| {
        error!("读取响应失败：{错误}");
        format!("读取响应失败: {错误}")
    })?;
    let 总耗时ms = 调用起.elapsed().as_millis() as u64;

    let 内容 = 解析回复(&文本)?;
    let 思考 = 提取思考链(&内容).unwrap_or_default();
    let 用量 = 解析用量(&文本);
    let 解码吞吐 = 解码吞吐量(用量.输出, 总耗时ms.saturating_sub(首token时延));
    记回复(
        角色,
        "模型连接-府::调用模型",
        &思考,
        &内容,
        关联,
        Some(用量附加扩展(
            配置,
            &用量,
            首token时延,
            总耗时ms,
            解码吞吐,
            重试次数,
            累计退避ms,
        )),
    );
    // token 数降为 debug 级别：保留成本观察能力，但不入 info 级别日志（安全报告 L7）。
    debug!(
        模型 = %配置.模型, 内容长度 = 内容.len(),
        提示词 = 用量.提示词, 输出 = 用量.输出, 缓存命中 = 用量.缓存命中,
        缓存写 = 用量.缓存写, 推理 = 用量.推理,
        首token时延, 总耗时ms, 解码吞吐, 重试次数,
        "模型返回正常"
    );
    Ok((内容, 用量))
}

/// 调用模型并允许工具调用（function calling）：发送消息与工具定义，落回文本或工具调用（附用量）。
/// clippy result_large_err：同上（ureq::Error 第三方库类型，允许）。
#[allow(clippy::result_large_err)]
pub fn 调用模型带工具(
    配置: &模型配置,
    消息们: &[对话消息],
    工具们: &[工具定义],
    输出上限: u32,
) -> Result<(模型回复, 用量), String> {
    预检地址(&配置.地址)?;
    let 请求体 = 构造工具请求体(配置, 消息们, 工具们, 输出上限);
    let (角色, 关联) = 当前观测();
    记请求(角色, "模型连接-府::调用模型带工具", &请求体, 关联.clone());

    let 调用起 = std::time::Instant::now();
    let (响应, 重试次数, 累计退避ms) = 发送并重试(
        || {
            ureq::post(&配置.地址)
                .timeout(请求超时)
                .set("Authorization", &format!("Bearer {}", 配置.密钥))
                .set("Content-Type", "application/json")
                .send_string(&请求体)
        },
        &配置.模型,
    )?;
    let 首token时延 = 调用起.elapsed().as_millis() as u64;

    let 文本 = 响应.into_string().map_err(|错误| {
        error!("读取响应失败：{错误}");
        format!("读取响应失败: {错误}")
    })?;
    let 总耗时ms = 调用起.elapsed().as_millis() as u64;

    let 回复 = 解析工具回复(&文本)?;
    let 用量 = 解析用量(&文本);
    // §13.f.7a 思考链提取：从工具调用模式的回复内容（含 think）中剥离思考文本入库。
    let 概要 = 回复概要(&回复);
    let 思考 = if let 模型回复::工具调用(ref 内容, _) = 回复 {
        提取思考链(内容).unwrap_or_default()
    } else if let 模型回复::文本(ref 内容) = 回复 {
        提取思考链(内容).unwrap_or_default()
    } else {
        String::new()
    };
    let 解码吞吐 = 解码吞吐量(用量.输出, 总耗时ms.saturating_sub(首token时延));
    记回复(
        角色,
        "模型连接-府::调用模型带工具",
        &思考,
        &概要,
        关联,
        Some(用量附加扩展(
            配置,
            &用量,
            首token时延,
            总耗时ms,
            解码吞吐,
            重试次数,
            累计退避ms,
        )),
    );
    // token 数降为 debug 级别：保留成本观察能力，但不入 info 级别日志（安全报告 L7）。
    debug!(
        模型 = %配置.模型, 内容长度 = 文本.len(),
        提示词 = 用量.提示词, 输出 = 用量.输出, 缓存命中 = 用量.缓存命中,
        缓存写 = 用量.缓存写, 推理 = 用量.推理,
        首token时延, 总耗时ms, 解码吞吐, 重试次数,
        "模型返回（工具模式）"
    );
    Ok((回复, 用量))
}

/// 解码吞吐量（tokens/秒）：输出 token 数 / 解码耗时（秒）。
/// §13.f.3 assistantMetrics 解码 tok/s。解码耗时 = 总耗时 - 首token时延。
/// 解码耗时为零时返回 0（避免除零；首token即结束的极短调用无解码阶段）。
fn 解码吞吐量(输出token: u64, 解码耗时ms: u64) -> f64 {
    if 解码耗时ms == 0 || 输出token == 0 {
        return 0.0;
    }
    let 秒 = 解码耗时ms as f64 / 1000.0;
    输出token as f64 / 秒
}

/// 观测附加：用量与模型名（白箱还原时核对「谁发了多少 token」）。
///
/// 保留原 `用量附加` 供向后兼容引用；新调用走 `用量附加扩展` 含 §13.f 全字段。
#[allow(dead_code)]
fn 用量附加(配置: &模型配置, 用量: &用量) -> serde_json::Value {
    serde_json::json!({
        "模型": 配置.模型,
        "提示词": 用量.提示词,
        "输出": 用量.输出,
        "缓存命中": 用量.缓存命中,
        "总计": 用量.总计,
    })
}

/// 观测附加扩展：§13.f 白箱全字段。
///
/// 在 `用量附加` 基础上补采集：
/// - `缓存写`/`推理`：§13.f.7 token 五分量
/// - `TTFT`：首 token 时延（毫秒），§13.f.3 assistantMetrics
/// - `耗时ms`：总耗时（毫秒），§13.f.2 行格式耗时列
/// - `解码吞吐`：解码 tok/s，§13.f.3 assistantMetrics
/// - `重试`/`最大重试`/`重试延迟ms`：§13.f.3 retry/maxRetries
/// - `提供者`：§13.f.3 provider/model（HTTP 提供者标识）
fn 用量附加扩展(
    配置: &模型配置,
    用量: &用量,
    首token时延: u64,
    总耗时ms: u64,
    解码吞吐: f64,
    重试次数: u32,
    累计退避ms: u64,
) -> serde_json::Value {
    serde_json::json!({
        "模型": 配置.模型,
        "提供者": "http",
        "提示词": 用量.提示词,
        "输出": 用量.输出,
        "缓存命中": 用量.缓存命中,
        "缓存写": 用量.缓存写,
        "推理": 用量.推理,
        "总计": 用量.总计,
        "TTFT": 首token时延,
        "耗时ms": 总耗时ms,
        "解码吞吐": 解码吞吐,
        "重试": 重试次数,
        "最大重试": 最大重试次数,
        "重试延迟ms": 累计退避ms,
    })
}

/// 观测用：把 模型回复 转为可落盘的概要文本（工具调用序列也可见）。
fn 回复概要(回复: &模型回复) -> String {
    match 回复 {
        模型回复::文本(内容) => 内容.clone(),
        模型回复::工具调用(内容, 调用们) => {
            let 调用们: Vec<String> = 调用们
                .iter()
                .map(|调用| format!("【{}】\n{}", 调用.名字, 调用.参数))
                .collect();
            format!("{内容}\n{工具调用们}", 工具调用们 = 调用们.join("\n"))
        }
        模型回复::参数缺失(名字们) => format!("【参数缺失】{}", 名字们.join(", ")),
    }
}

/// 从 OpenAI 兼容响应解析 usage（提示词 / 输出 / 缓存命中 / 缓存写 / 推理）。
///
/// - MiniMax 返回 `usage.cache_read_input_tokens` / `usage.cache_creation_input_tokens`
/// - OpenAI 兼容返回 `usage.prompt_tokens_details.cached_tokens`
/// - Anthropic 风格返回 `usage.cache_read_input_tokens` / `usage.cache_creation_input_tokens`
/// - reasoning_tokens：`usage.completion_tokens_details.reasoning_tokens`（思考链消耗）
///
/// §13.f.7 token 五分量：提示词 / 输出 / 缓存命中 / 缓存写 / 推理 / 总计。
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
    // §13.f.7 缓存写：cache_creation_input_tokens（Anthropic/MiniMax 风格）。
    let 缓存写 = 用量值["cache_creation_input_tokens"]
        .as_u64()
        .or_else(|| 用量值["cache_write_input_tokens"].as_u64())
        .unwrap_or(0);
    // §13.f.7 推理：completion_tokens_details.reasoning_tokens（思考链消耗，与输出分开计量）。
    let 推理 = 用量值["completion_tokens_details"]["reasoning_tokens"]
        .as_u64()
        .or_else(|| 用量值["reasoning_tokens"].as_u64())
        .unwrap_or(0);
    let 总计 = 用量值["total_tokens"].as_u64().unwrap_or(提示词 + 输出);
    用量 {
        提示词,
        输出,
        缓存命中,
        总计,
        缓存写,
        推理,
    }
}

/// 构造 OpenAI 兼容请求体（独立出便于测试）。
/// 显式声明 thinking:disabled 与 temperature：纯文本结构化调用（想法解析/设计/评审/记忆归纳等）
/// 走 thinking:disabled——JSON 直接输出、省 token、防 think 吃光配额致真内容截断
/// （设计稿 §12 P2-9：MiniMax-M3 自适应 think 曾吃光 4096 配额、真 JSON 被截断回退模板）。
/// temperature 对齐 构造工具请求体 的显式声明方式，固定可复现、可收敛。
pub fn 构造请求体(
    配置: &模型配置, 消息们: &[对话消息], 输出上限: u32
) -> String {
    serde_json::json!({
        "model": 配置.模型,
        "messages": 消息们.iter().map(|消息| serde_json::json!({
            "role": 消息.角色,
            "content": 消息.内容,
        })).collect::<Vec<_>>(),
        "max_tokens": 输出上限,
        "temperature": 0.2,
        "thinking": {"type": "disabled"},
    })
    .to_string()
}

/// 构造带工具定义的 OpenAI 兼容请求体（独立出便于测试）。
/// 显式声明 temperature 与 thinking：服务端默认值漂移曾致 MiniMax-M3 工具参数退化
/// （arguments={} 缺必填字段，见设计稿 4.1-4），显式固定可复现、可收敛。
pub fn 构造工具请求体(
    配置: &模型配置,
    消息们: &[对话消息],
    工具们: &[工具定义],
    输出上限: u32,
) -> String {
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

/// 从模型回复内容中提取 `<think>...</think>` 思考块。
///
/// §13.f.7a 思考链入库契约前置：调用模型 / 调用模型带工具 在 解析回复 后调本函数，
/// 把思考文本独立出来传给 jiance_fu::记回复（替代当前空字符串）。
///
/// 算法：字符串字面量感知的平衡扫描（与 提取对象 同思路，跳过字符串内部），找每段
/// `<think>...</think>` 内的文本，多段拼接返回；无任何思考块返回 None。
pub fn 提取思考链(内容: &str) -> Option<String> {
    let 修剪 = 内容.trim();
    if 修剪.is_empty() {
        return None;
    }
    let mut 结果们: Vec<&str> = Vec::new();
    let mut 游标 = 0;
    while 游标 < 修剪.len() {
        // 找下一个 <think> 起始
        let Some(开相对) = 修剪[游标..].find("<think>") else {
            break;
        };
        let 开绝对 = 游标 + 开相对;
        let 内容起点 = 开绝对 + "<think>".len();
        // 找对应的 </think> 结束（字符串字面量感知的平衡扫描）
        let 闭绝对 = match 配平思考块(&修剪[内容起点..]) {
            Some(偏移) => 内容起点 + 偏移,
            None => break,
        };
        // 提取内容并 trim
        let 块 = 修剪[内容起点..闭绝对].trim();
        if !块.is_empty() {
            结果们.push(块);
        }
        游标 = 闭绝对 + "</think>".len();
    }
    if 结果们.is_empty() {
        None
    } else {
        Some(结果们.join(
            "

",
        ))
    }
}

/// 从 `<think>` 起点开始，扫描到对应 `</think>` 结束位置（跳过字符串字面量）。
/// 找不到匹配闭标签返回 None。
fn 配平思考块(从开标签后: &str) -> Option<usize> {
    let 字节们 = 从开标签后.as_bytes();
    let mut 深度: usize = 0;
    let mut 在字符串 = false;
    let mut 转义 = false;
    let mut i = 0;
    while i < 字节们.len() {
        let b = 字节们[i];
        if 转义 {
            转义 = false;
            i += 1;
            continue;
        }
        if 在字符串 {
            match b {
                b'\\' => 转义 = true,
                b'"' => 在字符串 = false,
                _ => {}
            }
            i += 1;
            continue;
        }
        // 不在字符串内：检查标签
        // <think> 嵌套（罕见但允许）：增加深度
        if i + 7 <= 字节们.len() && &字节们[i..i + 7] == b"<think>" {
            深度 += 1;
            i += 7;
            continue;
        }
        // </think> 减少深度
        if i + 8 <= 字节们.len() && &字节们[i..i + 8] == b"</think>" {
            if 深度 == 0 {
                // 外层闭合
                return Some(i);
            }
            深度 -= 1;
            i += 8;
            continue;
        }
        // 进入字符串（双引号）
        if b == b'"' {
            在字符串 = true;
        }
        i += 1;
    }
    None
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
    let 去think = if let (Some(开), Some(闭)) = (文本.find("<think>"), 文本.find("</think>"))
    {
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

/// 消息序列化：工具链路用（额外携带 tool_calls / tool_call_id 回传）。
fn 消息们_带工具(消息们: &[对话消息]) -> Vec<serde_json::Value> {
    消息们
        .iter()
        .map(|消息| {
            let mut 对象 = serde_json::json!({
                "role": 消息.角色,
                "content": 消息.内容,
            });
            if let Some(调用们) = &消息.工具调用们 {
                对象["tool_calls"] = serde_json::json!(调用们
                    .iter()
                    .map(|调用| {
                        serde_json::json!({
                            "id": 调用.标识,
                            "type": "function",
                            "function": { "name": 调用.名字, "arguments": 调用.参数 }
                        })
                    })
                    .collect::<Vec<_>>());
            }
            if let Some(标识) = &消息.工具调用标识 {
                对象["tool_call_id"] = serde_json::json!(标识);
            }
            对象
        })
        .collect()
}

/// 模型提供者：换模型不改主循环的抽象契约（对齐 DeepSeek「模型无关」）。
/// 本质：任何 LLM 提供者的通用协议——发消息（可带工具）→ 回复 + 用量。
/// HTTP 走 OpenAI 兼容；模拟提供者供测试/离线演练，不耗真实 API。
pub trait 模型提供者 {
    fn 调用(
        &self,
        配置: &模型配置,
        消息们: &[对话消息],
        输出上限: u32,
    ) -> Result<(String, 用量), String>;
    fn 调用带工具(
        &self,
        配置: &模型配置,
        消息们: &[对话消息],
        工具们: &[工具定义],
        输出上限: u32,
    ) -> Result<(模型回复, 用量), String>;
}

/// HTTP 模型提供者：走 OpenAI 兼容 HTTP（当前 MiniMax-M3），退避重试 + 解析。
pub struct HTTP模型提供者;

impl 模型提供者 for HTTP模型提供者 {
    fn 调用(
        &self,
        配置: &模型配置,
        消息们: &[对话消息],
        输出上限: u32,
    ) -> Result<(String, 用量), String> {
        调用模型(配置, 消息们, 输出上限)
    }
    fn 调用带工具(
        &self,
        配置: &模型配置,
        消息们: &[对话消息],
        工具们: &[工具定义],
        输出上限: u32,
    ) -> Result<(模型回复, 用量), String> {
        调用模型带工具(配置, 消息们, 工具们, 输出上限)
    }
}

/// 模拟模型提供者：返回固定文本，供测试/离线演练，不耗真实 API。
pub struct 模拟模型提供者 {
    pub 回复文本: String,
}

impl 模型提供者 for 模拟模型提供者 {
    fn 调用(
        &self,
        _配置: &模型配置,
        _消息们: &[对话消息],
        _输出上限: u32,
    ) -> Result<(String, 用量), String> {
        Ok((self.回复文本.clone(), 用量::default()))
    }
    fn 调用带工具(
        &self,
        _配置: &模型配置,
        _消息们: &[对话消息],
        _工具们: &[工具定义],
        _输出上限: u32,
    ) -> Result<(模型回复, 用量), String> {
        Ok((模型回复::文本(self.回复文本.clone()), 用量::default()))
    }
}

/// 提供者句柄：Arc<dyn 模型提供者>（可 Clone，热替换语义）。
pub type 提供者句柄 = std::sync::Arc<dyn 模型提供者 + Send + Sync>;

/// 模型提供者注册表：按名注册/替换/注销/取用（阶段 3 · §14.10.3c 热替换，换模型不改主循环）。
/// 对齐 dsh ctx.llm 服务：模型适配器注册为可替换提供方。
#[derive(Default)]
pub struct 模型提供者注册表 {
    提供者们: std::collections::HashMap<String, 提供者句柄>,
}

impl 模型提供者注册表 {
    /// 新建空注册表。
    pub fn 新() -> Self {
        Self::default()
    }

    /// 注册提供者：同名已存在则报错（防静默覆盖；替换用 替换）。
    pub fn 注册(&mut self, 名: &str, 提供者: 提供者句柄) -> Result<(), String> {
        if self.提供者们.contains_key(名) {
            return Err(format!("模型提供者「{名}」已注册，用 替换 覆盖"));
        }
        self.提供者们.insert(名.to_string(), 提供者);
        Ok(())
    }

    /// 替换提供者：覆盖同名，返回旧句柄（热替换主入口）。
    pub fn 替换(&mut self, 名: &str, 提供者: 提供者句柄) -> Option<提供者句柄> {
        self.提供者们.insert(名.to_string(), 提供者)
    }

    /// 注销提供者：返回被注销的句柄，未注册返回 None。
    pub fn 注销(&mut self, 名: &str) -> Option<提供者句柄> {
        self.提供者们.remove(名)
    }

    /// 取提供者。
    pub fn 取(&self, 名: &str) -> Option<提供者句柄> {
        self.提供者们.get(名).cloned()
    }

    /// 已注册的全部提供者名。
    pub fn 全部名(&self) -> Vec<String> {
        self.提供者们.keys().cloned().collect()
    }
}

/// 全局提供者注册表：进程级单例（static OnceLock），供 派遣/验收 按名取模型提供者。
/// 首次访问注册默认 "http" 提供者（HTTP模型提供者）。
pub fn 全局提供者注册表() -> &'static std::sync::Mutex<模型提供者注册表> {
    static 全局: std::sync::OnceLock<std::sync::Mutex<模型提供者注册表>> =
        std::sync::OnceLock::new();
    全局.get_or_init(|| {
        let mut 表 = 模型提供者注册表::新();
        let _ = 表.注册("http", std::sync::Arc::new(HTTP模型提供者));
        std::sync::Mutex::new(表)
    })
}

#[cfg(test)]
mod 测试 {
    use super::{
        提取对象, 提取思考链, 是瞬时故障, 最大重试次数, 解析用量, 退避间隔, 预检地址
    };

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
        use super::{模型回复, 模型提供者, 模拟模型提供者};
        let 模拟 = 模拟模型提供者 {
            回复文本: "固定回复".to_string(),
        };
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

    /// Provider 注册表：注册/重复注册报错/替换返回旧/注销取回（阶段 3 热替换）。
    #[test]
    fn 提供者注册表_注册替换注销() {
        use super::{提供者句柄, 模型提供者注册表, 模拟模型提供者};
        let mut 表 = 模型提供者注册表::新();
        let 甲: 提供者句柄 = std::sync::Arc::new(模拟模型提供者 {
            回复文本: "甲".to_string(),
        });
        let 乙: 提供者句柄 = std::sync::Arc::new(模拟模型提供者 {
            回复文本: "乙".to_string(),
        });

        assert!(表.注册("模拟", 甲).is_ok(), "首次注册应成功");
        assert!(表.注册("模拟", 乙.clone()).is_err(), "同名重复注册应报错");

        // 替换返回旧句柄。
        let 旧 = 表.替换("模拟", 乙.clone());
        assert!(旧.is_some(), "替换应返回旧句柄");

        // 取回新句柄。
        let 取 = 表.取("模拟").expect("应取到");
        let 配置 = peizhi_fu::模型配置 {
            密钥: String::new(),
            地址: String::new(),
            模型: String::new(),
        };
        let (文本, _) = 取.调用(&配置, &[], 100).unwrap();
        assert_eq!(文本, "乙", "替换后应取到新提供者");

        // 注销取回。
        assert!(表.注销("模拟").is_some());
        assert!(表.取("模拟").is_none());
        assert!(表.注销("不存在").is_none());
    }

    // §13.f.7a 思考链提取：从模型回复剥离 <think>...</think> 块，多段拼接。
    #[test]
    fn 提取思考链_单段() {
        let 内容 = "<think>推理过程</think>\n最终回复";
        assert_eq!(提取思考链(内容), Some("推理过程".to_string()));
    }

    #[test]
    fn 提取思考链_多段拼接() {
        let 内容 = "<think>第一段</think>中间<think>第二段</think>回复";
        let 思考 = 提取思考链(内容);
        let 思考文本 = 思考.expect("应提取到思考");
        // 两段思考都应被收集
        assert!(思考文本.contains("第一段"));
        assert!(思考文本.contains("第二段"));
        // "中间"是思考外的回复，不应进思考
        assert!(
            !思考文本.contains("中间"),
            "中间是思考外的回复，不应混入思考"
        );
    }

    #[test]
    fn 提取思考链_无思考块返回_none() {
        assert_eq!(提取思考链("纯回复文本无思考"), None);
    }

    #[test]
    fn 提取思考链_空字符串返回_none() {
        assert_eq!(提取思考链(""), None);
        assert_eq!(提取思考链("   "), None);
    }

    #[test]
    fn 提取思考链_多行思考保留空白() {
        let 内容 = "<think>\n第一行\n第二行\n</think>\n回复";
        let 思考 = 提取思考链(内容).expect("应提取");
        assert!(思考.contains("第一行"));
        assert!(思考.contains("第二行"));
    }

    #[test]
    fn 提取思考链_思考在末尾() {
        let 内容 = "回复在前<think>后面是思考</think>";
        assert_eq!(提取思考链(内容), Some("后面是思考".to_string()));
    }

    #[test]
    fn 提取思考链_大文本不超时() {
        let 长思考 = "x".repeat(100_000);
        let 内容 = format!("<think>{长思考}</think>\n回复");
        let 思考 = 提取思考链(&内容);
        assert_eq!(思考.as_ref().map(|s| s.len()), Some(100_000));
    }

    #[test]
    fn 提取思考链_不平衡时优雅() {
        // <think> 但没有 </think>：不应 panic
        let 思考 = 提取思考链("<think>只有开头没有结尾");
        let _ = 思考;
    }
}
