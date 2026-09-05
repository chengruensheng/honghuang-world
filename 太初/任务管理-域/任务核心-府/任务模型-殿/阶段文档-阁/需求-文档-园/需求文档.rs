use serde::{Deserialize, Serialize};
use crate::任务模型_殿::TaskPriority;

/// 道祖需求文档：需求的极致表达（目标/背景/约束/验收标准）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementDoc {
    pub 目标: String,
    pub 背景: String,
    #[serde(default)]
    pub 约束: Vec<String>,
    #[serde(default)]
    pub 功能需求: Vec<String>,
    #[serde(default)]
    pub 非功能需求: Vec<String>,
    #[serde(default)]
    pub 验收标准: Vec<String>,
    pub 优先级: TaskPriority,
    pub created_at: u64,
}