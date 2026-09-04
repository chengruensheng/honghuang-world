use serde::{Deserialize, Serialize};

/// 任务状态：待受理 → 进行中 → 已完成（木之生长）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    待受理,
    进行中,
    已完成,
    已取消,
}

impl TaskStatus {
    /// 合法流转判断（状态机约束）
    pub fn 可流转到(&self, next: &TaskStatus) -> bool {
        matches!(
            (self, next),
            (TaskStatus::待受理, TaskStatus::进行中)
                | (TaskStatus::待受理, TaskStatus::已取消)
                | (TaskStatus::进行中, TaskStatus::已完成)
                | (TaskStatus::进行中, TaskStatus::已取消)
        )
    }
}
