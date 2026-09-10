use hm_contract::Component;
use hm_error::Result;
use serde::{Deserialize, Serialize};

/// 消息角色：function calling 多轮对话的角色标记
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum 消息角色 {
    System,
    User,
    Assistant,
    Tool,
}

impl 消息角色 {
    /// 转为 OpenAI 兼容 API 的 role 字符串
    pub fn 序列化(&self) -> &'static str {
        match self {
            消息角色::System => "system",
            消息角色::User => "user",
            消息角色::Assistant => "assistant",
            消息角色::Tool => "tool",
        }
    }
}

/// 工具调用：模型返回的一次工具调用意图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 工具调用 {
    pub id: String,
    pub 名称: String,
    pub 参数: String,
}

/// 对话消息：多轮 function calling 的一条消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 对话消息 {
    pub 角色: 消息角色,
    pub 内容: Option<String>,
    pub 工具调用: Vec<工具调用>,
    pub 工具调用id: Option<String>,
}

impl 对话消息 {
    /// 构造系统提示消息
    pub fn 系统(内容: impl Into<String>) -> Self {
        对话消息 { 角色: 消息角色::System, 内容: Some(内容.into()), 工具调用: Vec::new(), 工具调用id: None }
    }
    /// 构造用户任务消息
    pub fn 用户(内容: impl Into<String>) -> Self {
        对话消息 { 角色: 消息角色::User, 内容: Some(内容.into()), 工具调用: Vec::new(), 工具调用id: None }
    }
    /// 构造助手的工具调用消息
    pub fn 助手调用(调用: Vec<工具调用>) -> Self {
        对话消息 { 角色: 消息角色::Assistant, 内容: None, 工具调用: 调用, 工具调用id: None }
    }
    /// 构造工具执行结果消息
    pub fn 工具结果(id: impl Into<String>, 内容: impl Into<String>) -> Self {
        对话消息 { 角色: 消息角色::Tool, 内容: Some(内容.into()), 工具调用: Vec::new(), 工具调用id: Some(id.into()) }
    }
}

/// 模型响应：文本内容与工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 模型响应 {
    pub 内容: Option<String>,
    pub 工具调用: Vec<工具调用>,
    pub 思考: Option<String>,
}

/// 工具对话器契约：多轮对话 + 工具调用（function calling）
///
/// `工具` 为 tools 数组的 JSON 元素（由调用方按 OpenAI 工具定义格式构造），
/// 实现负责拼装请求、解析 `tool_calls` 响应。
pub trait 工具对话器: Component {
    fn 对话(&self, 消息: Vec<对话消息>, 工具: Vec<serde_json::Value>) -> Result<模型响应>;
}

/// 流式对话器契约：多轮对话 + 工具调用 + 增量内容回调（打字机式）。
///
/// 语义与 工具对话器::对话 等价（返回完整 模型响应 供后续解析意图），
/// 差异仅在生成过程中通过 on_chunk 实时推送文本增量。
/// 实现须保证：回调按序、无遗漏；任一回调返回 Err 时中止并向上传播。
pub trait 流式对话器: 工具对话器 + Component {
    fn 对话流式(
        &self,
        消息: Vec<对话消息>,
        工具: Vec<serde_json::Value>,
        on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), hm_error::Error>,
    ) -> Result<模型响应>;
}