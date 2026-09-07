use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 一次层级处理产生的产物记录：文件路径 + 内容摘要 + 提交哈希 + 关联任务。
///
/// 供按文件路径追溯「哪个层级/处理者/时间产出了该文件」。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct 产物记录 {
    pub 文件路径: String,
    pub 内容摘要: String,
    /// 提交哈希（未提交时为 None）
    pub 提交哈希: Option<String>,
    /// 关联任务唯一标识
    pub 关联任务id: Uuid,
}

impl 产物记录 {
    pub fn 新(文件路径: impl Into<String>, 内容摘要: impl Into<String>, 关联任务id: Uuid) -> Self {
        产物记录 {
            文件路径: 文件路径.into(),
            内容摘要: 内容摘要.into(),
            提交哈希: None,
            关联任务id,
        }
    }

    /// 记录提交哈希（实现层提交代码后由驱动器回填）
    pub fn 带提交(mut self, 哈希: impl Into<String>) -> Self {
        self.提交哈希 = Some(哈希.into());
        self
    }
}
