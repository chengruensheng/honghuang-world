//! 扫描 - 执行 - 园：六类扫描入口，落格位快照。

use crate::{记录, 模型存储, 扫描排除项, 工作区};
use rizhi_fu::debug;
use std::collections::BTreeMap;
use std::path::Path;

use super::收集::{收集cargo文件, 收集数据文件, 收集源文件, 归属crate, 相对路径};
use super::符号解析::提取符号签名;
use super::依赖边::扫描依赖边;

/// 扫描文件清单 → 「文件」格位（府级快照：每府源文件数），返回文件数。
pub fn 扫描文件清单(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let 文件们 = 收集源文件(根目录, &排除项);
    let mut 府文件: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for 文件 in &文件们 {
        let 府 = 归属crate(文件);
        let 相对 = 相对路径(根目录, 文件).replace('\\', "/");
        府文件.entry(府).or_default().push(相对);
    }
    let mut 行们 = Vec::new();
    for (府, 清单) in &府文件 {
        行们.push(format!("{府}：{} 个源文件", 清单.len()));
    }
    let 快照 = 行们.join("\n");
    存储.写记录(&记录::新("文件", &快照, &format!("扫描 {} 个源文件", 文件们.len()), "代码"))?;
    Ok(文件们.len())
}

/// 扫描目录结构 → 「结构」格位（crate 列表：含 Cargo.toml 的目录），返回 crate 数。
/// 块状落盘：只写一条快照（链头），crate 内目录细节按需经依赖图下探。
pub fn 扫描目录结构(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let cargo们 = 收集cargo文件(根目录, &排除项);
    let mut crate们: Vec<String> = cargo们
        .iter()
        .filter_map(|文件| 文件.parent())
        .map(|目录| 相对路径(根目录, 目录).replace('\\', "/"))
        .filter(|路径| !路径.is_empty())
        .collect();
    crate们.sort();
    crate们.dedup();
    let 快照 = crate们.join("\n");
    存储.写记录(&记录::新("结构", &快照, &format!("扫描 {} 个 crate", crate们.len()), "代码"))?;
    Ok(crate们.len())
}

/// 扫描所有 Cargo.toml 依赖 → 「环境·依赖」格位（按府聚合），返回依赖总数。
pub fn 扫描依赖(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let 文件们 = 收集cargo文件(根目录, &排除项);
    let mut 府依赖: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for 文件 in &文件们 {
        let 依赖们 = 解析依赖(文件)?;
        if 依赖们.is_empty() {
            continue;
        }
        let 府 = 归属crate(文件);
        for 依赖 in 依赖们 {
            府依赖.entry(府.clone()).or_default().push(依赖);
        }
    }
    let mut 行们 = Vec::new();
    let mut 总数 = 0;
    for (府, 依赖们) in &府依赖 {
        let mut 去重 = 依赖们.clone();
        去重.sort();
        去重.dedup();
        总数 += 去重.len();
        行们.push(format!("{府}：{}", 去重.join("、")));
    }
    let 快照 = 行们.join("\n");
    存储.写记录(&记录::新("环境·依赖", &快照, &format!("扫描 {总数} 个依赖"), "代码"))?;
    Ok(总数)
}

/// 解析单个 Cargo.toml 的依赖段，返回依赖名列表。
fn 解析依赖(文件: &Path) -> Result<Vec<String>, String> {
    let 内容 = std::fs::read_to_string(文件).map_err(|错误| format!("读 Cargo.toml 失败: {错误}"))?;
    let mut 依赖们 = Vec::new();
    let mut 在依赖段 = false;
    for 行 in 内容.lines() {
        let 行 = 行.trim();
        if 行.starts_with('[') {
            在依赖段 = 行 == "[dependencies]" || 行 == "[dev-dependencies]" || 行 == "[build-dependencies]";
            continue;
        }
        if 在依赖段 && !行.is_empty() && !行.starts_with('#') {
            if let Some(名) = 行.split(|字符: char| 字符 == '=' || 字符 == ' ' || 字符 == '{').next() {
                let 名 = 名.trim();
                if !名.is_empty() {
                    依赖们.push(名.to_string());
                }
            }
        }
    }
    Ok(依赖们)
}

/// 扫描 .rs 函数签名 → 「调用」格位（府级快照：每府 pub 符号名），返回符号数。
pub fn 扫描符号(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let 文件们 = 收集源文件(根目录, &排除项);
    let mut 府符号: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for 文件 in &文件们 {
        let 内容 = std::fs::read_to_string(文件).map_err(|错误| format!("读源文件失败: {错误}"))?;
        let 府 = 归属crate(文件);
        for 行 in 内容.lines() {
            // 只收 pub 符号（业务接口），跳过私有 fn 与测试 fn
            if let Some((符号名, _)) = 提取符号签名(行) {
                府符号.entry(府.clone()).or_default().push(符号名);
            }
        }
    }
    let mut 行们 = Vec::new();
    for (府, 符号们) in &府符号 {
        行们.push(format!("{府}：{}", 符号们.join("、")));
    }
    let 快照 = 行们.join("\n");
    let 总数: usize = 府符号.values().map(|符号们| 符号们.len()).sum();
    存储.写记录(&记录::新("调用", &快照, &format!("扫描 {总数} 个符号"), "代码"))?;
    Ok(总数)
}

/// 扫描数据文件 → 「数据」格位（.json/.jsonl/.csv/.db），返回条数。
pub fn 扫描数据(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根目录);
    let 文件们 = 收集数据文件(根目录, &排除项);
    for 文件 in &文件们 {
        let 相对 = 文件.display().to_string();
        let 记录 = 记录::新("数据", &相对, &format!("数据「{相对}」"), "代码");
        存储.写记录(&记录)?;
    }
    Ok(文件们.len())
}

/// 综合扫描：文件 + 结构 + 依赖 + 符号 + 数据 + 事件，返回总条数。
pub fn 扫描(存储: &模型存储, 根目录: &Path) -> Result<usize, String> {
    let mut 条数 = 0;
    条数 += 扫描文件清单(存储, 根目录)?;
    条数 += 扫描目录结构(存储, 根目录)?;
    条数 += 扫描依赖(存储, 根目录)?;
    条数 += 扫描符号(存储, 根目录)?;
    条数 += 扫描数据(存储, 根目录)?;
    // 依赖图：扫 use / pub use 依赖边，落盘 .上下文/依赖图.json，供执行层精确读现状。
    let 图 = 扫描依赖边(根目录)?;
    图.保存在工作区(&工作区::新(根目录))?;
    存储.写记录(&记录::新("事件", "代码扫描完成", &format!("扫描 {}", 根目录.display()), "代码"))?;
    debug!(根 = %根目录.display(), 条数, "代码扫描完成");
    Ok(条数 + 1)
}
