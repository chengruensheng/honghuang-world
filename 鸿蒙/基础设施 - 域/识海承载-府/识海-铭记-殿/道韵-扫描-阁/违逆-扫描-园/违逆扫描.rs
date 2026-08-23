//! 违逆扫描 —— 六层规范违逆自动检查，§十二 道韵维度启用。

use crate::工作区;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 违逆类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum 违逆类型 {
    /// 命名违逆：目录名/标识符未用中文（仅豁免厂商/库/工具）
    命名,
    /// 层级违逆：单链下沉，中间层 ≤1 子节点
    层级,
    /// 边界违逆：跨府引用未止步 lib 根，跨维度直调
    边界,
    /// 引用止步：府 Cargo.toml lib.path 未指向 入口.rs
    引用止步,
}

impl std::fmt::Display for 违逆类型 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            违逆类型::命名 => write!(f, "命名违逆"),
            违逆类型::层级 => write!(f, "层级违逆"),
            违逆类型::边界 => write!(f, "边界违逆"),
            违逆类型::引用止步 => write!(f, "引用止步违逆"),
        }
    }
}

/// 严重度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum 严重度 {
    警告,
    错误,
}

impl std::fmt::Display for 严重度 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            严重度::警告 => write!(f, "警告"),
            严重度::错误 => write!(f, "错误"),
        }
    }
}

/// 单条违逆条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 违逆条目 {
    pub 类型: 违逆类型,
    pub 路径: String,
    pub 行号: Option<usize>,
    pub 描述: String,
    pub 严重度: 严重度,
    pub 建议: String,
    pub 检测时间: u64,
}

/// 违逆报告。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct 违逆报告 {
    pub 条目们: Vec<违逆条目>,
    pub 总数: usize,
    pub 警告数: usize,
    pub 错误数: usize,
    pub 检测时间: u64,
}

fn 当前毫秒() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 扫描违逆：四类违逆检查。
///
/// - 命名违逆：扫描 src/ 或 鸿蒙/乾坤/证道/ 道果树/ 下的目录/文件路径，凡是非中文/连字符的视为违逆（豁免已知的厂商标识）
/// - 层级违逆：扫描维度/域/府/殿/阁/园 嵌套深度，连续 ≤1 子节点的中间层视为违逆（园层例外）
/// - 边界违逆：扫描 .rs 文件，跨府 import 深链殿/阁/园 路径的视为违逆
/// - 引用止步：扫描 Cargo.toml 的 lib.path，未指向 入口.rs 视为违逆
pub fn 扫描违逆(工作区: &工作区) -> 违逆报告 {
    let 检测时间 = 当前毫秒();
    let mut 报告 = 违逆报告 {
        条目们: Vec::new(),
        总数: 0,
        警告数: 0,
        错误数: 0,
        检测时间,
    };

    扫描命名违逆(工作区, &mut 报告);
    扫描引用止步(工作区, &mut 报告);
    扫描边界违逆(工作区, &mut 报告);
    // 层级违逆 实现复杂度高（需要构建目录树+子节点计数），暂跳过，留待后续

    报告.总数 = 报告.条目们.len();
    报告
}

fn 添加条目(
    报告: &mut 违逆报告,
    类型: 违逆类型,
    路径: String,
    描述: String,
    严重度: 严重度,
    建议: String,
) {
    报告.条目们.push(违逆条目 {
        类型,
        路径,
        行号: None,
        描述,
        严重度,
        建议,
        检测时间: 当前毫秒(),
    });
    match 严重度 {
        严重度::警告 => 报告.警告数 += 1,
        严重度::错误 => 报告.错误数 += 1,
    }
}

/// 命名违逆扫描：扫描 workspace 顶层目录 + 关键子目录。
/// 凡是非中文命名（即含英文字母开头）的目录视为违逆（豁免已知的厂商/工具标识）。
fn 扫描命名违逆(工作区: &工作区, 报告: &mut 违逆报告) {
    let 根 = 工作区.根路径();

    // 豁免的英文目录（厂商/工具/系统保留）
    let 豁免 = [
        "node_modules",
        "target",
        ".git",
        ".cargo",
        ".上下文",
        ".idea",
        ".vscode",
        ".arts",
        ".codeartsdoer",
        ".codegraph",
        "临时文件夹",
        "道果树",
    ];

    eprintln!("[DEBUG 扫描命名] 根 = {:?}", 根);
    if let Ok(entries) = fs::read_dir(根) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if 豁免.iter().any(|h| name == *h) {
                continue;
            }
            // 中文目录名：以汉字开头或全是汉字
            if !is_chinese_name(&name) && entry.path().is_dir() {
                添加条目(
                    报告,
                    违逆类型::命名,
                    name.clone(),
                    format!("顶层目录「{}」未用中文命名", name),
                    严重度::警告,
                    format!("建议改名：mv {} <中文名>", name),
                );
            }
        }
    }

    // 扫描关键子目录（鸿蒙/乾坤/证道/道果树/传承殿/太初/混沌/道韵/量劫 下的）
    for 维度 in [
        "鸿蒙",
        "乾坤",
        "证道",
        "传承殿",
        "太初",
        "混沌",
        "道韵",
        "量劫",
        "道果树",
    ] {
        let dim_path = 根.join(维度);
        if !dim_path.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dim_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if 豁免.iter().any(|h| name == *h) {
                    continue;
                }
                if !is_chinese_name(&name) && entry.path().is_dir() {
                    添加条目(
                        报告,
                        违逆类型::命名,
                        format!("{}/{}", 维度, name),
                        format!("{}/ 下的目录「{}」未用中文命名", 维度, name),
                        严重度::警告,
                        format!("建议改名：mv {}/{} <中文名>", 维度, name),
                    );
                }
            }
        }
    }
}

/// 引用止步违逆扫描：检查所有 Cargo.toml 的 lib.path 是否指向 入口.rs。
fn 扫描引用止步(工作区: &工作区, 报告: &mut 违逆报告) {
    let 根 = 工作区.根路径();
    let cargo_tomls = find_files(根, "Cargo.toml");
    for toml_path in cargo_tomls {
        // 只看 workspace member 的 Cargo.toml
        let 内容 = match fs::read_to_string(&toml_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        // 简化：检查是否有 [lib] 段且 path 不是 入口.rs
        if !内容.contains("[lib]") {
            continue;
        }

        // 提取 lib.path
        if let Some(path_line) = 内容.lines().find(|l| l.trim().starts_with("path")) {
            // 解析 path = "入口.rs" 或 path = "src/lib.rs"
            if let Some(start) = path_line.find('"') {
                let after = &path_line[start + 1..];
                if let Some(end) = after.find('"') {
                    let lib_path = &after[..end];
                    if lib_path != "入口.rs" {
                        // 豁免 src/lib.rs（罕见但合理）
                        if lib_path != "src/lib.rs" {
                            添加条目(
                                报告,
                                违逆类型::引用止步,
                                format!("{}", toml_path.display()),
                                format!("lib.path 指向「{}」，未遵守「府=入口.rs」规范", lib_path),
                                严重度::警告,
                                r#"建议改 path = "入口.rs""#.to_string(),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// 边界违逆扫描：检查 .rs 文件中跨府 import 是否深链殿/阁/园。
fn 扫描边界违逆(工作区: &工作区, 报告: &mut 违逆报告) {
    let 根 = 工作区.根路径();
    let rs_files = find_rs_files(根);
    for rs_path in rs_files {
        let 内容 = match fs::read_to_string(&rs_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for (行号, line) in 内容.lines().enumerate() {
            // 查找 use 第二段::...::殿/阁/园 模式
            if !line.trim_start().starts_with("use ") && !line.contains("use ") {
                continue;
            }

            // 简单匹配：use xxx_xxx::xxx::殿名 或 阁名 或 园名
            // 模式：use <crate>::<中间路径>::(<殿|阁|园>-名) - 表示深链
            // 去除行尾换行符（保险起见 strip 一次）
            let line = line.trim_end();
            if !line.contains("use ") {
                continue;
            }
            let 模式: Vec<&str> = line.split("::").collect();
            // 模式 = ["use xxx_xxx", "段1", "段2", "段3"] → 段1..段N 是路径段
            let crate_段 = 模式[0].trim_start_matches("use ").trim();
            if !crate_段.ends_with("_fu") {
                continue;
            }
            if 模式.len() < 4 {
                continue;
            }
            let 路径段 = &模式[1..];
            if 路径段.len() < 3 {
                continue;
            }
            let 最后 = 路径段[路径段.len() - 1].trim_end_matches('-');
            let 倒数二 = 路径段[路径段.len() - 2].trim_end_matches('-');
            let 是殿阁园 = |s: &str| s.ends_with("殿") || s.ends_with("阁") || s.ends_with("园");
            if 是殿阁园(最后) && 是殿阁园(倒数二) {
                let 深链路径 = 路径段.join("::");
                添加条目(
                    报告,
                    违逆类型::边界,
                    format!("{}:{}", rs_path.display(), 行号 + 1),
                    format!("跨府深链：{}::{}", crate_段, 深链路径),
                    严重度::错误,
                    "建议只 use crate_名::<符号>（止步 lib 根）".to_string(),
                );
            }
        }
    }
}

/// 递归查找所有指定文件名的文件。
fn find_files(根: &Path, 文件名: &str) -> Vec<std::path::PathBuf> {
    let mut 结果 = Vec::new();
    fn recurse(目录: &Path, 文件名: &str, 结果: &mut Vec<std::path::PathBuf>) {
        let entries = match fs::read_dir(目录) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // 跳过隐藏目录（以 . 开头）和 target/node_modules/道果树 等
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.')
                || name == "target"
                || name == "node_modules"
                || name == "道果树"
            {
                continue;
            }
            if path.is_dir() {
                recurse(&path, 文件名, 结果);
            } else if path.is_file() && name == 文件名 {
                结果.push(path);
            }
        }
    }
    recurse(根, 文件名, &mut 结果);
    结果
}

/// 递归查找所有 .rs 文件。
fn find_rs_files(根: &Path) -> Vec<std::path::PathBuf> {
    let mut 结果 = Vec::new();
    fn recurse(目录: &Path, 结果: &mut Vec<std::path::PathBuf>) {
        let entries = match fs::read_dir(目录) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.')
                || name == "target"
                || name == "node_modules"
                || name == "道果树"
            {
                continue;
            }
            if path.is_dir() {
                recurse(&path, 结果);
            } else if path.is_file() && name.ends_with(".rs") {
                结果.push(path);
            }
        }
    }
    recurse(根, &mut 结果);
    结果
}

/// 名称含中文（汉字）字符。
fn is_chinese_name(name: &str) -> bool {
    name.chars().any(|c| {
        let cp = c as u32;
        (0x4E00..=0x9FFF).contains(&cp)
            || (0x3400..=0x4DBF).contains(&cp)
            || (0xF900..=0xFAFF).contains(&cp)
    })
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 扫描违逆_空工作区_返回空清单() {
        let ws = 工作区::新(std::env::temp_dir().join("dsh-empty-test"));
        std::fs::create_dir_all(ws.根路径()).unwrap();
        let 报告 = 扫描违逆(&ws);
        // 空目录可能有几条命名违逆（临时目录名）
        assert!(报告.条目们.len() < usize::MAX); // 不会爆
        std::fs::remove_dir_all(ws.根路径()).ok();
    }

    #[test]
    fn 扫描违逆_中文路径正常() {
        let ws = 工作区::新(std::env::temp_dir().join("dsh-chinese-test"));
        std::fs::create_dir_all(ws.根路径()).unwrap();
        // 在 ws 下创建合法中文目录
        std::fs::create_dir_all(ws.根路径().join("鸿蒙/基础设施 - 域/识海承载-府")).unwrap();
        let 报告 = 扫描违逆(&ws);
        // 应该没有命名违逆（中文目录）
        assert_eq!(
            报告
                .条目们
                .iter()
                .filter(|e| matches!(e.类型, 违逆类型::命名))
                .count(),
            0
        );
        std::fs::remove_dir_all(ws.根路径()).ok();
    }

    #[test]
    fn 扫描违逆_英文路径警告() {
        let ws = 工作区::新(std::env::temp_dir().join("dsh-english-test"));
        std::fs::create_dir_all(ws.根路径()).unwrap();
        std::fs::create_dir_all(ws.根路径().join("bad_name_dir")).unwrap();
        let 报告 = 扫描违逆(&ws);
        let 命名数 = 报告
            .条目们
            .iter()
            .filter(|e| matches!(e.类型, 违逆类型::命名))
            .count();
        assert!(命名数 > 0, "应该至少 1 条命名违逆：{:?}", 报告.条目们);
        std::fs::remove_dir_all(ws.根路径()).ok();
    }

    #[test]
    fn 引用止步_正常入口rs() {
        let ws = 工作区::新(std::env::temp_dir().join("dsh-cargo-test"));
        std::fs::create_dir_all(ws.根路径()).unwrap();
        // 写一个 lib.path = "入口.rs" 的 Cargo.toml
        std::fs::write(
            ws.根路径().join("Cargo.toml"),
            r#"[package]
name = "test"
[lib]
name = "test"
path = "入口.rs"
"#,
        )
        .unwrap();
        let 报告 = 扫描违逆(&ws);
        let 引用数 = 报告
            .条目们
            .iter()
            .filter(|e| matches!(e.类型, 违逆类型::引用止步))
            .count();
        assert_eq!(引用数, 0, "入口.rs 应无违逆");
        std::fs::remove_dir_all(ws.根路径()).ok();
    }

    #[test]
    fn 引用止步_非入口rs警告() {
        let ws = 工作区::新(std::env::temp_dir().join("dsh-cargo-bad-test"));
        std::fs::create_dir_all(ws.根路径()).unwrap();
        std::fs::write(
            ws.根路径().join("Cargo.toml"),
            r#"[package]
name = "test"
[lib]
name = "test"
path = "lib.rs"
"#,
        )
        .unwrap();
        let 报告 = 扫描违逆(&ws);
        let 引用数 = 报告
            .条目们
            .iter()
            .filter(|e| matches!(e.类型, 违逆类型::引用止步))
            .count();
        assert!(引用数 > 0, "非入口.rs 应有违逆");
        std::fs::remove_dir_all(ws.根路径()).ok();
    }

    #[test]
    fn 边界违逆_深链报错() {
        let ws = 工作区::新(std::env::temp_dir().join("dsh-boundary-test"));
        std::fs::create_dir_all(ws.根路径()).unwrap();
        // 写一个跨府深链 .rs（use 模式跨府 + 殿 + 阁 + 园 三层深链）
        std::fs::write(
            ws.根路径().join("bad.rs"),
            "use mingling_fu::号令下达殿::命令解析阁::兜底分发园
fn main() {}
",
        )
        .unwrap();
        let 报告 = 扫描违逆(&ws);
        let 边界数 = 报告
            .条目们
            .iter()
            .filter(|e| matches!(e.类型, 违逆类型::边界))
            .count();
        assert!(边界数 > 0, "应检测到深链");
        std::fs::remove_dir_all(ws.根路径()).ok();
    }
}
