use serde::{Deserialize, Serialize};
use hm_cognition::AgentRole;
use crate::任务模型_殿::TaskStatus;

/// 任务场景：任务的执行场景类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskScene {
    #[default]
    理解,
    设计,
    修改,
    调试,
    重构,
}

/// 任务优先级：P0 最高，P3 最低
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskPriority {
    P0,
    P1,
    #[default]
    P2,
    P3,
}

/// 状态变更记录：一次状态流转的元信息（谁、何时、从何到何、备注）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusChange {
    pub 原状态: TaskStatus,
    pub 新状态: TaskStatus,
    pub 操作者: AgentRole,
    pub 时间: u64,
    pub 备注: Option<String>,
}