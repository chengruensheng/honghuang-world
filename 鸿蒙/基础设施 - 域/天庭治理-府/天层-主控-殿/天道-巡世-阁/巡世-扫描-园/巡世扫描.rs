//! 巡世 - 扫描 - 园：扫描世界，产出巡世报告与违逆清单。

use crate::类型_定义_殿::{巡世报告, 巡世候选, 要求类别, 优先级};
use std::path::{Path, PathBuf};

/// 扫描世界：收集源文件，按规模启发产出候选改进点。
pub fn 扫描世界(根目录: &Path) -> 巡世报告 {
    let 文件们 = 收集源文件(根目录);
    let mut 候选 = Vec::new();
    if 文件们.len() > 200 {
        候选.push(巡世候选 {
            目标: "项目规模较大，考虑按域拆分为更多府".to_string(),
            依据: format!("源文件数 {}", 文件们.len()),
            建议类别: 要求类别::优化,
            优先级: 优先级::低,
        });
    }
    巡世报告 {
        id: "巡世-1".to_string(),
        时间: 0,
        候选,
        违逆: Vec::new(),
    }
}

/// 递归收集 .rs 源文件，跳过排除项。
fn 收集源文件(根目录: &Path) -> Vec<PathBuf> {
    let 排除项 = shihai_fu::扫描排除项(根目录);
    let mut 结果 = Vec::new();
    递归(根目录, &排除项, &mut 结果);
    结果
}

fn 递归(目录: &Path, 排除项: &[String], 结果: &mut Vec<PathBuf>) {
    let Ok(条目们) = std::fs::read_dir(目录) else { return };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 排除项.iter().any(|项| 项 == &名) {
                continue;
            }
            递归(&路径, 排除项, 结果);
        } else if 名.ends_with(".rs") {
            结果.push(路径);
        }
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use std::fs;

    #[test]
    fn 扫描产出报告() {
        let 目录 = std::env::temp_dir().join("识海测试-巡世");
        fs::create_dir_all(&目录).unwrap();
        fs::write(目录.join("a.rs"), "x").unwrap();
        let 报告 = 扫描世界(&目录);
        assert!(报告.候选.is_empty()); // 文件数少，无候选
        let _ = fs::remove_dir_all(&目录);
    }
}
