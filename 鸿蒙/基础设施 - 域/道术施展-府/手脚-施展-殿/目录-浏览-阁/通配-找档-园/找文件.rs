//! 通配 - 找档 - 园：按通配符模式在目录树下找文件。

use rizhi_fu::{debug, error};
use std::fs;
use std::path::{Path, PathBuf};

/// 按通配模式找路径，支持 `*`（一段内任意字符）、`?`（单字符）、`**`（任意深度）。
/// 返回匹配到的路径，分隔符统一用 `/` 与 `\`。
pub fn 寻找文件(根: &str, 模式: &str) -> Result<Vec<String>, String> {
    let 根路径 = Path::new(根);
    if !根路径.is_dir() {
        error!(根, "找档根目录不存在");
        return Err(format!("根目录不存在：{根}"));
    }
    let 段们 = 拆分路径段(模式);
    let 命中们 = 递归找(根路径, &段们);
    debug!(根, 命中数 = 命中们.len(), "通配找档完成");
    Ok(命中们.iter().map(|路径| 路径.to_string_lossy().to_string()).collect())
}

/// 把模式拆成路径段，忽略空段。
fn 拆分路径段(模式: &str) -> Vec<&str> {
    模式.split(['/', '\\']).filter(|段| !段.is_empty()).collect()
}

fn 递归找(当前: &Path, 段们: &[&str]) -> Vec<PathBuf> {
    if 段们.is_empty() {
        return if 当前.exists() { vec![当前.to_path_buf()] } else { Vec::new() };
    }

    let 首段 = 段们[0];
    let 剩余 = &段们[1..];

    // `**` 匹配零层或一层目录后继续。
    if 首段 == "**" {
        let mut 结果 = 递归找(当前, 剩余);
        if 当前.is_dir() {
            if let Ok(条目们) = fs::read_dir(当前) {
                for 条目 in 条目们.flatten() {
                    结果.extend(递归找(&条目.path(), 段们));
                }
            }
        }
        return 结果;
    }

    if !当前.is_dir() {
        return Vec::new();
    }

    let mut 结果 = Vec::new();
    if let Ok(条目们) = fs::read_dir(当前) {
        for 条目 in 条目们.flatten() {
            let 路径 = 条目.path();
            let 名 = 路径
                .file_name()
                .map(|名| 名.to_string_lossy().to_string())
                .unwrap_or_default();
            if 通配匹配(首段, &名) {
                if 剩余.is_empty() {
                    结果.push(路径);
                } else {
                    结果.extend(递归找(&路径, 剩余));
                }
            }
        }
    }
    结果
}

/// 单段通配匹配：`*` 匹配任意字符，`?` 匹配单字符。
fn 通配匹配(模式: &str, 文本: &str) -> bool {
    let 模式字符: Vec<char> = 模式.chars().collect();
    let 文本字符: Vec<char> = 文本.chars().collect();
    let (mut 模式位, mut 文本位) = (0usize, 0usize);
    let (mut 星位, mut 星后) = (None, 0usize);

    while 文本位 < 文本字符.len() {
        if 模式位 < 模式字符.len() && (模式字符[模式位] == '?' || 模式字符[模式位] == 文本字符[文本位]) {
            模式位 += 1;
            文本位 += 1;
        } else if 模式位 < 模式字符.len() && 模式字符[模式位] == '*' {
            星位 = Some(模式位);
            星后 = 文本位;
            模式位 += 1;
        } else if let Some(位) = 星位 {
            模式位 = 位 + 1;
            星后 += 1;
            文本位 = 星后;
        } else {
            return false;
        }
    }
    while 模式位 < 模式字符.len() && 模式字符[模式位] == '*' {
        模式位 += 1;
    }
    模式位 == 模式字符.len()
}

