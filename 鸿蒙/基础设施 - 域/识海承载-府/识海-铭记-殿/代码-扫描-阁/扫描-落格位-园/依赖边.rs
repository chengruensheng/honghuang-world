//! 依赖 - 边：扫描 use / pub use 依赖边 → 依赖图。

use crate::世界结果;
use crate::{依赖图, 工作区, 扫描排除项, 符号档案};
use std::collections::HashMap;
use std::path::Path;

use super::收集::{归属crate, 收集源文件, 相对路径};
use super::符号解析::{
    提取use引用, 提取上方注释, 提取定义块, 提取符号签名
};

/// 扫描 use / pub use 依赖边 → 依赖图（符号档案：五层标识 + 解释 + 波及）。
/// 不动格位，产出依赖图交给依赖-边-园落盘。
pub fn 扫描依赖边(根目录: &Path) -> 世界结果<依赖图> {
    let 排除项 = 扫描排除项(根目录);
    let 文件们 = 收集源文件(根目录, &排除项);
    let 项目名 = 根目录
        .file_name()
        .map(|名| 名.to_string_lossy().to_string())
        .unwrap_or_else(|| "世界".to_string());
    // 加载旧图，保留 LLM 补的解释（/// 注释空时回退，避免重扫覆盖语义）
    let 旧图 = 依赖图::加载自工作区(&工作区::新(根目录)).unwrap_or_default();
    let 旧解释: HashMap<String, String> = 旧图
        .档案们
        .iter()
        .filter(|档案| !档案.解释.is_empty())
        .map(|档案| (format!("{}::{}", 档案.文件, 档案.符号), 档案.解释.clone()))
        .collect();
    let mut 图 = 依赖图::default();

    // 第一遍：符号定义 + 解释（波及暂空）。同时记录每文件的符号数，供第二遍补占位档案。
    let mut 文件符号数: HashMap<String, usize> = HashMap::new();
    for 文件 in &文件们 {
        let 内容 = std::fs::read_to_string(文件).map_err(|错误| format!("读源文件失败: {错误}"))?;
        let 相对 = 相对路径(根目录, 文件);
        let 模块 = 归属crate(文件);
        let 行们: Vec<&str> = 内容.lines().collect();
        let mut 本文件符号数 = 0;
        for (序号, 行) in 行们.iter().enumerate() {
            if let Some((名, 签名)) = 提取符号签名(行) {
                本文件符号数 += 1;
                let 解释 = 提取上方注释(&行们, 序号);
                let 解释 = if 解释.is_empty() {
                    旧解释
                        .get(&format!("{}::{}", 相对, 名))
                        .cloned()
                        .unwrap_or_default()
                } else {
                    解释
                };
                // 代码字段 = 完整定义体（M4 配方：执行层按函数级切片读现状，防签名幻觉）
                let 定义体 = 提取定义块(&行们, 序号);
                图.档案们.push(符号档案::新(
                    &项目名, &模块, &相对, &名, &定义体, &签名, &解释,
                ));
            }
        }
        文件符号数.insert(相对, 本文件符号数);
    }

    // 无 pub 符号的文件（如纯测试文件：#[test] fn 不带 pub）补文件级占位档案，
    // 保证依赖图覆盖全部源文件——否则涉及路径按文件查必然落空，回退兜底接线文件导致验收误判。
    for 文件 in &文件们 {
        let 相对 = 相对路径(根目录, 文件);
        if 文件符号数.get(&相对).copied().unwrap_or(0) != 0 {
            continue;
        }
        let 内容 = std::fs::read_to_string(文件).map_err(|错误| format!("读源文件失败: {错误}"))?;
        let 名 = 文件
            .file_stem()
            .map(|茎| 茎.to_string_lossy().to_string())
            .unwrap_or_default();
        let 解释 = 内容
            .lines()
            .find(|行| 行.trim_start().starts_with("//!"))
            .unwrap_or("")
            .trim()
            .trim_start_matches("//!")
            .trim()
            .to_string();
        图.档案们.push(符号档案::新(
            &项目名,
            &归属crate(文件),
            &相对,
            &名,
            &内容,
            &format!("文件 {名}"),
            &解释,
        ));
    }

    // 第二遍：use / pub use 引用，回填波及。
    for 文件 in &文件们 {
        let 内容 = std::fs::read_to_string(文件).map_err(|错误| format!("读源文件失败: {错误}"))?;
        let 相对 = 相对路径(根目录, 文件);
        for 行 in 内容.lines() {
            for 被引用名 in 提取use引用(行) {
                for 档案 in 图.档案们.iter_mut() {
                    if 档案.符号 == 被引用名 && !档案.波及.contains(&相对) {
                        档案.波及.push(相对.clone());
                    }
                }
            }
        }
    }

    // 第三遍：从文件路径构建结构树（crate 为根，根段=完整相对路径），支撑执行层按需下探。
    for 文件 in &文件们 {
        if let Some(段们) = crate内目录段们(根目录, 文件) {
            if !段们.is_empty() {
                图.结构树.插入(&段们);
            }
        }
    }

    Ok(图)
}

/// 从文件路径提取「crate 完整相对路径 + crate 内目录段」（crate 为根），供结构树下探。
/// 根段用相对项目根的完整路径（含域），执行者据此写真实路径，不再脑补父路径。
fn crate内目录段们(根目录: &Path, 文件: &Path) -> Option<Vec<String>> {
    let crate目录 = 找crate目录(文件)?;
    let crate名 = crate目录.file_name()?.to_string_lossy().to_string();
    let 相对crate = 文件.strip_prefix(&crate目录).ok()?;
    let 相对str = 相对crate.to_string_lossy().replace('\\', "/");
    let crate路径 = 相对路径(根目录, &crate目录).replace('\\', "/");
    let 根段 = if crate路径.is_empty() {
        crate名
    } else {
        crate路径
    };
    let mut 段们 = vec![根段];
    段们.extend(目录段们(&相对str));
    Some(段们)
}

/// 从相对路径提取目录段（去掉文件名段），用于构建结构树。
fn 目录段们(相对路径: &str) -> Vec<String> {
    相对路径
        .split('/')
        .filter(|段| !段.is_empty())
        .filter(|段| !是文件名段(段))
        .map(|段| 段.to_string())
        .collect()
}

/// 是否文件名段（含已知后缀）。
fn 是文件名段(段: &str) -> bool {
    [".rs", ".toml", ".json", ".md", ".html", ".css", ".js"]
        .iter()
        .any(|后缀| 段.ends_with(后缀))
}

/// 向上找最近的含 Cargo.toml 的目录（crate 根）。
fn 找crate目录(文件: &Path) -> Option<std::path::PathBuf> {
    let mut 目录 = 文件.parent();
    while let Some(路径) = 目录 {
        if 路径.join("Cargo.toml").exists() {
            return Some(路径.to_path_buf());
        }
        目录 = 路径.parent();
    }
    None
}
