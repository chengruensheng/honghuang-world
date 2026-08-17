//! 世界检索：项目源码全文检索，按命中数排序输出
//!
//! 流式扫描工作区源码文件（.rs / .toml，排除构建产物与临时目录），
//! 统计每个文件中关键词出现次数，按命中数降序排列，输出 Top 文件清单。

use crate::工作区根;
use rizhi_fu::{info, warn};
use std::fs;
use std::path::Path;

/// 单文件最大字节阈值：超出跳过，避免大文件 / 编译产物拖慢检索。
const 最大文件字节: u64 = 2_000_000;

/// 检索源码：按命中数排序输出 Top 50 命中文件。
///
/// 用法：世界 检索 <关键词>
/// - `关键词`：要搜索的字符串（必填，不可为空）。
pub fn 全文检索(关键词: &str) -> String {
    info!(关键词 = %关键词, "全文检索开始");
    if 关键词.is_empty() {
        warn!("关键词为空");
        return "世界 检索\n用法：世界 检索 <关键词>\n说明：流式扫描工作区源码，按命中数降序输出 Top 50".to_string();
    }

    let 根 = 工作区根();
    let mut 命中: Vec<(String, usize)> = Vec::new();
    扫描目录(&根, 关键词, &根, &mut 命中);

    // 按命中数降序排序
    命中.sort_by(|a, b| b.1.cmp(&a.1));

    let 总命中: usize = 命中.iter().map(|(_, n)| n).sum();
    let mut 输出 = format!(
        "世界 检索\n关键词：{}\n命中文件：{}\n总命中次数：{}\n\n命中文件（按命中数降序）：\n",
        关键词,
        命中.len(),
        总命中
    );

    if 命中.is_empty() {
        输出.push_str("  （无命中）\n");
    } else {
        for (路径, 次数) in 命中.iter().take(50) {
            输出.push_str(&format!("  {} · {} 次\n", 路径, 次数));
        }
        if 命中.len() > 50 {
            输出.push_str(&format!("  ... 其余 {} 个文件省略\n", 命中.len() - 50));
        }
    }

    info!(关键词 = %关键词, 命中文件 = 命中.len(), 总命中 = 总命中, "全文检索完成");
    输出
}

/// 递归扫描目录，命中文件推入 结果。
fn 扫描目录(目录: &Path, 关键词: &str, 根: &Path, 结果: &mut Vec<(String, usize)>) {
    let 读取 = match fs::read_dir(目录) {
        Ok(r) => r,
        Err(_) => return, // 不可读目录静默跳过
    };
    for 条目 in 读取.flatten() {
        let 路径 = 条目.path();
        let 名称 = 路径.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if 跳过目录(名称) {
            continue;
        }

        if 路径.is_dir() {
            扫描目录(&路径, 关键词, 根, 结果);
        } else if 路径.is_file() {
            统计文件(&路径, 关键词, 根, 结果);
        }
    }
}

/// 统计单文件命中次数；非源码 / 非 UTF-8 / 超大文件自动跳过。
fn 统计文件(路径: &Path, 关键词: &str, 根: &Path, 结果: &mut Vec<(String, usize)>) {
    let 名称 = match 路径.file_name().and_then(|名| 名.to_str()) {
        Some(名) => 名,
        None => return,
    };
    if !是源码文件(名称) {
        return;
    }
    let 元数据 = match fs::metadata(路径) {
        Ok(m) => m,
        Err(_) => return,
    };
    if 元数据.len() > 最大文件字节 {
        return;
    }
    let 内容 = match fs::read_to_string(路径) {
        Ok(c) => c,
        Err(_) => return, // 非 UTF-8（二进制）跳过
    };
    let 次数 = 内容.matches(关键词).count();
    if 次数 > 0 {
        let 相对 = 路径.strip_prefix(根).unwrap_or(路径).display().to_string();
        结果.push((相对, 次数));
    }
}

/// 应跳过的目录名（编译产物 / 版本库 / 上下文数据 / 临时目录）。
fn 跳过目录(名称: &str) -> bool {
    matches!(
        名称,
        "target" | ".git" | "node_modules" | ".上下文" | "道果树" | "临时文件夹"
    )
}

/// 只统计源码类文本文件，跳过文档与大二进制，控制检索面与耗时。
fn 是源码文件(名称: &str) -> bool {
    let 小写 = 名称.to_ascii_lowercase();
    小写.ends_with(".rs") || 小写.ends_with(".toml")
}

#[cfg(test)]
mod 测试 {
    //! 判定类纯函数的单元测试。
    //!
    //! 覆盖三类断言：
    //! ① 源码文件后缀判定（.rs / .toml）
    //! ② target 目录排除
    //! ③ 跳过目录清单逐元素覆盖

    use super::*;

    // ─── 验收①：源码文件后缀判定（.rs / .toml） ────────────────────────

    #[test]
    fn 源码文件接受点rs() {
        assert!(是源码文件("main.rs"));
        assert!(是源码文件("lib.rs"));
        assert!(是源码文件("流式检索.rs"));
    }

    #[test]
    fn 源码文件接受点toml() {
        assert!(是源码文件("Cargo.toml"));
        assert!(是源码文件("配置.toml"));
    }

    #[test]
    fn 源码文件拒绝非源码后缀() {
        assert!(!是源码文件("readme.md"));
        assert!(!是源码文件("note.txt"));
        assert!(!是源码文件("data.json"));
        assert!(!是源码文件("script.py"));
        assert!(!是源码文件("无后缀"));
        assert!(!是源码文件(""));
    }

    #[test]
    fn 源码文件后缀大小写不敏感() {
        assert!(是源码文件("MAIN.RS"));
        assert!(是源码文件("Cargo.TOML"));
        assert!(是源码文件("Main.Rs"));
    }

    // ─── 验收②：target 目录排除 ────────────────────────────────────────

    #[test]
    fn target目录被排除() {
        assert!(跳过目录("target"));
    }

    // ─── 验收③：跳过目录清单逐元素覆盖 ────────────────────────────────

    #[test]
    fn 跳过目录清单逐元素命中() {
        const 排除清单: &[&str] = &[
            "target",
            ".git",
            "node_modules",
            ".上下文",
            "道果树",
            "临时文件夹",
        ];
        for 名 in 排除清单 {
            assert!(跳过目录(名), "清单中的「{名}」应当被排除");
        }
    }

    #[test]
    fn 普通目录不在排除清单中() {
        const 普通目录: &[&str] = &["src", "lib", "tests", "鸿蒙", "乾坤", "证道"];
        for 名 in 普通目录 {
            assert!(!跳过目录(名), "「{名}」不应被排除");
        }
    }
}