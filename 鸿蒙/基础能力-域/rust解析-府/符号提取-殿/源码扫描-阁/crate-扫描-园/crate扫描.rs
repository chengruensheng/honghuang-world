//! Rust 单元包（crate）扫描：读 workspace 的 `Cargo.toml` 得到包名与包间依赖
//!
//! 产出 `依赖` 边（设计 §4.3），置信度 **高**——依赖关系来自 Cargo 清单，
//! 不是语法推断。
//!
//! 为什么不调 `cargo metadata`：引擎若由 `cargo run` 启动会持有 target 锁，
//! 子进程再触发 cargo 将互等死锁。直接读清单既无锁风险，也无额外进程开销。

use hm_symext::{边, 符号, 边种类, 符号种类, 置信度};
use std::collections::BTreeSet;
use std::path::Path;

/// Cargo 清单文件名（外部数据契约）
const 清单文件名: &str = "Cargo.toml";

/// 一个单元包的清单信息
pub struct 单元包信息 {
    pub 名: String,
    /// 仓库相对路径，如 `鸿蒙/基础能力-域/符号解析-府/Cargo.toml`
    pub 清单路径: String,
    /// 声明的依赖名（含 dev / build），未过滤是否为 workspace 内部
    pub 依赖: Vec<String>,
}

/// 读根 `Cargo.toml` 的 `workspace.members`，逐个读其清单
pub fn 扫描单元包(项目根: &Path) -> Result<Vec<单元包信息>, String> {
    let 根清单 = 项目根.join(清单文件名);
    let 文本 =
        std::fs::read_to_string(&根清单).map_err(|e| format!("读根 Cargo.toml 失败：{e}"))?;
    let 值: toml::Value =
        toml::from_str(&文本).map_err(|e| format!("根 Cargo.toml 非法：{e}"))?;

    let 成员们: Vec<String> = 值
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .map(|数组| {
            数组
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut 结果 = Vec::new();
    for 成员 in 成员们 {
        let 清单路径 = 项目根.join(&成员).join(清单文件名);
        let 文本 = match std::fs::read_to_string(&清单路径) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let 值: toml::Value = match toml::from_str(&文本) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let 名 = match 值
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        {
            Some(n) => n.to_string(),
            None => continue,
        };

        let mut 依赖 = Vec::new();
        for 段 in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(表) = 值.get(段).and_then(|d| d.as_table()) {
                for 键 in 表.keys() {
                    依赖.push(键.clone());
                }
            }
        }

        结果.push(单元包信息 {
            名,
            清单路径: format!("{成员}/{清单文件名}"),
            依赖,
        });
    }

    Ok(结果)
}

/// 由单元包信息生成 crate 符号与 `依赖` 边
///
/// 只保留 **workspace 内部** 依赖：外部 crate（如 serde）没有对应符号，
/// 建边会被降级校验判为悬空，反而制造噪音。
pub fn 生成符号与边(单元包们: &[单元包信息]) -> (Vec<符号>, Vec<边>) {
    let 已知: BTreeSet<&str> = 单元包们.iter().map(|包| 包.名.as_str()).collect();

    let mut 符号们 = Vec::new();
    let mut 边们 = Vec::new();
    let mut 已建: BTreeSet<String> = BTreeSet::new();

    for 包 in 单元包们 {
        let id = format!("rust::{}", 包.名);

        符号们.push(符号 {
            id: id.clone(),
            种类: 符号种类::Crate,
            名: 包.名.clone(),
            全名: 包.名.clone(),
            文件: 包.清单路径.clone(),
            行: 1,
            可见性: "pub".to_string(),
            容器: None,
        });

        for 依赖 in &包.依赖 {
            if !已知.contains(依赖.as_str()) {
                continue;
            }
            let 到 = format!("rust::{依赖}");
            if !已建.insert(format!("{id}->{到}")) {
                continue;
            }
            边们.push(边 {
                从: id.clone(),
                到,
                类型: 边种类::依赖,
                置信度: 置信度::高,
                位置: Some(hm_symext::位置 {
                    文件: 包.清单路径.clone(),
                    行: 1,
                    列: None,
                }),
            });
        }
    }

    (符号们, 边们)
}
