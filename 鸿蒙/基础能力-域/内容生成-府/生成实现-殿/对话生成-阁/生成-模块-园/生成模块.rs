use std::time::Duration;
use hm_content_contract::{内容生成器, 工具对话器, 对话消息, 消息角色, 工具调用, 模型响应};
use hm_contract::Component;
use hm_error::{Error, Result};
use serde_json::json;

/// 单次请求超时（秒）
const 请求超时秒: u64 = 30;
/// 失败后额外重试次数（首次 + 重试 = 总尝试次数）
const 请求重试次数: u32 = 2;
/// 重试间隔（秒）
const 请求重试间隔秒: u64 = 2;

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
    /// 单次 HTTP 请求：发送 body 并解析 JSON 响应体（不重试）
    fn 请求(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
        let resp = ureq::post(&self.base_url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(请求超时秒))
            .send_string(&body.to_string())
            .map_err(|e| Error::模型(format!("请求模型失败: {e}")))?;
        let text = resp
            .into_string()
            .map_err(|e| Error::模型(format!("读取模型响应失败: {e}")))?;
        serde_json::from_str(&text)
            .map_err(|e| Error::模型(format!("解析模型响应失败: {e}")))
    }
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
        let 造体 = |提供商: &模型提供商| json!({
            "model": &提供商.model,
            "messages": &消息json,
            "tools": &工具,
        });
        let 解析 = |值: &serde_json::Value| {
            let message = &值["choices"][0]["message"];
            Ok(模型响应 {
                内容: message["content"].as_str().map(|s| s.to_string()),
                工具调用: 解析工具调用(message),
            })
        };
        self.逐个生成(&造体, &解析)
    }
}

/// 将对话消息转为 OpenAI 兼容 API 的 messages 元素
fn 消息转json(消息: &对话消息) -> serde_json::Value {
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

/// 从模型响应消息里解析工具调用列表
fn 解析工具调用(message: &serde_json::Value) -> Vec<工具调用> {
    let mut 结果 = Vec::new();
    if let Some(调用数组) = message["tool_calls"].as_array() {
        for 条目 in 调用数组 {
            let id = 条目["id"].as_str().unwrap_or_default().to_string();
            let 名称 = 条目["function"]["name"].as_str().unwrap_or_default().to_string();
            let 参数 = 条目["function"]["arguments"].as_str().unwrap_or_default().to_string();
            if !名称.is_empty() {
                结果.push(工具调用 { id, 名称, 参数 });
            }
        }
    }
    结果
}
