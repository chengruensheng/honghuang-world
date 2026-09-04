use serde::{Deserialize, Serialize};
use crate::版本定义_殿::Version;

/// 迭代状态：进行中 → 已完成 / 已放弃（火之焚炼与重生）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum 迭代状态 {
    进行中,
    已完成,
    已放弃,
}

/// 迭代：一次版本演进记录（火之变革）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iteration {
    pub id: u64,
    pub version: Version,
    pub 变更说明: String,
    pub status: 迭代状态,
    pub created_at: u64,
}

impl Iteration {
    pub fn 新建(id: u64, version: Version, 变更说明: String, created_at: u64) -> Self {
        Iteration {
            id,
            version,
            变更说明,
            status: 迭代状态::进行中,
            created_at,
        }
    }
}
