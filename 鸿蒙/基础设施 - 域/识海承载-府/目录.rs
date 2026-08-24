//! §B.2.5 统一目录路径：12 crate 共享 .上下文/状态/ + .上下文/观测/。
//!
//! 替换各府各自写的 状态目录/观测目录 fn（之前 jiance_fu/tianting_fu/... 各自实现）。

use std::path::PathBuf;

/// .上下文/状态/ 绝对路径（落 状态 jsonl + 验收 jsonl + ...）。
pub fn 状态目录() -> PathBuf {
    crate::工作区::定位().上下文目录().join("状态")
}

/// .上下文/观测/ 绝对路径（落 记录 jsonl）。
pub fn 观测目录() -> PathBuf {
    crate::工作区::定位().上下文目录().join("观测")
}
