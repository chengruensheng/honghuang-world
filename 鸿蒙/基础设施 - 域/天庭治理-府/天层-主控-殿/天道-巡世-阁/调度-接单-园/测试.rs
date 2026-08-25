//! 调度 - 接单 - 园 · 测试：智能接单门 5 维评估 + 12 红线单测。
//!
//! 依据：多智能体架构设计.md §19.4 智能接单门 + AGENTS.md 第 16 条铁律。
//! 目的：保护 commit 272cb6c 落地的接单门逻辑，避免未来重构破坏。

#![allow(clippy::useless_vec, dead_code, unused_imports)]

use super::调度接单::{
    add, dispatch, 接单决策, 接单门示例闭包调用, 测试覆盖查询, 触碰红线, 评估接单,
};
use crate::类型_定义_殿::{巡世候选, 本质档位};
use shihai_fu::{依赖图, 测试覆盖判定, 符号档案};
use std::path::PathBuf;

// 造候选 辅助函数：被所有 #[test] 调用，clippy 误报 dead_code。
#[allow(dead_code)]
fn 造候选(目标: &str, 依据: &str, 档: 本质档位) -> 巡世候选 {
    巡世候选 {
        目标: 目标.to_string(),
        依据: 依据.to_string(),
        建议类别: crate::类型_定义_殿::要求类别::补基础,
        优先级: crate::类型_定义_殿::优先级::中,
        本质类别: crate::类型_定义_殿::本质类别::覆盖率不足,
        本质档位: 档,
    }
}

#[test]
fn 维度1_s0候选_候选池非空_仍接受() {
    // 依据「评估接单」第 1 维：S0 抢占 —— 候选池非空也接受。
    // 设计意图：S0 是崩溃/数据损坏/死锁等紧急档位，必须立即处理。
    let 候选 = 造候选("崩溃", "项目起不来", 本质档位::S0);
    let 涉及: Vec<PathBuf> = Vec::new();
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        false,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        None::<fn(&str) -> bool>,
    );
    assert!(
        matches!(决策, 接单决策::接受),
        "S0 候选 + 候选池非空应被接受，但被拒：{:?}",
        决策
    );
}

#[test]
fn 维度1_非s0候选_候选池空_接受() {
    // 依据：候选池空 → 润色注入器可能产出；S0~S5（补救类）紧急，池空+无测试也接受。
    let 候选 = 造候选("崩溃修复", "修复崩溃", 本质档位::S5); // S5=补救类,不要求测试
    let 涉及: Vec<PathBuf> = Vec::new();
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        true,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        None::<fn(&str) -> bool>,
    );
    assert!(
        matches!(决策, 接单决策::接受),
        "S5 候选 + 候选池空应被接受，但被拒：{:?}",
        决策
    );
}

#[test]
fn 维度1_非s0候选_候选池非空_接受() {
    // 关键：档位优先已选过 → 接受（不是「池非空就拒」）
    // 这是 272cb6c 修复后的逻辑：S0 抢占通过；其他档位假设已被档位优先选出 → 通过。
    let 候选 = 造候选("补测试", "为某园补测试", 本质档位::S11);
    let 涉及: Vec<PathBuf> = Vec::new();
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        false,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        None::<fn(&str) -> bool>,
    );
    assert!(
        matches!(决策, 接单决策::接受),
        "S11 候选 + 候选池非空应被接受（档位优先已选过），但被拒：{:?}",
        决策
    );
}

#[test]
fn 维度5_无测试覆盖_档位低_拒绝() {
    // 依据：依据不含「测试」/「验证」 + 档位 ≤ S6 → 拒绝。
    let 候选 = 造候选("性能优化", "优化某处", 本质档位::S8);
    let 涉及: Vec<PathBuf> = Vec::new();
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        true,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        None::<fn(&str) -> bool>,
    );
    assert!(
        matches!(决策, 接单决策::拒绝(_)),
        "S8 候选 + 无测试覆盖应被拒，但接受：{:?}",
        决策
    );
    if let 接单决策::拒绝(原因) = 决策 {
        assert!(
            原因.contains("test") || 原因.contains("#[test]"),
            "拒绝原因应含 'test' 或 '#[test]'，实为：{}",
            原因
        );
    }
}

#[test]
fn 维度5_有测试覆盖_档位低_接受() {
    // 依据：依据含「测试」字眼 → 接受（弱信号匹配）。
    let 候选 = 造候选("性能优化", "某处已加测试覆盖", 本质档位::S8);
    let 涉及: Vec<PathBuf> = Vec::new();
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        true,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        None::<fn(&str) -> bool>,
    );
    assert!(
        matches!(决策, 接单决策::接受),
        "S8 候选 + 依据含'测试'应被接受，但被拒：{:?}",
        决策
    );
}

#[test]
fn 红线1_路径含git_拒绝() {
    // 依据：.git/ 下任何东西都拒绝（红线 1）。
    let 候选 = 造候选("提交", "改 commit", 本质档位::S0); // S0 也挡不住红线
    let 涉及 = vec![PathBuf::from("D:\\洪荒 - 世界\\.git\\config")];
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        true,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        None::<fn(&str) -> bool>,
    );
    assert!(
        matches!(决策, 接单决策::拒绝(_)),
        ".git/ 路径应被红线拒，但接受：{:?}",
        决策
    );
}

#[test]
fn 红线2_md设计稿_拒绝() {
    // 依据：.md 文件都被拒（除非当前架构现状.md）。
    let 候选 = 造候选("改设计稿", "修 AGENTS.md", 本质档位::S0);
    let 涉及 = vec![PathBuf::from("D:\\洪荒 - 世界\\AGENTS.md")];
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        true,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        None::<fn(&str) -> bool>,
    );
    assert!(
        matches!(决策, 接单决策::拒绝(_)),
        "AGENTS.md 应被红线拒，但接受：{:?}",
        决策
    );
}

#[test]
fn 红线5_env文件_拒绝() {
    let 候选 = 造候选("改 env", "更新 API key", 本质档位::S0);
    let 涉及 = vec![PathBuf::from("D:\\洪荒 - 世界\\.env")];
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        true,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        None::<fn(&str) -> bool>,
    );
    assert!(
        matches!(决策, 接单决策::拒绝(_)),
        ".env 应被红线拒，但接受：{:?}",
        决策
    );
}

#[test]
fn 触碰红线_md排除_当前架构现状() {
    // 唯一例外：当前架构现状.md 不被红线拦。
    let 涉及 = vec![PathBuf::from("D:\\洪荒 - 世界\\当前架构现状.md")];
    let 工作区 = std::env::temp_dir();
    assert!(!触碰红线(
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区
    ));
}

#[test]
fn 触碰红线_md非当前架构现状_触发() {
    let 涉及 = vec![PathBuf::from("D:\\洪荒 - 世界\\多智能体架构设计.md")];
    let 工作区 = std::env::temp_dir();
    assert!(触碰红线(
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区
    ));
}

#[test]
fn 触碰红线_cargo_toml_触发() {
    let 涉及 = vec![PathBuf::from("D:\\洪荒 - 世界\\Cargo.toml")];
    let 工作区 = std::env::temp_dir();
    assert!(触碰红线(
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区
    ));
}

#[test]
fn 维度5_闭包返回true_档位低_接受() {
    // 依据：测试覆盖查询 Some(闭包) + 闭包返回 true → 接单门接受。
    // §19.4.1/§8.5/§19.4.2.2：4 层瀑布判定，闭包封装路径/依赖图/证道域/LLM 查询。
    let 候选 = 造候选("某改进", "补某覆盖", 本质档位::S8);
    let 涉及: Vec<PathBuf> = vec![PathBuf::from("D:\\洪荒 - 世界\\some_file.rs")];
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        false,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        Some(|_路径: &str| true),
    );
    assert!(
        matches!(决策, 接单决策::接受),
        "闭包返回 true + 档位 S8 应被接受，但被拒：{:?}",
        决策
    );
}

#[test]
fn 维度5_闭包返回false_档位低_拒() {
    // 依据：测试覆盖查询 Some(闭包) + 闭包返回 false + 档位 ≥ S6 → 拒绝。
    // S0~S5 补救类即使无测试也接受（接单门闭包不拒），档位 ≥ S6 才严格要求。
    let 候选 = 造候选("某优化", "优化某处", 本质档位::S8);
    let 涉及: Vec<PathBuf> = vec![PathBuf::from("D:\\洪荒 - 世界\\some_file.rs")];
    let 工作区 = std::env::temp_dir();
    let 决策 = 评估接单(
        &候选,
        false,
        &涉及.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
        &工作区,
        Some(|_路径: &str| false),
    );
    assert!(
        matches!(决策, 接单决策::拒绝(_)),
        "闭包返回 false + 档位 S8 应被拒，但接受：{:?}",
        决策
    );
}

#[test]
fn add_1_plus_1_eq_2() {
    // 1+1=2 的字面量锚点：直接调用 + 通过调度门闭包调用，两路共验。
    assert_eq!(add(1, 1), 2, "add(1,1) 应等于 2");
    assert_eq!(接单门示例闭包调用(), 2, "接单闭包调用 add(1,1) 应等于 2");
}

/// 调度门 dispatch 入口内闭包调用 add(1,1) → 结果 == 2。
///
/// 任务依据（要求-70）：验收标准②「调度门 dispatch 入口内闭包调用 add(1,1) 可被 git diff 锚定」。
/// - dispatch 是调度门公开入口，签名 `dispatch(命令: &str) -> i64`；
/// - 函数体内定义闭包 `接单闭包`，捕获 `add`；
/// - 闭包调用 `add(1, 1)` 字面量在源码中显式出现。
#[test]
fn dispatch_入口闭包调用_add_1_1_等于_2() {
    let 结果 = dispatch("接单");
    assert_eq!(结果, 2, "dispatch 闭包调用 add(1,1) 应等于 2");
}

// ─────────── §19.4.2.7 P0-1 + P0-2：测试覆盖查询 4 层瀑布回归保护（2026-08-25）───────────
//
// 6 个用例覆盖：
//   层 1：路径前缀命中（零 IO）
//   层 2：依赖图缓存字段读取（零额外 IO）
//   层 4：反向匹配（基名出现在依赖图档案的证道域路径）
//   兜底：模糊情况保守接受（让 §19.8 名单消费阶段调 LLM 兜底）
//
// 依据：上下文.md §8.5 + 多智能体 §19.4.1 + §19.4.2.2 + §19.7 + §6.4 + AGENTS §16。

/// 辅助：构造一个测试档案，含指定 测试覆盖判定 字段值。
#[allow(dead_code)]
fn 造测试档案(文件: &str, 符号: &str, 判定: 测试覆盖判定) -> 符号档案 {
    let mut 档案 = 符号档案::新(
        "世界",
        "测试crate",
        文件,
        符号,
        "fn 测试体() {}",
        "fn 测试体()",
        "",
    );
    档案.测试覆盖判定 = 判定;
    档案
}

#[test]
fn 测试覆盖查询_层1_路径前缀命中_返true() {
    // 路径前缀命中 `证道/` → 是_测试文件 返 true → 层 1 直接过。
    let 图 = 依赖图::default();
    assert!(
        测试覆盖查询(&图, "证道/某府/某园/某测试.rs"),
        "路径前缀命中证道域应被判定为有覆盖"
    );
    assert!(
        测试覆盖查询(&图, "鸿蒙/某府/某园/某业务测试.rs"),
        "文件名含『测试』也应被判定为有覆盖（路径前缀模式：*测试*.rs）"
    );
}

#[test]
fn 测试覆盖查询_层2_缓存字段是测试_返true() {
    // 依赖图缓存：档案.测试覆盖判定 = 是测试 → 返 true（明确有覆盖）。
    let mut 图 = 依赖图::default();
    图.档案们.push(造测试档案(
        "鸿蒙/某府/某园/某业务.rs",
        "某业务_fn",
        测试覆盖判定::是测试,
    ));
    assert!(
        测试覆盖查询(&图, "鸿蒙/某府/某园/某业务.rs"),
        "缓存字段=是测试 应返 true"
    );
}

#[test]
fn 测试覆盖查询_层2_缓存字段不是测试_返false() {
    // 缓存字段 = 不是测试 → 返 false（明确无覆盖，触发接单门拒）。
    // §19.4.2.7 P0-2 关键回归：缓存字段接单门真读，不是「写入但没人读」。
    let mut 图 = 依赖图::default();
    图.档案们.push(造测试档案(
        "鸿蒙/某府/某园/某业务.rs",
        "某业务_fn",
        测试覆盖判定::不是测试,
    ));
    assert!(
        !测试覆盖查询(&图, "鸿蒙/某府/某园/某业务.rs"),
        "缓存字段=不是测试 应返 false（接单门据此拒绝）"
    );
}

#[test]
fn 测试覆盖查询_层2_缓存字段边缘_返true() {
    // 缓存字段 = 边缘（LLM 判定模糊）→ 保守接受 true，让批量阶段决断。
    let mut 图 = 依赖图::default();
    图.档案们.push(造测试档案(
        "鸿蒙/某府/某园/某业务.rs",
        "某业务_fn",
        测试覆盖判定::边缘,
    ));
    assert!(
        测试覆盖查询(&图, "鸿蒙/某府/某园/某业务.rs"),
        "缓存字段=边缘 应保守接受 true"
    );
}

#[test]
fn 测试覆盖查询_层4_反向匹配命中_返true() {
    // 缓存字段 = 未判定 → 走层 4 反向匹配：
    // 涉及文件基名 `某业务` 出现在依赖图档案的证道域路径 → 命中 → 返 true。
    let mut 图 = 依赖图::default();
    图.档案们.push(造测试档案(
        "证道/鸿蒙 - 域/某府/某业务测试.rs",
        "某业务_fn_test",
        测试覆盖判定::未判定,
    ));
    assert!(
        测试覆盖查询(&图, "鸿蒙/某府/某园/某业务.rs"),
        "基名『某业务』出现在证道域档案应命中层 4"
    );
}

#[test]
fn 测试覆盖查询_兜底_模糊情况_保守接受() {
    // 缓存字段 = 未判定 + 层 4 反向未命中 → 保守接受 true（让 §19.8 名单消费阶段调 LLM 兜底）。
    // 依据 §6.4「热路径不调 LLM」+ §19.7「LLM 调用不在接单门评估时」+ §19.4.2.2 闭包实现指南。
    let 图 = 依赖图::default();
    assert!(
        测试覆盖查询(&图, "鸿蒙/某府/某园/某业务.rs"),
        "模糊情况应保守接受（不误拒真实有覆盖的候选）"
    );
}
