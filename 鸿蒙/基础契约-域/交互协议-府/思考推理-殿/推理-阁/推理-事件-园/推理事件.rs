use serde::{Deserialize, Serialize};

/// REASONING_MESSAGE_START：推理（思考）消息开始。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningMessageStart {
    /// 推理消息唯一 id（后续 CONTENT/END 须一致）
    pub message_id: String,
}

/// REASONING_MESSAGE_CONTENT：推理消息内容增量。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningMessageContent {
    /// 推理消息唯一 id
    pub message_id: String,
    /// 内容增量
    pub delta: String,
}

/// REASONING_MESSAGE_END：推理消息结束。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningMessageEnd {
    /// 推理消息唯一 id
    pub message_id: String,
}
