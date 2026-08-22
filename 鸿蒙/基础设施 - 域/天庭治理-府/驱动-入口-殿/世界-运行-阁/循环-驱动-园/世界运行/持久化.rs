//! 持久化：列表原子落盘泛型 + 要求/任务线列表落盘。
//!
//! 拆出自 `世界运行.rs`（D2 §2.10.1），职责单一：把列表项序列化为 jsonl 后原子落盘
//! （写临时文件再 rename，避免半写损坏）。泛型 `持久化列表` 消除要求/任务线两份重复。

use crate::类型_定义_殿::{任务线, 要求书};

/// 列表原子落盘泛型（消除 持久化要求们/持久化任务线们 重复）：
/// 写临时文件再 rename，避免半写损坏 jsonl。`类型名` 用于序列化错误信息。
pub(super) fn 持久化列表<T: serde::Serialize>(
    路径: &std::path::Path,
    项们: &[T],
    类型名: &str,
) -> Result<(), String> {
    let mut 行们 = Vec::with_capacity(项们.len());
    for 项 in 项们 {
        let 行 = serde_json::to_string(项).map_err(|错误| format!("序列化{类型名}失败: {错误}"))?;
        行们.push(行);
    }
    let 内容 = if 行们.is_empty() {
        String::new()
    } else {
        format!("{}\n", 行们.join("\n"))
    };
    let 临时路径 = 路径.with_extension("jsonl.tmp");
    std::fs::write(&临时路径, &内容).map_err(|错误| format!("写临时文件失败: {错误}"))?;
    std::fs::rename(&临时路径, 路径).map_err(|错误| format!("原子改名失败: {错误}"))?;
    Ok(())
}

/// 要求列表原子落盘：写临时文件再 rename，避免半写损坏 jsonl。
pub(super) fn 持久化要求们(
    队列路径: &std::path::Path,
    项们: &[要求书],
) -> Result<(), String> {
    持久化列表(队列路径, 项们, "要求")
}

/// 任务线落盘（读改写后原子重写，与 持久化要求们 同款）。
pub(super) fn 持久化任务线们(
    路径: &std::path::Path, 项们: &[任务线]
) -> Result<(), String> {
    持久化列表(路径, 项们, "任务线")
}
