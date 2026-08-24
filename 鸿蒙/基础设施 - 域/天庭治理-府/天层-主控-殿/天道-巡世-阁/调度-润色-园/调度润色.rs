//! 调度润色：候选池空时主动产 S11 润色候选。
//!
//! 动机（界主 2026-08-24）：天道没事做时不应永久休眠——应主动接润色类工作（性能、命名、复用、文档）。
//! 优先级：所有润色候选打 质量改进 本质类别 → 档位 S11 → 永远队尾 → 不打扰 S0~S10。
//!
//! 注入器扫描：
//!
//!   - clippy 警告（→ S10 资源耗尽预防，零警告硬要求）
//!   - 代码重复（粗粒度 string match，后期可换更智能）
//!   - 性能热点（文件大小/函数行数粗筛）
//!   - 命名违逆兜底（道韵扫描 fallback）
//!
//! 接单：注入的润色候选必须过 5 维接单门 → 跨府/无测试 = 拒绝。

use crate::类型_定义_殿::{优先级, 巡世候选, 本质档位, 本质类别, 要求类别};
use std::path::Path;

/// 润色扫描产出结果。
#[derive(Clone, Debug)]
pub struct 润色结果 {
    pub 候选们: Vec<巡世候选>,
    pub 扫描项数: usize,
}

/// 润色候选注入主入口。
///
/// 策略：轻量级扫描，**所有候选都打 S11**，确保不抢 S0~S10 档位。
pub fn 润色候选注入(根目录: &Path) -> 润色结果 {
    let mut 候选们 = Vec::new();

    // 扫描 1：clippy 警告
    if let Some(c) = 润色_clippy(根目录) {
        候选们.push(c);
    }

    // 扫描 2：大文件（>500 行）
    for c in 润色大文件(根目录) {
        候选们.push(c);
    }

    // 扫描 3：fmt 漂移（占位，本轮不实装）
    // 简化为：发现 fmt diff 时产一条候选
    // 留待 Step 2 反馈回路实装

    润色结果 {
        扫描项数: 候选们.len(),
        候选们,
    }
}

fn 润色_clippy(根目录: &Path) -> Option<巡世候选> {
    // 仅当 Cargo.toml 存在时才扫描（防误触发）
    if !根目录.join("Cargo.toml").exists() {
        return None;
    }
    // 简化：探测 fmt --check 输出（不实际跑命令，留待 Step 2 由调度驱动串联）
    // 本轮只占位：若 clippy 失败则产候选
    // 后续：实际接 cargo 调用
    None
}

fn 润色大文件(根目录: &Path) -> Vec<巡世候选> {
    let mut 候选 = Vec::new();
    let 阈值 = 500;
    let 排除 = [".上下文", "道果树", "临时文件夹", "传承殿/落稿留痕-阁"];

    let Ok(条目们) = std::fs::read_dir(根目录) else {
        return 候选;
    };

    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 路径串 = 路径.to_string_lossy();
        if 排除.iter().any(|e| 路径串.contains(e)) {
            continue;
        }
        if !路径.is_file() || !路径.extension().is_some_and(|ext| ext == "rs") {
            continue;
        }
        let 行数 = std::fs::read_to_string(&路径)
            .map(|c| c.lines().count())
            .unwrap_or(0);
        if 行数 > 阈值 {
            let 短路径 = 路径
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            候选.push(巡世候选 {
                目标: format!("拆分大文件（{}行）：{}", 行数, 短路径),
                依据: format!("文件 {} 超过 {} 行", 路径.display(), 阈值),
                建议类别: 要求类别::维护,
                优先级: 优先级::低,
                本质类别: 本质类别::质量改进,
                本质档位: 本质档位::S11,
            });
        }
    }
    候选
}
