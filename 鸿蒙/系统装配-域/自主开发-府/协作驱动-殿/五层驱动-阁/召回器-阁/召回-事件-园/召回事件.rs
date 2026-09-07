use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 召回事件状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum 召回事件状态 {
    召回中,
    已解除,
}

/// 召回事件：触发任务回退后，受影响任务被召回一次的完整记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 召回事件 {
    /// 触发召回的（回退）任务唯一标识
    pub 触发任务id: Uuid,
    pub 召回原因: String,
    /// 受影响被召回的任务列表
    pub 影响任务: Vec<Uuid>,
    /// 召回时间（秒级时间戳）
    pub 召回时间: u64,
    pub 状态: 召回事件状态,
}

impl 召回事件 {
    pub fn 新(触发任务id: Uuid, 召回原因: impl Into<String>, 影响任务: Vec<Uuid>, 召回时间: u64) -> Self {
        召回事件 {
            触发任务id,
            召回原因: 召回原因.into(),
            影响任务,
            召回时间,
            状态: 召回事件状态::召回中,
        }
    }
}
