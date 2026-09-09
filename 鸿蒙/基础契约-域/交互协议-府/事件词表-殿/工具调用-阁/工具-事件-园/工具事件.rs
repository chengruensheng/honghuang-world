use serde::{Deserialize, Serialize};

/// TOOL_CALL_START：工具调用开始。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallStart {
    /// 工具调用唯一 id（ARGS/END/RESULT 须一致）
    pub tool_call_id: String,
    /// 工具名
    pub tool_call_name: String,
    /// 父消息 id（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<String>,
}

/// TOOL_CALL_ARGS：工具调用参数增量。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallArgs {
    /// 工具调用唯一 id
    pub tool_call_id: String,
    /// 参数增量（字符串）
    pub delta: String,
}

/// TOOL_CALL_END：工具调用结束。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallEnd {
    /// 工具调用唯一 id
    pub tool_call_id: String,
}

/// TOOL_CALL_RESULT：工具执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    /// 工具调用唯一 id
    pub tool_call_id: String,
    /// 结果所属消息 id（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// 结果内容
    pub content: String,
}
