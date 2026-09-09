use serde::{Deserialize, Serialize};

/// RUN_STARTED：一次 agent run 开始，建立 run/thread 上下文。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStarted {
    /// 会话线程 id
    pub thread_id: String,
    /// 本次 run 的唯一 id
    pub run_id: String,
    /// 父 run id（分支/时间旅行时指向来源）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
    /// 本次 run 的输入载荷
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
}

/// RUN_FINISHED：一次 agent run 成功结束。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunFinished {
    /// 会话线程 id
    pub thread_id: String,
    /// 本次 run 的唯一 id（对应 RUN_STARTED）
    pub run_id: String,
    /// 输出数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
}

/// RUN_ERROR：不可恢复错误，终止 run。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunError {
    /// 错误描述
    pub message: String,
    /// 错误码（如 abort / timeout）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// STEP_STARTED：命名步骤开始（可选，用于进度可见性）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepStarted {
    /// 步骤名（如节点/函数名）
    pub step_name: String,
}

/// STEP_FINISHED：命名步骤结束。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepFinished {
    /// 步骤名（对应 STEP_STARTED）
    pub step_name: String,
}
