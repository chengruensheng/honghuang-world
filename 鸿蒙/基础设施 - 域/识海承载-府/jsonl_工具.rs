//! §B.2.8 jsonl schema 校验 + 截断修复（§B.0 读格位容错基础上加强）。
//!
//! 项目当前 jsonl 文件（.上下文/状态/*.jsonl + .上下文/观测/*.jsonl）缺 schema 校验。
//! 读取时遇 schema 错误（key 缺失 / 类型错）→ 跳过该行（不 panic），并 warn 日志。
//! 截断修复：最后一行不完整（JSON parse fail）→ 截断文件。

use crate::世界错误::世界错误;
use std::path::Path;

/// 读 jsonl 文件为 T — 容错：单行错跳过 + warn。
/// 截断修复：发现末行不完整则截断（仅保留完整行）。
pub fn 读_jsonl<T: serde::de::DeserializeOwned>(路径: &Path) -> Result<Vec<T>, 世界错误> {
    let 内容 = std::fs::read_to_string(路径).map_err(世界错误::from)?;
    let mut 结果 = Vec::new();
    let 行们: Vec<&str> = 内容.lines().collect();
    let 末 = 行们.len();
    let mut 截断位置 = 末; // 完整行数
    for (i, 行) in 行们.iter().enumerate() {
        if 行.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(行) {
            Ok(项) => 结果.push(项),
            Err(_e) if i == 末 - 1 && !行.ends_with('}') && !行.ends_with(']') => {
                // 末行不完整（不闭合括号）→ 截断
                截断位置 = i;
            }
            Err(_e) => {
                // 中间行 schema 错 — 跳过 + warn（沙箱兼容）
                rizhi_fu::warn!(路径 = %路径.display(), 行号 = i, "jsonl schema 错，跳过该行");
            }
        }
    }
    if 截断位置 < 末 {
        截断到完整行(路径, 截断位置)?;
    }
    Ok(结果)
}

/// 截断文件到指定行数（保留前 N 行完整 + \n）。
fn 截断到完整行(路径: &Path, 行数: usize) -> Result<(), 世界错误> {
    let 内容 = std::fs::read_to_string(路径).map_err(世界错误::from)?;
    let 截断: String = 内容.lines().take(行数).collect::<Vec<_>>().join("\n") + "\n";
    std::fs::write(路径, 截断).map_err(世界错误::from)?;
    rizhi_fu::warn!(路径 = %路径.display(), 行数, "jsonl 末行不完整已截断");
    Ok(())
}
