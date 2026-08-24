//! 调度接单：5 维评估 + 12 红线。
//!
//! 动机（界主 2026-08-24）：哪些可以、哪些不行需根据现实判断，不能预先编码。
//! 框架而非规则——五个维度是结构化表达，最终由天道/鸿钧当下判断。
//!
//! 5 维：
//!   1 候选池：候选池不空时，本候选非优先 → 拒绝
//!   2 世界状态：有未救火失败 → 拒绝（先救火）
//!   3 影响范围：跨 ≥2 府 → 拒绝（须界主确认）
//!   4 可逆性：不可逆改动 → 拒绝（改核心算法/历史）
//!   5 可验证性：无 #[test] 覆盖 → 拒绝（无法验证=无法做）
//!
//! 12 红线：
//!   - .git/ 下任何东西
//!   - .md 设计稿（除非 当前架构现状.md）
//!   - AGENTS.md 铁律
//!   - Cargo.toml 的 members/dependencies
//!   - .env 密钥/地址
//!   - 核心算法逻辑（未经界主确认）
//!   - .上下文/状态/*.jsonl 历史记录
//!   - 已存档版本快照
//!   - LLM 调用凭据
//!   - .github/workflows
//!   - Cargo.lock 手写修改
//!   - 甲阶段做美化

use crate::类型_定义_殿::{巡世候选, 本质档位};
use std::path::Path;

/// 接单决策。
#[derive(Clone, Debug, PartialEq)]
pub enum 接单决策 {
    接受,
    拒绝(&'static str),
}

/// 评估接单：5 维评估 + 12 红线检查。
///
/// 入参：候选 + 候选池是否空 + 涉及路径(用于影响范围评估) + 工作区根(用于红线检查)。
/// 出参：接单决策（接受/拒绝+原因）。
pub fn 评估接单(
    候选: &巡世候选,
    _候选池已空: bool,
    涉及路径: &[&Path],
    工作区根: &Path,
) -> 接单决策 {
    // 维度 1：候选池非空 + S0 抢占放行；其他档位假设已被「档位优先」选出 → 通过。
    // （维度 1 的实际判定移到调用方「调度驱动」的「档位优先选下一个」里完成，
    //  本函数只验证其他维度。2026-08-25 修复：旧逻辑「池非空 + 非 S0 → 拒」错误拒所有候选。）

    // 维度 3：影响范围（跨 ≥2 府）—— 先判（影响范围与档位无关）
    let 涉及府: std::collections::HashSet<&str> = 涉及路径
        .iter()
        .filter_map(|p| p.to_str())
        .filter_map(|s| s.split("鸿蒙/\\").nth(1).map(|rest| rest.split('/').next().unwrap_or("")))
        .collect();
    if 涉及府.len() >= 2 {
        return 接单决策::拒绝("跨 ≥2 府改动须界主确认");
    }

    // 维度 4：可逆性（不可逆 = 改核心算法/历史）
    if 候选.依据.contains("不可逆") || 候选.依据.contains("删除") {
        return 接单决策::拒绝("不可逆改动");
    }

    // 红线 12 条——早判（红线比「可验证」严，触红线就拒）
    if 触碰红线(涉及路径, 工作区根) {
        return 接单决策::拒绝("触碰红线");
    }

    // 维度 5：可验证性（无测试覆盖）—— 末判
    // 2026-08-25 修复：原 <= S6 因 PartialOrd 实际反向导致判断反（紧急的反而被放行）。
    // 正确语义：档位 ≥ S6（预防/改进类）要求有测试；S0~S5（补救类）紧急可无测试。
    if !候选.依据.contains("测试") && !候选.依据.contains("验证") && 候选.本质档位 >= 本质档位::S6 {
        return 接单决策::拒绝("无 #[test] 覆盖，无法验证");
    }

    // 维度 2：世界状态——本函数签名未接世界状态，留给调用方在入队前自检「有未救火失败」场景。
    // 阶段二可通过「封装 接单上下文」扩展签名。

    接单决策::接受
}

/// 红线检查：13 条永远不碰（2026-08-25 增 红线 13 工具链保护）。
///
/// 红线 13：`.git/`、`.github/`、`.vscode/`、`.idea/`、`.codegraph/` 等是 Git/GitHub/IDE/工具链硬约定的目录与文件名。
/// 涉及路径若整个位于这些目录内 → 拒。这条与「道韵违逆顶层目录未用中文命名」豁免关联（违逆扫描会跳过这些目录）。
pub fn 触碰红线(涉及路径: &[&Path], 工作区根: &Path) -> bool {
    for 路径 in 涉及路径 {
        let 路径串 = 路径.to_string_lossy();

        // 红线 1: .git/
        if 路径串.contains("\\.git\\") || 路径串.contains("/.git/") {
            return true;
        }

        // 红线 2: .md 设计稿（除非 当前架构现状.md）
        if 路径串.ends_with(".md") && !路径串.ends_with("当前架构现状.md") {
            return true;
        }

        // 红线 4: Cargo.toml 的 members/dependencies
        // 简化检查：任何 Cargo.toml 改动都要求界主确认（保守）
        if 路径串.ends_with("Cargo.toml") {
            return true;
        }

        // 红线 5: .env
        if 路径串.ends_with(".env") || 路径串.contains("\\.env") {
            return true;
        }

        // 红线 7: .上下文/状态/*.jsonl
        if 路径串.contains(".上下文/状态/") && 路径串.ends_with(".jsonl") {
            return true;
        }

        // 红线 8: 已存档版本快照（.上下文/版本库/）
        if 路径串.contains(".上下文/版本库/") {
            return true;
        }

        // 红线 10: .github/workflows
        if 路径串.contains(".github/workflows/") {
            return true;
        }

        // 红线 11: Cargo.lock
        if 路径串.ends_with("Cargo.lock") {
            return true;
        }

        // 红线 13: 工具链约定目录（不可改名中文）
        // 涉及路径若整个位于这些目录之一 → 拒。
        if 路径串.contains("\\.git\\")
            || 路径串.contains("\\.github\\")
            || 路径串.contains("\\.vscode\\")
            || 路径串.contains("\\.idea\\")
            || 路径串.contains("\\.codegraph\\")
        {
            return true;
        }
    }
    let _ = 工作区根; // 暂未使用，保留扩展位
    false
}

// 红线 6 + 9 + 12 由调用方在「实施前最终判定」阶段检查（涉及核心算法/凭据/阶段判断）。
// 本函数只机械检查路径层面。
