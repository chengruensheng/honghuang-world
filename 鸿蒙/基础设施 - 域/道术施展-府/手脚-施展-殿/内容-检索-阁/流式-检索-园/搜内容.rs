//! 流式 - 检索 - 园：在目录树下按字面串检索文本行。

use rizhi_fu::{debug, error};
use shihai_fu::世界结果;
use std::fs;
use std::path::Path;

/// 一条检索命中：文件路径 + 行号 + 行内容。
#[derive(Clone, Debug, PartialEq)]
pub struct 检索命中 {
    pub 路径: String,
    pub 行号: usize,
    pub 行内容: String,
}

/// 跳过这些构建物与版本库目录。
fn 应跳过(名: &str) -> bool {
    名 == "target" || 名 == ".git" || 名 == "node_modules" || 名.contains("构建物")
}

/// 在根目录下检索含指定字面串的文本行（跳过构建物与版本库目录）。
pub fn 搜索内容(根: &str, 字面串: &str) -> 世界结果<Vec<检索命中>> {
    let 根路径 = Path::new(根);
    if !根路径.is_dir() {
        // 根填成文件路径时给出纠正提示（模型常把文件路径当根传）。
        error!(根, "检索根目录不存在");
        if 根路径.is_file() {
            return Err(format!(
                "根必须是目录，不能是文件路径：{根}。请把根改成文件所在目录，如 鸿蒙/基础设施 - 域"
            )
            .into());
        }
        return Err(format!("根目录不存在：{根}").into());
    }
    if 字面串.is_empty() {
        return Ok(Vec::new());
    }
    let mut 命中们 = Vec::new();
    递归检索(根路径, 字面串, &mut 命中们);
    debug!(根, 命中数 = 命中们.len(), "内容检索完成");
    Ok(命中们)
}

fn 递归检索(当前: &Path, 字面串: &str, 命中们: &mut Vec<检索命中>) {
    if !当前.is_dir() {
        return;
    }
    if let Ok(条目们) = fs::read_dir(当前) {
        for 条目 in 条目们.flatten() {
            let 路径 = 条目.path();
            if 路径.is_dir() {
                let 名 = 路径
                    .file_name()
                    .map(|名| 名.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !应跳过(&名) {
                    递归检索(&路径, 字面串, 命中们);
                }
            } else if let Ok(内容) = fs::read_to_string(&路径) {
                for (行号, 行) in 内容.lines().enumerate() {
                    if 行.contains(字面串) {
                        命中们.push(检索命中 {
                            路径: 路径.to_string_lossy().to_string(),
                            行号: 行号 + 1,
                            行内容: 行.to_string(),
                        });
                    }
                }
            }
        }
    }
}
