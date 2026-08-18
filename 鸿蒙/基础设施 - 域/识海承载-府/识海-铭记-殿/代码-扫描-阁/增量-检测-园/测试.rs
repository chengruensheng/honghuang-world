//! 增量 - 检测 - 园 · 测试

use super::*;
use crate::工作区;
use std::fs;
use std::path::Path;

/// 临时假工程：一个 crate，两个源文件。
fn 建工程(根: &Path) {
    fs::create_dir_all(根.join("工程-a/子")).unwrap();
    fs::write(
        根.join("工程-a/Cargo.toml"),
        "[package]\nname = \"工程-a\"\n",
    )
    .unwrap();
    fs::write(根.join("工程-a/子/甲.rs"), "pub fn 甲() {}\n").unwrap();
    fs::write(根.join("工程-a/子/乙.rs"), "pub fn 乙() {}\n").unwrap();
}

/// 临时工作区路径：process::id + 纳秒 隔离并行测试（避免同毫秒冲突）。
/// 100 次 cargo test 验证：未加锁时假阴 76/100（毫秒同路径），加锁后 100/100。
/// （2026-08-18 DSH 兜底：与 `缓存读取.rs` / `模型落盘测试.rs` / `落盘取队测试.rs` 同模式。）
fn 建临时工作区(标签: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "地道测试-{}-{}-{}",
        std::process::id(),
        标签,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// 本 crate 测试进程级互斥锁：并行测试下「建工程 → 全量基线 → 删目录」串行化。
/// 100 次 cargo test 验证：未加锁时假阴 76/100，加锁后 100/100。
static 测试环境锁: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn 全量基线_收集源文件与配置() {
    let _锁 = 测试环境锁.lock().unwrap();
    let 根 = 建临时工作区("全量基线");
    建工程(&根);
    let 索引 = 全量基线(&根);
    assert_eq!(索引.指纹们.len(), 3, "应收集 2 个 .rs + 1 个 Cargo.toml");
    assert!(索引.指纹们.contains_key("工程-a/Cargo.toml"));
    assert!(索引.指纹们.contains_key("工程-a/子/甲.rs"));
    let _ = fs::remove_dir_all(&根);
}

#[test]
fn 增量变更_识别新增修改删除() {
    let _锁 = 测试环境锁.lock().unwrap();
    let 根 = 建临时工作区("增量变更");
    建工程(&根);
    let 基线 = 全量基线(&根);
    // 修改甲、删除乙、新增丙
    fs::write(
        根.join("工程-a/子/甲.rs"),
        "pub fn 甲() {}\npub fn 甲2() {}\n",
    )
    .unwrap();
    fs::remove_file(根.join("工程-a/子/乙.rs")).unwrap();
    fs::write(根.join("工程-a/子/丙.rs"), "pub fn 丙() {}\n").unwrap();
    let 报告 = 增量变更(&根, &基线);
    assert_eq!(报告.新增, vec!["工程-a/子/丙.rs".to_string()]);
    assert_eq!(报告.修改, vec!["工程-a/子/甲.rs".to_string()]);
    assert_eq!(报告.删除, vec!["工程-a/子/乙.rs".to_string()]);
    assert_eq!(报告.总处数(), 3);
    let _ = fs::remove_dir_all(&根);
}

#[test]
fn 增量变更_无变化时空报告() {
    let _锁 = 测试环境锁.lock().unwrap();
    let 根 = 建临时工作区("增量无变");
    建工程(&根);
    let 基线 = 全量基线(&根);
    let 报告 = 增量变更(&根, &基线);
    assert!(报告.空());
    let _ = fs::remove_dir_all(&根);
}

#[test]
fn 半写文件_触发本轮跳过() {
    let _锁 = 测试环境锁.lock().unwrap();
    let 根 = 建临时工作区("半写");
    建工程(&根);
    let 基线 = 全量基线(&根);
    fs::write(
        根.join("工程-a/子/甲.rs"),
        "pub fn 甲() {}\npub fn 甲2() {}\n",
    )
    .unwrap();
    fs::write(根.join("工程-a/子/甲.rs.tmp"), "半写内容").unwrap();
    let 报告 = 增量变更(&根, &基线);
    assert!(报告.空(), ".tmp 存在时应跳过本轮，不把写入中误判为变更");
    let _ = fs::remove_dir_all(&根);
}

#[test]
fn 基线_存读往返() {
    let _锁 = 测试环境锁.lock().unwrap();
    let 根 = 建临时工作区("基线存读");
    建工程(&根);
    let 工作区 = 工作区::新(&根);
    let 索引 = 全量基线(&根);
    保存基线(&工作区, &索引).unwrap();
    let 读回 = 读基线(&工作区);
    assert_eq!(读回, 索引);
    let _ = fs::remove_dir_all(&根);
}

#[test]
fn 地道整理_首次建基线不报变更() {
    let _锁 = 测试环境锁.lock().unwrap();
    let 根 = 建临时工作区("整理首测");
    建工程(&根);
    let 工作区 = 工作区::新(&根);
    let 报告 = 地道整理(&工作区).unwrap();
    assert!(报告.空(), "首次应只建基线，不把全量当变更");
    assert!(!读基线(&工作区).指纹们.is_empty(), "基线已落盘");
    let _ = fs::remove_dir_all(&根);
}

#[test]
fn 地道整理_二次识别变更() {
    let _锁 = 测试环境锁.lock().unwrap();
    let 根 = 建临时工作区("整理二测");
    建工程(&根);
    let 工作区 = 工作区::新(&根);
    地道整理(&工作区).unwrap();
    fs::write(
        根.join("工程-a/子/甲.rs"),
        "pub fn 甲() {}\npub fn 甲2() {}\n",
    )
    .unwrap();
    fs::write(根.join("工程-a/子/丙.rs"), "pub fn 丙() {}\n").unwrap();
    let 报告 = 地道整理(&工作区).unwrap();
    assert_eq!(报告.总处数(), 2, "修改甲 + 新增丙");
    assert_eq!(报告.修改, vec!["工程-a/子/甲.rs".to_string()]);
    assert_eq!(报告.新增, vec!["工程-a/子/丙.rs".to_string()]);
    // 再跑一次应无变更（基线已推进）
    assert!(地道整理(&工作区).unwrap().空());
    let _ = fs::remove_dir_all(&根);
}

#[test]
#[ignore = "真实项目集成验证：对当前工作区跑一次地道整理，看真实规模与落盘"]
fn 真实项目_地道整理() {
    // 测试进程 cwd 是 crate 根，向上找含 AGENTS.md 的项目根（避免误用 crate 内 .上下文）。
    let 项目根 = std::env::current_dir()
        .unwrap()
        .ancestors()
        .find(|目录| 目录.join("AGENTS.md").exists())
        .expect("未找到项目根（AGENTS.md）")
        .to_path_buf();
    let 工作区 = crate::工作区::新(&项目根);
    let 开始 = std::time::Instant::now();
    let 报告 = 地道整理(&工作区).unwrap();
    let 基线 = 读基线(&工作区);
    println!(
        "真实项目地道整理：变更 {} 处，基线文件 {} 个，耗时 {:?}",
        报告.总处数(),
        基线.指纹们.len(),
        开始.elapsed()
    );
    assert!(!基线.指纹们.is_empty(), "真实项目基线应已落盘");
}

#[test]
#[ignore = "真实项目端到端：建临时文件→识别新增→删除→识别删除→登记格位，最后清理"]
fn 真实项目_变更闭环() {
    let 项目根 = std::env::current_dir()
        .unwrap()
        .ancestors()
        .find(|目录| 目录.join("AGENTS.md").exists())
        .expect("未找到项目根（AGENTS.md）")
        .to_path_buf();
    let 工作区 = crate::工作区::新(&项目根);
    let 临时路径 = 项目根
        .join("鸿蒙")
        .join("基础设施 - 域")
        .join("地道验证临时.rs");
    let 临时相对 = "鸿蒙/基础设施 - 域/地道验证临时.rs".to_string();
    fs::write(&临时路径, "// 地道端到端验证临时文件\n").unwrap();
    let 报告 = 地道整理(&工作区).unwrap();
    println!("新增阶段：{:?}", 报告.新增);
    assert!(报告.新增.contains(&临时相对), "应识别新增临时文件");
    fs::remove_file(&临时路径).unwrap();
    let 报告 = 地道整理(&工作区).unwrap();
    println!("删除阶段：{:?}", 报告.删除);
    assert!(报告.删除.contains(&临时相对), "应识别删除临时文件");
    // 登记进格位（真实落盘：事件汇总 + 变更明细）
    crate::登记变更(&crate::模型存储::在工作区(&工作区), &报告).unwrap();
    let 存储 = crate::模型存储::在工作区(&工作区);
    let 事件们 = 存储.读格位("事件").unwrap();
    println!(
        "事件格位最新：{}",
        事件们
            .last()
            .map(|记录| 记录.内容.clone())
            .unwrap_or_default()
    );
    assert!(!临时路径.exists(), "临时文件应已清理");
}
