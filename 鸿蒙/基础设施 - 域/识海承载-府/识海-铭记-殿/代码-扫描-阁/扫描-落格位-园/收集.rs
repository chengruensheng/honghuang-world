//! 收集 - 遍历：源文件 / Cargo.toml / 数据文件的递归收集与路径归属。

use std::path::{Path, PathBuf};

/// 是否命中排除项。
pub(crate) fn 应排除(名: &str, 排除项: &[String]) -> bool {
    排除项.iter().any(|项| 项 == 名)
}

/// 递归收集 .rs 源文件，跳过排除项。
pub(crate) fn 收集源文件(根目录: &Path, 排除项: &[String]) -> Vec<PathBuf> {
    let mut 结果 = Vec::new();
    递归(根目录, 排除项, &mut 结果);
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

/// 递归收集 Cargo.toml 文件，跳过排除项。
pub(crate) fn 收集cargo文件(根目录: &Path, 排除项: &[String]) -> Vec<PathBuf> {
    let mut 结果 = Vec::new();
    递归cargo(根目录, 排除项, &mut 结果);
    结果
}

fn 递归cargo(目录: &Path, 排除项: &[String], 结果: &mut Vec<PathBuf>) {
    let Ok(条目们) = std::fs::read_dir(目录) else { return };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 应排除(&名, 排除项) {
                continue;
            }
            递归cargo(&路径, 排除项, 结果);
        } else if 名 == "Cargo.toml" {
            结果.push(路径);
        }
    }
}

/// 递归收集数据文件，跳过排除项。
pub(crate) fn 收集数据文件(根目录: &Path, 排除项: &[String]) -> Vec<PathBuf> {
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

/// 从文件路径向上找最近的 Cargo.toml，返回其目录名（crate 名）。
pub(crate) fn 归属crate(文件: &Path) -> String {
    找crate目录(文件)
        .and_then(|目录| 目录.file_name().map(|名| 名.to_string_lossy().to_string()))
        .unwrap_or_else(|| "（根）".to_string())
}

/// 向上找最近的含 Cargo.toml 的目录（crate 根）。
pub(crate) fn 找crate目录(文件: &Path) -> Option<PathBuf> {
    let mut 目录 = 文件.parent();
    while let Some(路径) = 目录 {
        if 路径.join("Cargo.toml").exists() {
            return Some(路径.to_path_buf());
        }
        目录 = 路径.parent();
    }
    None
}

/// 计算相对根目录的路径（含文件名）。
pub(crate) fn 相对路径(根目录: &Path, 文件: &Path) -> String {
    文件
        .strip_prefix(根目录)
        .map(|相对| 相对.display().to_string())
        .unwrap_or_else(|_| 文件.display().to_string())
}
