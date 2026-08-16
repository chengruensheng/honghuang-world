//! 模型连接-府 · 核心类型：模型请求与响应。

use serde::{Deserialize, Serialize};

/// 一条对话消息。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 对话消息 {
    pub 角色: String,
    pub 内容: String,
    /// assistant 消息回传模型上次的工具调用（工具循环用）；纯文本对话为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub 工具调用们: Option<Vec<工具调用>>,
    /// tool 角色消息回传执行结果对应的调用标识；其他角色为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub 工具调用标识: Option<String>,
}

impl 对话消息 {
    /// 构造一条用户消息。
    pub fn 用户(内容: impl Into<String>) -> 对话消息 {
        对话消息 { 角色: "user".to_string(), 内容: 内容.into(), 工具调用们: None, 工具调用标识: None }
    }

    /// 构造一条系统消息。
    pub fn 系统(内容: impl Into<String>) -> 对话消息 {
        对话消息 { 角色: "system".to_string(), 内容: 内容.into(), 工具调用们: None, 工具调用标识: None }
    }

    /// assistant 消息：回传本轮工具调用，内容保留模型原回复（含 think，官方要求多轮保留完整 assistant 消息）。
    pub fn 助手_带工具调用(内容: impl Into<String>, 调用们: Vec<工具调用>) -> 对话消息 {
        对话消息 { 角色: "assistant".to_string(), 内容: 内容.into(), 工具调用们: Some(调用们), 工具调用标识: None }
    }

    /// tool 消息：回传某次工具调用的执行结果。
    pub fn 工具结果(标识: impl Into<String>, 结果: impl Into<String>) -> 对话消息 {
        对话消息 { 角色: "tool".to_string(), 内容: 结果.into(), 工具调用们: None, 工具调用标识: Some(标识.into()) }
    }
}

/// 工具定义：函数名 + 描述 + 参数 JSON Schema（OpenAI 兼容 function calling）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 工具定义 {
    pub 名字: String,
    pub 描述: String,
    pub 参数: serde_json::Value,
}

/// 工具调用：模型请求执行一个工具。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 工具调用 {
    /// 模型返回的调用标识（tool_call_id），回传执行结果时须一致。
    pub 标识: String,
    pub 名字: String,
    pub 参数: String,
}

/// 模型回复：文本内容或工具调用。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 模型回复 {
    文本(String),
    /// 工具调用，附模型原回复内容（含 think，多轮回传须保留）。
    工具调用(String, Vec<工具调用>),
    /// 工具调用参数缺失/非法（arguments 为空、{} 或非法 JSON），携带缺失参数的工具名；由上层引导模型重发完整参数。
    参数缺失(Vec<String>),
}

/// 一次模型调用的 token 用量（含缓存命中，便于成本观测与对账）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 用量 {
    /// 提示词 token 数。
    pub 提示词: u64,
    /// 输出 token 数。
    pub 输出: u64,
    /// 缓存命中 token 数（cache_read_input_tokens / prompt_tokens_details.cached_tokens）。
    pub 缓存命中: u64,
    /// 总 token 数。
    pub 总计: u64,
}

impl 用量 {
    /// 累计另一次用量（多轮 / 多任务聚合用）。
    pub fn 加(&mut self, 其他: &用量) {
        self.提示词 += 其他.提示词;
        self.输出 += 其他.输出;
        self.缓存命中 += 其他.缓存命中;
        self.总计 += 其他.总计;
    }
}
