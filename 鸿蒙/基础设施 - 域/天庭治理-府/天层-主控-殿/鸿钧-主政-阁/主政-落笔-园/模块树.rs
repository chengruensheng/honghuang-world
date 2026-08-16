//! 模块 - 树：校验产物文件是否被模块桥接声明（反孤儿）。

use std::path::{Path, PathBuf};

/// 检查产物文件是否接入模块树（反孤儿）。
/// 从文件向上逐层校验 pub mod 声明链，直到 crate 根（含 Cargo.toml 的目录）。
/// 模块.rs 是「目录的模块文件」而非子模块，自身作为产物时跳过校验，用所在目录名向上查。
pub(crate) fn 接入模块树(根: &Path, 产物路径: &str) -> bool {
    let 绝对 = 根.join(产物路径);
    let mut 目录 = 绝对.parent().map(|路径| 路径.to_path_buf());
    let mut 当前名 = 绝对
        .file_name()
        .map(|名| 名.to_string_lossy().to_string())
        .unwrap_or_default();

    while let Some(目录路径) = 目录 {
        let 是crate根 = 目录路径.join("Cargo.toml").exists();
        // 模块文件自身（模块.rs/mod.rs 或 crate 根声明文件）作为产物时天然在声明链上：
        // 模块.rs 跳过自身、用所在目录名向上查；crate 根声明文件直接放行。
        if 当前名 == "模块.rs" || 当前名 == "mod.rs" {
            if 是crate根 {
                return true;
            }
            当前名 = 目录路径
                .file_name()
                .map(|名| 名.to_string_lossy().to_string())
                .unwrap_or_default();
            目录 = 目录路径.parent().map(|路径| 路径.to_path_buf());
            continue;
        }
        if 是crate根 && ["lib.rs", "main.rs", "入口.rs"].contains(&当前名.as_str()) {
            return true;
        }
        let Some(模块文件) = 模块文件(&目录路径, 是crate根) else {
            return false;
        };
        let 内容 = std::fs::read_to_string(&模块文件).unwrap_or_default();
        if !声明了子模块(&内容, &当前名) {
            return false;
        }
        if 是crate根 {
            return true;
        }
        当前名 = 目录路径
            .file_name()
            .map(|名| 名.to_string_lossy().to_string())
            .unwrap_or_default();
        目录 = 目录路径.parent().map(|路径| 路径.to_path_buf());
    }
    false
}

/// 某目录的模块声明文件：crate 根用 lib.rs/main.rs/入口.rs，子目录用 mod.rs/模块.rs。
fn 模块文件(目录: &Path, 是crate根: bool) -> Option<PathBuf> {
    let 候选们: &[&str] = if 是crate根 {
        &["lib.rs", "main.rs", "入口.rs"]
    } else {
        &["mod.rs", "模块.rs"]
    };
    候选们
        .iter()
        .map(|名| 目录.join(名))
        .find(|路径| 路径.exists())
}

/// 检查模块文件内容里是否声明了某个子模块（层名去 .rs 后缀、连字符转下划线）。
fn 声明了子模块(内容: &str, 层名: &str) -> bool {
    let 名 = 层名.trim_end_matches(".rs").replace('-', "_");
    let 声明 = format!("mod {名};");
    let 声明块 = format!("mod {名} {{");
    内容.contains(&声明) || 内容.contains(&声明块)
}
