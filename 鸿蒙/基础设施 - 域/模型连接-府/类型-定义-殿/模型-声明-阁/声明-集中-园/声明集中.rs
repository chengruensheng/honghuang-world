//! 模型连接-府 · 核心类型：模型请求与响应。

use serde::{Deserialize, Serialize};

/// 一条对话消息。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 对话消息 {
    pub 角色: String,
    pub 内容: String,
}

impl 对话消息 {
    /// 构造一条用户消息。
    pub fn 用户(内容: impl Into<String>) -> 对话消息 {
        对话消息 { 角色: "user".to_string(), 内容: 内容.into() }
    }

    /// 构造一条系统消息。
    pub fn 系统(内容: impl Into<String>) -> 对话消息 {
        对话消息 { 角色: "system".to_string(), 内容: 内容.into() }
    }
}
