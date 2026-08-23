//! 星图投影 —— 依赖图 → 节点 + 边，§13.f.10.3b 星空视图。
//!
//! 数据源：shihai_fu::依赖图::加载自工作区()。

use serde::{Deserialize, Serialize};
use shihai_fu::{依赖图, 工作区};

/// 星图节点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 星图节点 {
    pub id: String,
    pub 名字: String,
    pub 文件: String,
    pub crate_名: String,
    pub 大小: u32,
    pub 类型: String,
}

/// 星图边。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 星图边 {
    pub 源: String,
    pub 目标: String,
}

/// 星图全量。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct 星图 {
    pub 节点们: Vec<星图节点>,
    pub 边们: Vec<星图边>,
    pub 节点数: usize,
    pub 边数: usize,
}

pub fn 投影星图(图: &依赖图) -> 星图 {
    let mut 节点们: Vec<星图节点> = Vec::with_capacity(图.档案们.len());
    let mut 边们: Vec<星图边> = Vec::new();

    use std::collections::HashMap;
    let mut 名索引: HashMap<String, usize> = HashMap::with_capacity(图.档案们.len());
    for (i, 档) in 图.档案们.iter().enumerate() {
        let id = format!("{}::{}", 档.模块, 档.符号);
        let crate_名 = crate_名(&档.文件);
        let 类型 = if 档.签名.starts_with("pub fn ")
            || 档.签名.starts_with("fn ")
            || 档.签名.starts_with("pub async fn ")
        {
            "fn".to_string()
        } else {
            "type".to_string()
        };
        节点们.push(星图节点 {
            id: id.clone(),
            名字: 档.符号.clone(),
            文件: 档.文件.clone(),
            crate_名,
            大小: 1,
            类型,
        });
        名索引.insert(id, i);
    }

    let mut 入度: HashMap<usize, u32> = HashMap::new();
    for 档 in &图.档案们 {
        let 源id = format!("{}::{}", 档.模块, 档.符号);
        let 源idx = 名索引.get(&源id).copied();
        for 被调用 in &档.波及 {
            if let Some(被调用符号) = 解析被调用(被调用) {
                if let Some(目标idx) = 找节点idx(&节点们, &被调用符号) {
                    if let Some(源idx) = 源idx {
                        if 源idx != 目标idx {
                            入度.entry(目标idx).and_modify(|n| *n += 1).or_insert(1);
                            边们.push(星图边 {
                                源: 节点们[源idx].id.clone(),
                                目标: 节点们[目标idx].id.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    for (i, n) in 节点们.iter_mut().enumerate() {
        n.大小 = 入度.get(&i).copied().unwrap_or(1);
    }

    let 节点数 = 节点们.len();
    let 边数 = 边们.len();
    星图 {
        节点们,
        边们,
        节点数,
        边数,
    }
}

pub fn 加载星图() -> 星图 {
    let 工作区 = 工作区::定位();
    let 图 = match 依赖图::加载自工作区(&工作区) {
        Ok(g) => g,
        Err(_) => return 星图::default(),
    };
    投影星图(&图)
}

fn crate_名(文件: &str) -> String {
    let parts: Vec<&str> = 文件.split('/').collect();
    if parts.len() >= 3 {
        parts[2].to_string()
    } else if !parts.is_empty() {
        parts[0].to_string()
    } else {
        "unknown".to_string()
    }
}

fn 解析被调用(波及: &str) -> Option<String> {
    // 波及形如：
    //   "crate::module::Symbol"   → Symbol
    //   "path/to/file.rs::Symbol" → Symbol
    //   "path/to/file.rs"         → 文件名（去掉 .rs）
    if let Some(双冒号_pos) = 波及.rfind("::") {
        Some(波及[双冒号_pos + 2..].to_string())
    } else if 波及.ends_with(".rs") {
        // 文件路径：取 basename 去扩展名
        let 末尾斜杠 = 波及.rfind('/').map(|i| i + 1).unwrap_or(0);
        let basename = &波及[末尾斜杠..];
        let 去扩展 = basename.strip_suffix(".rs").unwrap_or(basename);
        if 去扩展.is_empty() {
            None
        } else {
            Some(去扩展.to_string())
        }
    } else {
        // 既无 :: 也非 .rs 文件 → 不是有效符号
        None
    }
}

fn 找节点idx(节点们: &[星图节点], 符号: &str) -> Option<usize> {
    for (i, n) in 节点们.iter().enumerate() {
        if n.名字 == 符号 {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 投影空图_返回空星图() {
        let 图 = 依赖图::default();
        let 星图 = 投影星图(&图);
        assert_eq!(星图.节点们.len(), 0);
        assert_eq!(星图.边们.len(), 0);
    }

    #[test]
    fn crate_名_提取() {
        assert_eq!(
            crate_名("鸿蒙/基础设施 - 域/识海承载-府/入口.rs"),
            "识海承载-府"
        );
        assert_eq!(crate_名("乾坤/呈现-域/命令操作-府/入口.rs"), "命令操作-府");
    }

    #[test]
    fn 解析被调用_取末尾符号() {
        assert_eq!(
            解析被调用("crate::module::Symbol"),
            Some("Symbol".to_string())
        );
        assert_eq!(解析被调用("a::b::c::Foo"), Some("Foo".to_string()));
        assert_eq!(解析被调用("no_colon"), None);
    }
}
