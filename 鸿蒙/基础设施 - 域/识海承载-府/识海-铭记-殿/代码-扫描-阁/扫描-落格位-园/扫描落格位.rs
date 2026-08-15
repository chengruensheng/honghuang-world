//! 扫描 - 落格位 - 园：扫描代码，落到事实记录（文件 / 结构 / 环境·依赖 / 调用 / 数据 / 事件）。

use crate::{坐标, 坐标层, 记录, 模型存储, 扫描排除项};
use std::path::{Path, PathBuf};

/// 扫描文件清单 → 「文件」格位（.rs 源文件），返回条数。
pub fn 扫描文件清单(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let 文件们 = 收集源文件(根目录, &排除项);
    for 文件 in &文件们 {
        let 相对 = 文件.display().to_string();
        let mut 记录 = 记录::新("文件", &相对, &format!("扫描「{相对}」"), "代码");
        记录.坐标 = Some(坐标 { 层: 坐标层::文件, 对象: "源文件".into(), 属性: "清单".into() });
        存储.写记录(&记录)?;
    }
    Ok(文件们.len())
}

/// 扫描目录结构 → 「结构」格位，返回条数。
pub fn 扫描目录结构(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let 目录们 = 收集目录(根目录, &排除项);
    for 目录 in &目录们 {
        let 相对 = 目录.display().to_string();
        let mut 记录 = 记录::新("结构", &相对, &format!("目录「{相对}」"), "代码");
        记录.坐标 = Some(坐标 { 层: 坐标层::模块, 对象: "目录".into(), 属性: "结构".into() });
        存储.写记录(&记录)?;
    }
    Ok(目录们.len())
}

/// 扫描 Cargo.toml 依赖 → 「环境·依赖」格位，返回条数。
pub fn 扫描依赖(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 路径 = 根目录.join("Cargo.toml");
    let 内容 = std::fs::read_to_string(&路径).map_err(|错误| format!("读 Cargo.toml 失败: {错误}"))?;
    let mut 条数 = 0;
    let mut 在依赖段 = false;
    for 行 in 内容.lines() {
        let 行 = 行.trim();
        if 行.starts_with('[') {
            在依赖段 = 行 == "[dependencies]" || 行 == "[dev-dependencies]" || 行 == "[build-dependencies]";
            continue;
        }
        if 在依赖段 && !行.is_empty() && !行.starts_with('#') {
            if let Some(名) = 行.split(|字符: char| 字符 == '=' || 字符 == ' ' || 字符 == '{').next() {
                let 名 = 名.trim();
                if !名.is_empty() {
                    存储.写记录(&记录::新("环境·依赖", 名, &format!("依赖「{名}」"), "代码"))?;
                    条数 += 1;
                }
            }
        }
    }
    Ok(条数)
}

/// 扫描 .rs 函数签名 → 「调用」格位（符号层坐标），返回条数。
pub fn 扫描符号(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let 文件们 = 收集源文件(根目录, &排除项);
    let mut 条数 = 0;
    for 文件 in &文件们 {
        let 内容 = std::fs::read_to_string(文件).map_err(|错误| format!("读源文件失败: {错误}"))?;
        for 行 in 内容.lines() {
            if let Some(函数名) = 提取函数名(行) {
                let 相对 = 文件.display().to_string();
                let mut 记录 = 记录::新("调用", &函数名, &format!("函数「{函数名}」于「{相对}」"), "代码");
                记录.坐标 = Some(坐标 { 层: 坐标层::符号, 对象: "函数".into(), 属性: "签名".into() });
                存储.写记录(&记录)?;
                条数 += 1;
            }
        }
    }
    Ok(条数)
}

/// 扫描数据文件 → 「数据」格位（.json/.jsonl/.csv/.db），返回条数。
pub fn 扫描数据(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let 文件们 = 收集数据文件(根目录, &排除项);
    for 文件 in &文件们 {
        let 相对 = 文件.display().to_string();
        let mut 记录 = 记录::新("数据", &相对, &format!("数据「{相对}」"), "代码");
        记录.坐标 = Some(坐标 { 层: 坐标层::文件, 对象: "数据".into(), 属性: "数据".into() });
        存储.写记录(&记录)?;
    }
    Ok(文件们.len())
}

/// 综合扫描：文件 + 结构 + 依赖 + 符号 + 数据 + 事件，返回总条数。
pub fn 扫描(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let mut 条数 = 0;
    条数 += 扫描文件清单(存储, 根目录)?;
    条数 += 扫描目录结构(存储, 根目录)?;
    条数 += 扫描依赖(存储, 根目录)?;
    条数 += 扫描符号(存储, 根目录)?;
    条数 += 扫描数据(存储, 根目录)?;
    存储.写记录(&记录::新("事件", "代码扫描完成", &format!("扫描 {}", 根目录.display()), "代码"))?;
    Ok(条数 + 1)
}

fn 提取函数名(行: &str) -> Option<String> {
    let 行 = 行.trim_start();
    let 行 = 行.strip_prefix("pub ").unwrap_or(行);
    let 行 = 行.strip_prefix("async ").unwrap_or(行);
    let 行 = 行.strip_prefix("unsafe ").unwrap_or(行);
    let 行 = 行.strip_prefix("fn ")?;
    let 名: String = 行.chars().take_while(|字符| *字符 != '(' && !字符.is_whitespace()).collect();
    if 名.is_empty() { None } else { Some(名) }
}

/// 是否命中排除项。
fn 应排除(名: &str, 排除项: &[String]) -> bool {
    排除项.iter().any(|项| 项 == 名)
}

/// 递归收集 .rs 源文件，跳过排除项。
fn 收集源文件(根目录: &Path, 排除项: &[String]) -> Vec<PathBuf> {
    let mut 结果 = Vec::new();
    递归(根目录, 排除项, &mut 结果);
    结果
}

/// 递归收集目录，跳过排除项。
fn 收集目录(根目录: &Path, 排除项: &[String]) -> Vec<PathBuf> {
    let mut 结果 = Vec::new();
    递归目录(根目录, 排除项, &mut 结果);
    结果
}

fn 递归(目录: &Path, 排除项: &[String], 结果: &mut Vec<PathBuf>) {
    let Ok(条目们) = std::fs::read_dir(目录) else { return };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 应排除(&名, 排除项) {
                continue;
            }
            递归(&路径, 排除项, 结果);
        } else if 名.ends_with(".rs") {
            结果.push(路径);
        }
    }
}

fn 递归目录(目录: &Path, 排除项: &[String], 结果: &mut Vec<PathBuf>) {
    let Ok(条目们) = std::fs::read_dir(目录) else { return };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 应排除(&名, 排除项) {
                continue;
            }
            结果.push(路径.clone());
            递归目录(&路径, 排除项, 结果);
        }
    }
}

/// 递归收集数据文件，跳过排除项。
fn 收集数据文件(根目录: &Path, 排除项: &[String]) -> Vec<PathBuf> {
    let mut 结果 = Vec::new();
    递归数据(根目录, 排除项, &mut 结果);
    结果
}

fn 递归数据(目录: &Path, 排除项: &[String], 结果: &mut Vec<PathBuf>) {
    let Ok(条目们) = std::fs::read_dir(目录) else { return };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 应排除(&名, 排除项) {
                continue;
            }
            递归数据(&路径, 排除项, 结果);
        } else if 是数据文件(&名) {
            结果.push(路径);
        }
    }
}

fn 是数据文件(名: &str) -> bool {
    let 小写 = 名.to_lowercase();
    [".json", ".jsonl", ".csv", ".db"].iter().any(|后缀| 小写.ends_with(后缀))
}
