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

impl TaskScene {
    /// 按中文场景名解析，未知返回 None（跨府共享：看板 API / 道祖对齐 复用）
    pub fn 解析(s: &str) -> Option<Self> {
        match s {
            "理解" => Some(TaskScene::理解),
            "设计" => Some(TaskScene::设计),
            "修改" => Some(TaskScene::修改),
            "调试" => Some(TaskScene::调试),
            "重构" => Some(TaskScene::重构),
            _ => None,
        }
    }
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

impl TaskPriority {
    /// 按优先级名解析，未知返回 None（跨府共享：看板 API / 道祖对齐 复用）
    pub fn 解析(s: &str) -> Option<Self> {
        match s {
            "P0" => Some(TaskPriority::P0),
            "P1" => Some(TaskPriority::P1),
            "P2" => Some(TaskPriority::P2),
            "P3" => Some(TaskPriority::P3),
            _ => None,
        }
    }
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