//! 流式 - 检索 - 园：在目录树下按字面串检索文本行。

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
pub fn 搜索内容(根: &str, 字面串: &str) -> Result<Vec<检索命中>, String> {
    let 根路径 = Path::new(根);
    if !根路径.is_dir() {
        return Err(format!("根目录不存在：{根}"));
    }
    if 字面串.is_empty() {
        return Ok(Vec::new());
    }
    let mut 命中们 = Vec::new();
    递归检索(根路径, 字面串, &mut 命中们);
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

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_搜内容_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 搜内容命中带行号() {
        let 目录 = 临时路径("命中");
        std::fs::create_dir_all(&目录).unwrap();
        std::fs::write(目录.join("a.rs"), "第一行\n目标词\n第三行").unwrap();

        let 命中们 = 搜索内容(目录.to_str().unwrap(), "目标词").unwrap();
        assert_eq!(命中们.len(), 1);
        assert_eq!(命中们[0].行号, 2);
        assert_eq!(命中们[0].行内容, "目标词");

        std::fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 搜内容空串不命中() {
        let 目录 = 临时路径("空串");
        std::fs::create_dir_all(&目录).unwrap();
        assert!(搜索内容(目录.to_str().unwrap(), "").unwrap().is_empty());
        std::fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 搜内容根不存在报错() {
        let 目录 = 临时路径("无此根");
        assert!(搜索内容(目录.to_str().unwrap(), "x").is_err());
    }
}
