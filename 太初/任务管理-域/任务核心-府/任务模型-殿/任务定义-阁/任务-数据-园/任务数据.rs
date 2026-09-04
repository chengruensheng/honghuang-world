use serde::{Deserialize, Serialize};
use crate::任务模型_殿::TaskStatus;

/// 任务：太初之木，从无到有的存在
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: u64,
}

impl Task {
    pub fn 新建(id: u64, title: String, description: String, created_at: u64) -> Self {
        Task {
            id,
            title,
            description,
            status: TaskStatus::待受理,
            created_at,
        }
    }
}