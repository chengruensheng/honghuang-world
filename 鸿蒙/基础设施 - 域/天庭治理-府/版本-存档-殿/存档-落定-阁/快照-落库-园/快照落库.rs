//! 快照 - 落库 - 园：源码快照、版本记录与回退。

use crate::类型_定义_殿::{阶段, 版本记录};
use std::fs;
use std::path::Path;

/// 复制源码到快照目录，排除构建产物/版本库/依赖，返回复制文件数。
pub fn 源码快照(源目录: &Path, 目标目录: &Path) -> Result<usize, String> {
    let 排除项 = shihai_fu::扫描排除项(源目录);
    let mut 计数 = 0;
    复制目录(源目录, 目标目录, &排除项, &mut 计数)?;
    Ok(计数)
}

fn 复制目录(源: &Path, 目标: &Path, 排除项: &[String], 计数: &mut usize) -> Result<(), String> {
    fs::create_dir_all(目标).map_err(|错误| format!("建目录失败: {错误}"))?;
    for 条目 in fs::read_dir(源).map_err(|错误| format!("读目录失败: {错误}"))? {
        let 条目 = 条目.map_err(|错误| format!("读条目失败: {错误}"))?;
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 排除项.iter().any(|项| 项 == &名) {
            continue;
        }
        let 源路径 = 条目.path();
        let 目标路径 = 目标.join(&名);
        if 源路径.is_dir() {
            复制目录(&源路径, &目标路径, 排除项, 计数)?;
        } else {
            fs::copy(&源路径, &目标路径).map_err(|错误| format!("复制文件失败: {错误}"))?;
            *计数 += 1;
        }
    }
    Ok(())
}

/// 生成一条版本记录。
pub fn 生成版本记录(
    版本号: &str,
    时间: u64,
    阶段: 阶段,
    改了什么: &str,
    源码快照路径: &str,
    构建产物路径: &str,
    验收结论: Vec<String>,
    对比上一版: &str,
) -> 版本记录 {
    版本记录 {
        版本号: 版本号.to_string(),
        时间,
        阶段,
        改了什么: 改了什么.to_string(),
        源码快照路径: 源码快照路径.to_string(),
        构建产物路径: 构建产物路径.to_string(),
        验收结论,
        对比上一版: 对比上一版.to_string(),
    }
}

/// 回退版本：清空目标目录，从快照恢复。
pub fn 回退版本(快照目录: &Path, 目标目录: &Path) -> Result<usize, String> {
    if 目标目录.exists() {
        fs::remove_dir_all(目标目录).map_err(|错误| format!("清目标失败: {错误}"))?;
    }
    let 排除项 = shihai_fu::扫描排除项(快照目录);
    let mut 计数 = 0;
    复制目录(快照目录, 目标目录, &排除项, &mut 计数)?;
    Ok(计数)
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 快照排除构建物() {
        let 源 = std::env::temp_dir().join("识海测试-快照源");
        let 目标 = std::env::temp_dir().join("识海测试-快照目标");
        fs::create_dir_all(源.join("target")).unwrap();
        fs::write(源.join("a.rs"), "x").unwrap();
        fs::write(源.join("target/b.rs"), "y").unwrap();
        let 数 = 源码快照(&源, &目标).unwrap();
        assert_eq!(数, 1); // 只复制 a.rs，跳过 target
        let _ = fs::remove_dir_all(&源);
        let _ = fs::remove_dir_all(&目标);
    }
}
