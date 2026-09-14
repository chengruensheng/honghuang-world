use std::time::Duration;
use hm_content_contract::{内容生成器, 工具对话器, 流式对话器, 对话消息, 消息角色, 工具调用, 模型响应};

use super::super::解析密钥;
use super::看门狗::{限时执行, 流式限时执行};
use hm_contract::Component;
use hm_error::{Error, Result};
use serde_json::json;

/// 单次请求超时（秒）
const 请求超时秒: u64 = 30;
/// 流式请求超时（秒）：流式生成等待可能远超同步单次
const 流式超时秒: u64 = 120;
/// 失败后额外重试次数（首次 + 重试 = 总尝试次数）
const 请求重试次数: u32 = 2;
/// 重试间隔（秒）
const 请求重试间隔秒: u64 = 2;
/// 单次生成最大输出 tokens：必须显式给足，推理模型思考链会吃光网关默认上限（实测 8192）
/// 致 finish_reason=length 且正文为空（2026-09-14 实证）。
const 输出令牌上限: u64 = 32768;

/// 主模型环境变量名
const 主密钥环境变量: &str = "LLM_API_KEY";
const 主地址环境变量: &str = "LLM_BASE_URL";
const 主模型环境变量: &str = "LLM_MODEL";
/// 备选模型环境变量名（主模型失败时自动降级）
const 备选密钥环境变量: &str = "LLM_FALLBACK_API_KEY";
const 备选地址环境变量: &str = "LLM_FALLBACK_BASE_URL";
const 备选模型环境变量: &str = "LLM_FALLBACK_MODEL";

/// 模型提供商：一组 OpenAI 兼容端点的凭据（密钥 / 地址 / 模型名）
struct 模型提供商 {
    api_key: String,
    base_url: String,
    model: String,
}

impl 模型提供商 {
    /// 单次 HTTP 请求：发送 body 并解析 JSON 响应体（不重试）。
    /// 整体交看门狗限时——Windows 下 ureq 读体阶段超时失效，挂死须由看门狗斩断（见 看门狗.rs 头注）。
    fn 请求(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
        let 地址 = self.base_url.clone();
        let 密钥 = self.api_key.clone();
        let 体 = body.to_string();
        let 时限 = Duration::from_secs(请求超时秒);
        限时执行(时限, move || {
            let resp = ureq::post(&地址)
                .set("Authorization", &format!("Bearer {密钥}"))
                .set("Content-Type", "application/json")
                .timeout(时限)
                .send_string(&体)
                .map_err(|e| Error::模型(format!("请求模型失败: {e}")))?;
            let text = resp
                .into_string()
                .map_err(|e| Error::模型(format!("读取模型响应失败: {e}")))?;
            serde_json::from_str(&text)
                .map_err(|e| Error::模型(format!("解析模型响应失败: {e}")))
        })
    }

    /// 流式对话：stream=true 请求 + 按配置重试 + SSE 增量回调。
    /// 中流失败（已推部分内容）不重试，避免向客户端重复推送；仅在未推任何块时重试。
    fn 流式对话(
        &self,
        消息: Vec<对话消息>,
        工具: &[serde_json::Value],
        on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), Error>,
    ) -> Result<模型响应> {
        let 消息json: Vec<serde_json::Value> = 消息.iter().map(消息转json).collect();
        let mut body = json!({
            "model": &self.model,
            "messages": &消息json,
            "tools": 工具,
            "stream": true,
            "max_tokens": 输出令牌上限,
        });
        // tool_choice=auto：道祖接待三选一场景，引导模型在需要时稳定触发工具调用
        if !工具.is_empty() {
            body["tool_choice"] = json!("auto");
        }
        let mut 已发块 = false;
        let mut 最后错误: Option<Error> = None;
        for 尝试 in 0..=请求重试次数 {
            if 尝试 > 0 && !已发块 {
                std::thread::sleep(Duration::from_secs(请求重试间隔秒));
            }
            match self.单次流式(&body, &mut 已发块, on_chunk) {
                Ok(响应) => return Ok(响应),
                Err(错误) => {
                    最后错误 = Some(错误);
                    if 已发块 {
                        break; // 已推送部分内容，重试会重复 → 中止向上报错
                    }
                }
            }
        }
        Err(最后错误.unwrap_or_else(|| Error::模型("请求模型失败".into())))
    }

    /// 单次流式请求：发送 + 逐行解析 SSE（content delta → on_chunk）。
    /// 块间隔看门狗：首块等待与中流停顿超过时限即判败（读流失效须看门狗斩断，见 看门狗.rs）。
    fn 单次流式(
        &self,
        body: &serde_json::Value,
        已发块: &mut bool,
        on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), Error>,
    ) -> Result<模型响应> {
        let 地址 = self.base_url.clone();
        let 密钥 = self.api_key.clone();
        let 体 = body.to_string();
        let 时限 = Duration::from_secs(流式超时秒);
        流式限时执行(
            时限,
            move |转发| {
                let resp = ureq::post(&地址)
                    .set("Authorization", &format!("Bearer {密钥}"))
                    .set("Content-Type", "application/json")
                    .timeout(时限)
                    .send_string(&体)
                    .map_err(|e| Error::模型(format!("请求模型失败: {e}")))?;
                let 读 = std::io::BufReader::new(resp.into_reader());
                流式解析_sse(读, 转发)
            },
            已发块,
            on_chunk,
        )
    }
}

/// 解析 OpenAI 兼容 chat/completions 流式响应（SSE：`data: {json}` 行）。
/// content delta → on_chunk 增量回调；tool_calls 增量按 index 拼接。
/// 响应完全没有 data: 行（供应商不支持流式）→ 按整段 JSON 兜底解析，内容一次性回调。
pub fn 流式解析_sse<R: std::io::BufRead>(
    mut 读: R,
    on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), Error>,
) -> Result<模型响应> {
    let mut 行 = String::new();
    let mut 内容 = String::new();
    let mut 思考 = String::new();
    let mut 调用们: Vec<工具调用> = Vec::new();
    let mut 原始 = String::new();
    let mut 见到data = false;
    loop {
        行.clear();
        let n = 读
            .read_line(&mut 行)
            .map_err(|e| Error::模型(format!("读取流式响应失败: {e}")))?;
        if n == 0 {
            break;
        }
        原始.push_str(&行);
        let 修剪 = 行.trim();
        if 修剪.is_empty() {
            continue;
        }
        if let Some(数据) = 修剪.strip_prefix("data:") {
            见到data = true;
            let 数据 = 数据.trim();
            if 数据 == "[DONE]" {
                break;
            }
            let 值: serde_json::Value = serde_json::from_str(数据)
                .map_err(|e| Error::模型(format!("解析流式块失败: {e}")))?;
            // 思考内容（reasoning_content / reasoning）
            if let Some(思考块) = 值["choices"][0]["delta"]["reasoning_content"].as_str() {
                if !思考块.is_empty() {
                    思考.push_str(思考块);
                }
            }
            if let Some(思考块) = 值["choices"][0]["delta"]["reasoning"].as_str() {
                if !思考块.is_empty() {
                    思考.push_str(思考块);
                }
            }
            if let Some(delta) = 值["choices"][0]["delta"]["content"].as_str() {
                if !delta.is_empty() {
                    内容.push_str(delta);
                    on_chunk(delta.to_string())?;
                }
            }
            if let Some(数组) = 值["choices"][0]["delta"]["tool_calls"].as_array() {
                for 片段 in 数组 {
                    let 索引 = 片段["index"].as_u64().unwrap_or(0) as usize;
                    while 调用们.len() <= 索引 {
                        调用们.push(工具调用 { id: String::new(), 名称: String::new(), 参数: String::new() });
                    }
                    if let Some(id) = 片段["id"].as_str() {
                        if 调用们[索引].id.is_empty() {
                            调用们[索引].id = id.to_string();
                        }
                    }
                    if let Some(名) = 片段["function"]["name"].as_str() {
                        调用们[索引].名称.push_str(名);
                    }
                    match &片段["function"]["arguments"] {
                        // 流式规范：arguments 按字符串分片累积
                        serde_json::Value::String(参) => 调用们[索引].参数.push_str(参),
                        // 非规范：网关把完整对象/数组塞进一个分片，对象无法分片拼接 → 整体覆盖
                        值 @ (serde_json::Value::Object(_) | serde_json::Value::Array(_)) => {
                            调用们[索引].参数 = 值.to_string();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    // 非流式兜底：供应商忽略了 stream 标志，整段返回标准 JSON
    if !见到data {
        let 值: serde_json::Value = serde_json::from_str(&原始)
            .map_err(|e| Error::模型(format!("响应非流式且非合法 JSON: {e}")))?;
        let message = &值["choices"][0]["message"];
        let 完整 = message["content"].as_str().unwrap_or_default();
        if !完整.is_empty() {
            on_chunk(完整.to_string())?;
            内容.push_str(完整);
        }
        调用们 = 解析工具调用(message);
    }
    Ok(模型响应 {
        内容: if 内容.is_empty() { None } else { Some(内容) },
        工具调用: 调用们,
        思考: if 思考.is_empty() { None } else { Some(思考) },
    })
}

/// 对话生成器：通过 OpenAI 兼容的 chat/completions 接口调用外部大模型（默认 MiniMax）。
/// 凭据与端点经环境变量注入，密钥不入库；主模型失败（限流/超时/解析）自动降级备选。
pub struct 对话生成器 {
    主: 模型提供商,
    备选: Option<模型提供商>,
}

impl 对话生成器 {
    /// 显式构造（密钥不入库，由调用方注入；仅主模型，无降级）
    pub fn 新(api_key: String, base_url: String, model: String) -> Self {
        对话生成器 {
            主: 模型提供商 { api_key, base_url, model },
            备选: None,
        }
    }

    /// 显式构造主 + 备选提供商（密钥不入库，由调用方注入；主失败自动降级备选）
    pub fn 新带备选(
        主密钥: String, 主地址: String, 主模型: String,
        备密钥: String, 备地址: String, 备模型: String,
    ) -> Self {
        对话生成器 {
            主: 模型提供商 { api_key: 主密钥, base_url: 主地址, model: 主模型 },
            备选: Some(模型提供商 { api_key: 备密钥, base_url: 备地址, model: 备模型 }),
        }
    }

    /// 从 LLM 池配置构造：取前两个 启用且密钥有效 的供应商为 主/备（兼容池配置与旧主备语义）。
    /// 无有效供应商时返回空主（生成时报错），由调用方决定是否回退。
    pub fn 从配置(配置: &hm_config::LlmConfig) -> Self {
        let mut 启用们 = 配置
            .providers
            .iter()
            .filter(|p| p.enabled && !解析密钥(&p.api_key).is_empty());
        let 主项 = 启用们.next();
        let 备项 = 启用们.next();
        let 主 = match 主项 {
            Some(p) => 模型提供商 {
                api_key: 解析密钥(&p.api_key),
                base_url: p.base_url.clone(),
                model: p.model.clone(),
            },
            None => 模型提供商 {
                api_key: String::new(),
                base_url: String::new(),
                model: String::new(),
            },
        };
        let 备选 = 备项.map(|p| 模型提供商 {
            api_key: 解析密钥(&p.api_key),
            base_url: p.base_url.clone(),
            model: p.model.clone(),
        });
        对话生成器 { 主, 备选 }
    }

    /// 从环境变量构建：主 LLM_API_KEY / LLM_BASE_URL / LLM_MODEL，
    /// 备选 LLM_FALLBACK_API_KEY / LLM_FALLBACK_BASE_URL / LLM_FALLBACK_MODEL（可选）。
    /// 主 key 缺失 fail-loud（返回错误），绝不静默降级。
    pub fn 从环境() -> Result<Self> {
        // 尽力加载环境密钥文件（不存在时忽略）
        let _ = dotenvy::dotenv();
        let api_key = std::env::var(主密钥环境变量)
            .map_err(|_| Error::Config("缺少 LLM_API_KEY，请在环境密钥文件中配置".into()))?;
        let base_url = std::env::var(主地址环境变量)
            .map_err(|_| Error::Config("缺少 LLM_BASE_URL，请配置环境变量".into()))?;
        let model = std::env::var(主模型环境变量)
            .map_err(|_| Error::Config("缺少 LLM_MODEL，请配置环境变量".into()))?;
        let mut 生成器 = 对话生成器::新(api_key, base_url, model);
        // 备选提供商（三项齐全才启用降级，缺任一则仅用主模型）
        if let (Ok(备密钥), Ok(备地址), Ok(备模型)) = (
            std::env::var(备选密钥环境变量),
            std::env::var(备选地址环境变量),
            std::env::var(备选模型环境变量),
        ) {
            生成器.备选 = Some(模型提供商 { api_key: 备密钥, base_url: 备地址, model: 备模型 });
        }
        Ok(生成器)
    }

    /// 依次尝试主、备选提供商：构造请求体 → 请求（含重试）→ 解析，任一成功即返回。
    /// 主失败（请求或解析）记录警告并降级备选；备选也失败则返回最后的错误。
    fn 逐个生成<T, 造, 析>(&self, 造体: &造, 解析: &析) -> Result<T>
    where
        造: Fn(&模型提供商) -> serde_json::Value,
        析: Fn(&serde_json::Value) -> Result<T>,
    {
        match self.请求解析(&self.主, &造体(&self.主), 解析) {
            Ok(值) => Ok(值),
            Err(主错误) => {
                tracing::warn!("主模型请求失败，尝试降级备选: {主错误}");
                match &self.备选 {
                    Some(备) => match self.请求解析(备, &造体(备), 解析) {
                        Ok(值) => Ok(值),
                        Err(备错误) => {
                            tracing::warn!("备选模型请求失败: {备错误}");
                            Err(备错误)
                        }
                    },
                    None => Err(主错误),
                }
            }
        }
    }

    /// 单提供商：请求 + 按配置重试 + 解析
    fn 请求解析<T, 析>(&self, 提供商: &模型提供商, body: &serde_json::Value, 解析: &析) -> Result<T>
    where
        析: Fn(&serde_json::Value) -> Result<T>,
    {
        let mut 最后一次错误: Option<Error> = None;
        for 尝试 in 0..=请求重试次数 {
            if 尝试 > 0 {
                std::thread::sleep(Duration::from_secs(请求重试间隔秒));
            }
            match 提供商.请求(body) {
                Ok(值) => return 解析(&值),
                Err(错误) => 最后一次错误 = Some(错误),
            }
        }
        Err(最后一次错误.unwrap_or_else(|| Error::模型("请求模型失败".into())))
    }
}

impl Component for 对话生成器 {
    fn name(&self) -> &'static str { "对话生成器" }
}

impl 内容生成器 for 对话生成器 {
    fn 生成(&self, 提示词: String) -> Result<String> {
        let 造体 = |提供商: &模型提供商| json!({
            "model": &提供商.model,
            "messages": [{ "role": "user", "content": 提示词.as_str() }],
            "max_tokens": 输出令牌上限,
        });
        let 解析 = |值: &serde_json::Value| {
            值["choices"][0]["message"]["content"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| Error::模型("模型响应缺少 choices[0].message.content".into()))
        };
        self.逐个生成(&造体, &解析)
    }
}

impl 工具对话器 for 对话生成器 {
    fn 对话(&self, 消息: Vec<对话消息>, 工具: Vec<serde_json::Value>) -> Result<模型响应> {
        let 消息json: Vec<serde_json::Value> = 消息.iter().map(消息转json).collect();
        let 造体 = |提供商: &模型提供商| {
            let mut 体 = json!({
                "model": &提供商.model,
                "messages": &消息json,
                "tools": &工具,
                "max_tokens": 输出令牌上限,
            });
            // tool_choice=auto：道祖接待三选一场景，引导模型在需要时稳定触发工具调用
            if !工具.is_empty() {
                体["tool_choice"] = json!("auto");
            }
            体
        };
        let 解析 = |值: &serde_json::Value| {
            let message = &值["choices"][0]["message"];
            Ok(模型响应 {
                内容: message["content"].as_str().map(|s| s.to_string()),
                工具调用: 解析工具调用(message),
                思考: None,
            })
        };
        self.逐个生成(&造体, &解析)
    }
}

impl 流式对话器 for 对话生成器 {
    fn 对话流式(
        &self,
        消息: Vec<对话消息>,
        工具: Vec<serde_json::Value>,
        on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), Error>,
    ) -> Result<模型响应> {
        match self.主.流式对话(消息.clone(), &工具, on_chunk) {
            Ok(响应) => Ok(响应),
            Err(主错误) => {
                tracing::warn!("主模型流式请求失败，尝试降级备选: {主错误}");
                match &self.备选 {
                    Some(备) => 备.流式对话(消息, &工具, on_chunk),
                    None => Err(主错误),
                }
            }
        }
    }
}

/// 将对话消息转为 OpenAI 兼容 API 的 messages 元素（供 对话生成器 与 LLM池 共用）
pub fn 消息转json(消息: &对话消息) -> serde_json::Value {
    match 消息.角色 {
        消息角色::System | 消息角色::User => json!({
            "role": 消息.角色.序列化(),
            "content": 消息.内容.clone().unwrap_or_default(),
        }),
        消息角色::Assistant => {
            if 消息.工具调用.is_empty() {
                json!({ "role": "assistant", "content": 消息.内容.clone().unwrap_or_default() })
            } else {
                let 调用: Vec<serde_json::Value> = 消息
                    .工具调用
                    .iter()
                    .map(|t| json!({
                        "id": &t.id,
                        "type": "function",
                        "function": { "name": &t.名称, "arguments": &t.参数 },
                    }))
                    .collect();
                json!({ "role": "assistant", "tool_calls": 调用 })
            }
        }
        消息角色::Tool => json!({
            "role": "tool",
            "tool_call_id": 消息.工具调用id.clone().unwrap_or_default(),
            "content": 消息.内容.clone().unwrap_or_default(),
        }),
    }
}

/// 从模型响应消息里解析工具调用列表（供 对话生成器 与 LLM池 共用）
pub fn 解析工具调用(message: &serde_json::Value) -> Vec<工具调用> {
    let mut 结果 = Vec::new();
    if let Some(调用数组) = message["tool_calls"].as_array() {
        for 条目 in 调用数组 {
            let id = 条目["id"].as_str().unwrap_or_default().to_string();
            let 名称 = 条目["function"]["name"].as_str().unwrap_or_default().to_string();
            let 参数 = 规范参数文本(&条目["function"]["arguments"]);
            if !名称.is_empty() {
                结果.push(工具调用 { id, 名称, 参数 });
            }
        }
    }
    结果
}

/// 规范化 tool_call 的 arguments 为可 JSON 解析的文本。
///
/// 只做 `as_str()` 会在这三种常见网关/模型形态下静默变成空串，下游执行器随即报
/// 「解析工具参数失败: EOF while parsing a value at line 1 column 0」——错误文案完全看不出真因
/// （2026-09-14 排查「LLM 传参不对」时定位）：
/// * arguments 是 JSON 对象/数组（未按规范转义成字符串）→ 直接序列化；
/// * arguments 为 null / 缺失 → 视为空参数 `{}`；
/// * 文本被 markdown 代码围栏或首尾空白包裹（```json {...} ```）→ 剥离后取正文。
fn 规范参数文本(值: &serde_json::Value) -> String {
    let 原文 = match 值 {
        serde_json::Value::String(文) => 文.as_str(),
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => return 值.to_string(),
        _ => return "{}".to_string(),
    };
    let 修剪 = 原文.trim();
    let 修剪 = 修剪
        .strip_prefix("```json")
        .or_else(|| 修剪.strip_prefix("```"))
        .unwrap_or(修剪);
    let 修剪 = 修剪.strip_suffix("```").unwrap_or(修剪).trim();
    if 修剪.is_empty() {
        "{}".to_string()
    } else {
        修剪.to_string()
    }
}
