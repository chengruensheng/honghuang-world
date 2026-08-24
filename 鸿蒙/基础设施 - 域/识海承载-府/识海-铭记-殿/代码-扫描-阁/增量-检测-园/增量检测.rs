//! 增量 - 检测 - 园：文件索引基线 + 增量变更检测（新增 / 修改 / 删除，.tmp 半写保护）。
//!
//! 定位：种子（记忆 播种）接手时全量打底，地道在执行时做增量收尾——
//! 只比对「上一轮基线 vs 当前盘面」，不重复全量扫描。
//! 测试在 测试.rs。

use crate::世界结果;
use crate::{依赖图, 工作区, 扫描排除项};
use rizhi_fu::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 文件指纹：大小 + 修改时间（unix 毫秒）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct 文件指纹 {
    pub 大小: u64,
    pub 修改: u64,
}

/// 文件索引基线：相对路径（正斜杠）→ 指纹。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 文件索引 {
    pub 指纹们: BTreeMap<String, 文件指纹>,
}

/// 变更报告：新增 / 修改 / 删除 三类清单（正斜杠相对路径）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct 变更报告 {
    pub 新增: Vec<String>,
    pub 修改: Vec<String>,
    pub 删除: Vec<String>,
}

impl 变更报告 {
    /// 变更总处数。
    pub fn 总处数(&self) -> usize {
        self.新增.len() + self.修改.len() + self.删除.len()
    }

    /// 无变更。
    pub fn 空(&self) -> bool {
        self.总处数() == 0
    }
}

/// 基线文件路径：.上下文/状态/文件索引.json。
fn 基线路径(工作区: &工作区) -> PathBuf {
    工作区.上下文目录().join("状态").join("文件索引.json")
}

/// 执行前基线路径：.上下文/状态/执行-基线.json。
/// 与 文件索引.json 分离——后者被地道整理/存档不断更新到当前盘面，
/// 执行基线 固定在任务开始前，供验收审验做「改前 → 改后」对比
/// （2026-08-16 修复：准圣拿当前盘面当改前状态，产物=涉及现状=同字节数 → 误判「未见增量」打回）。
fn 执行基线路径(工作区: &工作区) -> PathBuf {
    工作区.上下文目录().join("状态").join("执行-基线.json")
}

/// 保存执行前基线：任务开始前由派发落单调用（覆盖旧值；并发任务以最近保存者为准）。
pub fn 保存执行基线(工作区: &工作区, 索引: &文件索引) -> 世界结果<()> {
    let 路径 = 执行基线路径(工作区);
    let 父 = 路径.parent().ok_or("执行基线目录缺失")?;
    std::fs::create_dir_all(父).map_err(|错误| format!("建执行基线目录失败: {错误}"))?;
    let 内容 =
        serde_json::to_string_pretty(索引).map_err(|错误| format!("序列化执行基线失败: {错误}"))?;
    std::fs::write(&路径, 内容).map_err(|错误| format!("写执行基线失败: {错误}"))?;
    debug!(路径 = %路径.display(), 文件数 = 索引.指纹们.len(), "执行前基线已保存");
    Ok(())
}

/// 读执行前基线（文件缺失 / 解析失败时返回空基线）。
pub fn 读执行基线(工作区: &工作区) -> 文件索引 {
    let 路径 = 执行基线路径(工作区);
    let Ok(内容) = std::fs::read_to_string(&路径) else {
        return 文件索引::default();
    };
    serde_json::from_str(&内容).unwrap_or_default()
}

/// 读文件索引基线（文件缺失 / 解析失败时返回空基线）。
pub fn 读基线(工作区: &工作区) -> 文件索引 {
    let 路径 = 基线路径(工作区);
    let Ok(内容) = std::fs::read_to_string(&路径) else {
        return 文件索引::default();
    };
    serde_json::from_str(&内容).unwrap_or_default()
}

/// 保存文件索引基线（覆盖旧基线，幂等）。
pub fn 保存基线(工作区: &工作区, 索引: &文件索引) -> 世界结果<()> {
    let 路径 = 基线路径(工作区);
    let 父 = 路径.parent().ok_or("基线目录缺失")?;
    std::fs::create_dir_all(父).map_err(|错误| format!("建基线目录失败: {错误}"))?;
    let 内容 =
        serde_json::to_string_pretty(索引).map_err(|错误| format!("序列化基线失败: {错误}"))?;
    std::fs::write(&路径, 内容).map_err(|错误| format!("写基线失败: {错误}"))?;
    debug!(路径 = %路径.display(), 文件数 = 索引.指纹们.len(), "文件索引基线已保存");
    Ok(())
}

/// 全量建立基线：收集 .rs 源文件与 Cargo.toml（跳过排除项，跳过 .tmp 半写文件）。
pub fn 全量基线(根: &Path) -> 文件索引 {
    let 排除项 = 扫描排除项(根);
    let mut 索引 = 文件索引::default();
    收集指纹(根, 根, &排除项, &mut 索引.指纹们);
    info!(根 = %根.display(), 文件数 = 索引.指纹们.len(), "全量基线建立");
    索引
}

/// 增量变更检测：旧基线 vs 当前盘面。
/// .tmp 半写保护：存在未完成写入时本轮返回空报告（交给下一轮，防误判）。
pub fn 增量变更(根: &Path, 旧: &文件索引) -> 变更报告 {
    if 有半写文件(根) {
        warn!(根 = %根.display(), "检测到 .tmp 半写文件，本轮变更检测跳过");
        return 变更报告::default();
    }
    let 排除项 = 扫描排除项(根);
    let mut 当前: BTreeMap<String, 文件指纹> = BTreeMap::new();
    收集指纹(根, 根, &排除项, &mut 当前);
    let mut 报告 = 变更报告::default();
    for (路径, 指纹) in &当前 {
        match 旧.指纹们.get(路径) {
            None => 报告.新增.push(路径.clone()),
            Some(旧指纹) if 旧指纹 != 指纹 => 报告.修改.push(路径.clone()),
            Some(_) => {}
        }
    }
    for 路径 in 旧.指纹们.keys() {
        if !当前.contains_key(路径) {
            报告.删除.push(路径.clone());
        }
    }
    debug!(
        新增 = 报告.新增.len(),
        修改 = 报告.修改.len(),
        删除 = 报告.删除.len(),
        "增量变更检测完成"
    );
    报告
}

/// 地道整理：读基线 → 增量变更检测 → 有变更时以当前盘面重建基线 → 返回变更报告。
/// 首次（无基线）只建基线打底、不报变更（种子已全量打底，地道从下一轮开始增量）；
/// .tmp 半写时本轮跳过。由主控在 LLM 执行期间并行调用，join 后统一登记。
/// 识别删除后同步清理依赖图陈旧边（设计稿 §14.20.6）。
pub fn 地道整理(工作区: &工作区) -> 世界结果<变更报告> {
    let 根 = 工作区.根路径();
    let 旧 = 读基线(工作区);
    if 旧.指纹们.is_empty() {
        let 新 = 全量基线(根);
        保存基线(工作区, &新)?;
        info!(文件数 = 新.指纹们.len(), "地道首次基线建立");
        return Ok(变更报告::default());
    }
    let 报告 = 增量变更(根, &旧);
    if !报告.空() {
        let 新 = 全量基线(根);
        保存基线(工作区, &新)?;
    }
    if !报告.删除.is_empty() {
        清理依赖图陈旧边(工作区, &报告.删除);
    }
    Ok(报告)
}

/// 清理依赖图陈旧边：加载依赖图 → 对每个删除文件调 清理文件 → 保存。
/// 失败仅 warn 不阻断（全量重建兜底：任务后扫描天然排除已删文件）。
fn 清理依赖图陈旧边(工作区: &工作区, 删除清单: &[String]) {
    let mut 图 = match 依赖图::加载自工作区(工作区) {
        Ok(图) => 图,
        Err(说明) => {
            warn!(说明 = %说明, "清理陈旧边：加载依赖图失败，跳过（全量重建兜底）");
            return;
        }
    };
    let mut 总移除 = 0usize;
    for 路径 in 删除清单 {
        总移除 += 图.清理文件(路径);
    }
    if 总移除 == 0 {
        return;
    }
    if let Err(说明) = 图.保存在工作区(工作区) {
        warn!(说明 = %说明, "清理陈旧边：保存依赖图失败，跳过（全量重建兜底）");
        return;
    }
    info!(
        删除数 = 删除清单.len(),
        移除档案数 = 总移除,
        "依赖图陈旧边已清理"
    );
}

/// 递归收集源文件指纹（.rs + Cargo.toml），相对路径统一正斜杠。
fn 收集指纹(
    根: &Path, 目录: &Path, 排除项: &[String], 结果: &mut BTreeMap<String, 文件指纹>
) {
    let Ok(条目们) = std::fs::read_dir(目录) else {
        return;
    };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 排除项.contains(&名) {
                continue;
            }
            收集指纹(根, &路径, 排除项, 结果);
        } else if 名.ends_with(".rs") || 名 == "Cargo.toml" {
            if let Some(指纹) = 文件指纹(&路径) {
                let 相对 = 路径
                    .strip_prefix(根)
                    .map(|相对| 相对.display().to_string())
                    .unwrap_or_else(|_| 路径.display().to_string())
                    .replace('\\', "/");
                结果.insert(相对, 指纹);
            }
        }
    }
}

/// 是否存在未完成的半写文件（*.tmp）。
fn 有半写文件(根: &Path) -> bool {
    let 排除项 = 扫描排除项(根);
    递归查半写(根, &排除项)
}

fn 递归查半写(目录: &Path, 排除项: &[String]) -> bool {
    let Ok(条目们) = std::fs::read_dir(目录) else {
        return false;
    };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 排除项.contains(&名) {
                continue;
            }
            if 递归查半写(&路径, 排除项) {
                return true;
            }
        } else if 名.ends_with(".tmp") {
            return true;
        }
    }
    false
}

/// 读取文件指纹（大小 + 修改时间）。
fn 文件指纹(路径: &Path) -> Option<文件指纹> {
    let 元 = std::fs::metadata(路径).ok()?;
    let 修改 = 元
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as u64;
    Some(文件指纹 {
        大小: 元.len(),
        修改,
    })
}
