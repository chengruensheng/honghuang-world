use serde::{Deserialize, Serialize};

/// TEXT_MESSAGE_START：开始一条流式文本消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextMessageStart {
    /// 消息唯一 id（后续 CONTENT/END 须一致）
    pub message_id: String,
    /// 发送角色（developer / system / assistant / user）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// TEXT_MESSAGE_CONTENT：文本内容增量。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextMessageContent {
    /// 消息唯一 id（对应 TEXT_MESSAGE_START）
    pub message_id: String,
    /// 文本增量（非空）
    pub delta: String,
}

/// TEXT_MESSAGE_END：文本消息结束。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextMessageEnd {
    /// 消息唯一 id（对应 TEXT_MESSAGE_START）
    pub message_id: String,
}
