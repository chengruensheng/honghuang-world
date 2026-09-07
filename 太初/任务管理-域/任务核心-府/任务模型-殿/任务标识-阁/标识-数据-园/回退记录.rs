use serde::{Deserialize, Serialize};
use crate::任务模型_殿::五行层级;

/// 一次定向回退的记录：谁在何时因何原因把任务回退到哪个层级。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct 回退记录 {
    /// 回退发生时间（秒级时间戳）
    pub 回退时间: u64,
    /// 来源层级（验收发现问题的层级）
    pub 来源层级: 五行层级,
    /// 目标层级（追溯出的错误根源层级）
    pub 目标层级: 五行层级,
    /// 回退原因（简短归类）
    pub 原因: String,
    /// 错误描述（验收不通过的错误现象）
    pub 错误描述: String,
    /// 累计回退次数（含本次）
    pub 回退次数: u32,
}

impl 回退记录 {
    pub fn 新(
        回退时间: u64,
        来源层级: 五行层级,
        目标层级: 五行层级,
        原因: impl Into<String>,
        错误描述: impl Into<String>,
        回退次数: u32,
    ) -> Self {
        回退记录 {
            回退时间,
            来源层级,
            目标层级,
            原因: 原因.into(),
            错误描述: 错误描述.into(),
            回退次数,
        }
    }
}
