use serde::{Deserialize, Serialize};

/// 记忆：土之承载，经验沉淀的单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: u64,
    pub 内容: String,
    pub 标签: String,
    pub created_at: u64,
    /// 是否待归档（木克土：记忆过盛时新记忆降级标注，而非删除旧记忆）
    #[serde(default)]
    pub 归档: bool,
}

impl Memory {
    pub fn 新建(id: u64, 内容: String, 标签: String, created_at: u64) -> Self {
        Memory { id, 内容, 标签, created_at, 归档: false }
    }
}
