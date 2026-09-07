use serde::{Deserialize, Serialize};

/// 漂移项类型：声明未兑现 / 未声明的多余新增
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum 漂移类型 {
    /// 实现文档声明了改动，但工作区不存在该文件（声明未兑现）
    声明未兑现,
    /// 工作区存在文件，但实现文档未声明（越界新增）
    未声明新增,
}

/// 一条漂移项：路径 + 类型 + 说明
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct 漂移项 {
    pub 类型: 漂移类型,
    pub 路径: String,
    pub 说明: String,
}

/// 漂移检测报告：声明文件集 × 实际文件集的核对结果
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct 漂移报告 {
    /// 实现文档声明的文件路径总数（去重后）
    pub 声明数: usize,
    /// 工作区实际文件总数（剔除系统/构建目录后）
    pub 实际数: usize,
    /// 声明且实际存在的文件数
    pub 兑现数: usize,
    /// 声明但工作区不存在的路径数
    pub 未兑现数: usize,
    /// 工作区存在但未声明的路径数
    pub 多余数: usize,
    pub 漂移项: Vec<漂移项>,
}

impl 漂移报告 {
    /// 是否零漂移（声明全部兑现且无多余新增）
    pub fn 零漂移(&self) -> bool {
        self.未兑现数 == 0 && self.多余数 == 0
    }
}

/// 扫尾记录：一次扫尾检查的完整结论，写入任务作为交付证据链的一环
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct 扫尾记录 {
    /// 检查发生秒级时间戳
    pub 检查时间: u64,
    /// 变更总数（实现文档声明、去重后）
    pub 变更总数: usize,
    /// 声明且实际兑现数
    pub 兑现数: usize,
    /// 声明未兑现数
    pub 未兑现数: usize,
    /// 未声明的多余新增数
    pub 多余数: usize,
    /// 是否通过（未兑现/多余/自检不过 任一即不通过）
    pub 通过: bool,
    pub 漂移项: Vec<漂移项>,
    /// 说明（自检不通过等补充信息）
    pub 说明: String,
}
