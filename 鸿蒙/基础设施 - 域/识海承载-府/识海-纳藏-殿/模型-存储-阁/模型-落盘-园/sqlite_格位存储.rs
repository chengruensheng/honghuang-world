//! Sqlite 格位存储（§B.2.2 三个实现之二 — 骨架，待 B.3 实装）。
//!
//! 当前为骨架：占位 trait 实现 + TODO。
//! 后续：依赖 `rusqlite` crate + 写 .上下文/格位.db 单一文件。

use super::工作区;
use super::格位存储;
use crate::世界结果;
use crate::记录;

/// Sqlite 格位存储
pub struct Sqlite格位存储 {
    _私有: (),
}

impl Sqlite格位存储 {
    pub fn 新() -> Self {
        Self { _私有: () }
    }
}

impl 格位存储 for Sqlite格位存储 {
    fn 写记录(&self, _记录: &记录) -> 世界结果<()> {
        // TODO B.3: rusqlite insert + 索引
        Err("Sqlite格位存储 未实装 — B.3 阶段加 rusqlite"
            .to_string()
            .into())
    }
    fn 在工作区(_工作区: &工作区) -> Box<dyn 格位存储> {
        Box::new(Sqlite格位存储::新())
    }
}
