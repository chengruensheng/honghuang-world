use serde::{Deserialize, Serialize};

/// STATE_SNAPSHOT：状态全量快照。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateSnapshot {
    /// 任意 JSON 结构的状态快照
    pub snapshot: serde_json::Value,
}

/// STATE_DELTA：状态增量更新（RFC 6902 JSON Patch 数组）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateDelta {
    /// JSON Patch 操作数组，如 `[{ "op": "replace", "path": "/progress", "value": 75 }]`
    pub delta: Vec<serde_json::Value>,
}
