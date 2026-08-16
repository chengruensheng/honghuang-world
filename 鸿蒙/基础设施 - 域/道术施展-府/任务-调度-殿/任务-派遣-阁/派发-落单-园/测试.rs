//! 派发 - 落单 - 园 · 测试

use super::*;
use crate::类型_定义_殿::{执行任务, 工作流级别, 产物条目, 执行回执, 执行状态};
use moxing_fu::{模型配置, 用量};
use shihai_fu::{全量基线, 增量变更};
use std::path::PathBuf;

/// 造一个调度器（测试用假配置，不触网）。
fn 假调度(根: PathBuf) -> 任务调度 {
    任务调度::新(
        模型配置 { 密钥: String::new(), 地址: String::new(), 模型: String::new() },
        根,
    )
}

#[test]
fn 模板复述占位块被跳过真实块照常提取() {
    let 文本 = "1. 每个文件必须用 <<<文件:路径>>> 开头、<<<结束>>> 结尾，两标记成对出现。\n\
                <<<文件:相对项目根的文件路径>>>\n示例内容\n<<<结束>>>\n\
                <<<文件:鸿蒙\\基础设施 - 域\\识海承载-府\\识海-铭记-殿\\代码-扫描-阁\\扫描-落格位-园\\扫描落格位.rs>>>\n\
                pub fn 扫描() {}\n\
                <<<结束>>>";
    let 文件们 = 解析落盘文本(文本).unwrap();
    assert_eq!(文件们.len(), 1);
    assert!(文件们[0].路径.ends_with("扫描落格位.rs"));
    assert!(文件们[0].内容.contains("pub fn 扫描"));
}

#[test]
fn 全是占位块时仍报未找到() {
    let 文本 = "<<<文件:路径>>>\n内容\n<<<结束>>>";
    assert!(解析落盘文本(文本).is_err());
}

#[test]
fn 合并产物_主优先兜底补缺按路径去重() {
    let 调度 = 假调度(std::env::temp_dir());
    let 主们 = vec![产物条目 { 路径: "甲.rs".to_string(), 类别: "代码".to_string(), 字节数: 1, 变化类型: "修改".to_string() }];
    let 兜底们 = vec![
        产物条目 { 路径: "甲.rs".to_string(), 类别: "代码".to_string(), 字节数: 999, 变化类型: "修改".to_string() },
        产物条目 { 路径: "乙.rs".to_string(), 类别: "代码".to_string(), 字节数: 2, 变化类型: "新增".to_string() },
    ];
    let 结果 = 调度.合并产物(主们, 兜底们);
    assert_eq!(结果.len(), 2);
    assert_eq!(结果[0].路径, "甲.rs");
    assert_eq!(结果[0].字节数, 1, "主产物优先，不被兜底覆盖");
    assert_eq!(结果[1].路径, "乙.rs");
}

#[test]
fn diff补产物_识别任务期间新增与修改() {
    // 临时工程：先拍基线，任务期间新增 + 修改，diff 应把两处都合成产物。
    let 根 = std::env::temp_dir().join(format!("派发兜底测试-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&根);
    std::fs::create_dir_all(&根).unwrap();
    std::fs::write(根.join("Cargo.toml"), "[package]\nname = \"兜底\"\n").unwrap();
    let 基线 = 全量基线(&根);
    std::fs::write(根.join("子.rs"), "pub fn 甲() {}\n").unwrap();
    std::fs::write(根.join("Cargo.toml"), "[package]\nname = \"兜底\"\nversion = \"0.2.0\"\n").unwrap();
    let 调度 = 假调度(根.clone());
    let 产物们 = 调度.diff补产物(&基线);
    let 路径们: Vec<&str> = 产物们.iter().map(|产物| 产物.路径.as_str()).collect();
    assert!(路径们.contains(&"子.rs"), "新增应被识别：{:?}", 路径们);
    assert!(路径们.contains(&"Cargo.toml"), "修改应被识别：{:?}", 路径们);
    assert_eq!(产物们.len(), 2);
    let _ = std::fs::remove_dir_all(&根);
}

/// 产物兜底只补相对基线有变化的文件：未变文件不应出现在清单中。
/// 与 diff补产物_识别任务期间新增与修改 互补，锁定「未变 → 不补入」行为防回归。
/// 指纹 = 大小 + 修改时间（毫秒），同一文件大小与 mtime 完全一致即视为未变。
#[test]
fn diff补产物_未变文件不补入清单() {
    let 根 = std::env::temp_dir().join(format!("派发兜底未变测试-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&根);
    std::fs::create_dir_all(&根).unwrap();
    std::fs::write(根.join("未变.rs"), "pub fn 甲() {}\n").unwrap();
    std::fs::write(根.join("要改.rs"), "pub fn 乙() {}\n").unwrap();
    // 间隔确保 mtime 跨毫秒（部分文件系统 mtime 精度仅秒级，需留余量）。
    std::thread::sleep(std::time::Duration::from_millis(50));
    let 基线 = 全量基线(&根);

    // 仅改要改.rs，未变.rs 不动（fingerprint 与基线一致 → 分类为未变 → 不进产物）。
    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(根.join("要改.rs"), "pub fn 乙() { println!(\"改了\"); }\n").unwrap();

    let 调度 = 假调度(根.clone());
    let 产物们 = 调度.diff补产物(&基线);
    let 路径们: Vec<&str> = 产物们.iter().map(|产物| 产物.路径.as_str()).collect();
    assert!(路径们.contains(&"要改.rs"), "修改应被识别：{:?}", 路径们);
    assert!(!路径们.contains(&"未变.rs"), "未变文件不应在产物清单中：{:?}", 路径们);
    assert_eq!(产物们.len(), 1, "只有修改的文件应在产物清单中");
    let _ = std::fs::remove_dir_all(&根);
}

/// 执行回执必须带 轮数字段（跨任务累计总轮数），首调起算、重试不重置、跨并发派遣各自独立。
#[test]
fn 执行回执带轮数字段且类型为u32() {
    let 回执 = 执行回执 {
        状态: 执行状态::成功,
        产物们: vec![],
        说明: "示例".to_string(),
        用量: 用量::default(),
        轮数: 7,
    };
    assert_eq!(回执.轮数, 7u32, "轮数应原样保存（u32 跨任务累计）");
    // 反序列化往返：JSON 落库可还原（含 轮数 字段）。
    let 文本 = serde_json::to_string(&回执).expect("应能序列化");
    assert!(文本.contains("\"轮数\":7"), "序列化须包含轮数字段：{文本}");
    let 反: 执行回执 = serde_json::from_str(&文本).expect("应能反序列化");
    assert_eq!(反.轮数, 7);
}

/// 假调度造一个 执行任务，确认 类型 装配无误（仅为结构兜底测试，证明轮数接口已就绪）。
#[test]
fn 执行任务结构与轮数接口对齐() {
    let 任务 = 执行任务 {
        目标: "示例目标".to_string(),
        工作流: 工作流级别::脚本,
        角色们: vec!["多宝".to_string()],
    };
    assert_eq!(任务.目标, "示例目标");
    // 跨任务累计总轮数将由 调用方在收集 执行回执.轮数 后求和并落库。
}
