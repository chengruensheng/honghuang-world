//! 版本定档：源码快照存档 / 回退
//!
//! 「版本 存档」完整闭环（设计稿 §5 版本递增与增量快照）：
//! 1) 版本号自动递增：读版本历史末条 → 版本-v(N+1)，不覆盖旧版；
//!    增量快照到 `.上下文/版本库/版本-vN/源码-快照/`（相对上一版只复制新增/修改，无上一版则全量即 v1）；
//! 2) 生成版本记录（版本号/时间/阶段/改了什么/源码快照路径/验收结论/对比上一版）→ 追加到 `.上下文/状态/版本.jsonl`（原子写）
//! 3) 追加世界状态版本历史并原子写回（v1已存档 保持 true 不回退）

use crate::工作区根;
use crate::状态目录;
use rizhi_fu::{error, info, warn};
use shihai_fu::世界结果;
use std::path::{Path, PathBuf};

fn 版本库(根: &Path) -> PathBuf {
    根.join(".上下文").join("版本库")
}

/// 每次存档都先保证世界状态存在：不存在则原子落盘默认（阶段=甲、v1已存档=false）。
fn 确保状态已初始化(状态目录: &Path) -> 世界结果<tianting_fu::世界状态> {
    tianting_fu::确保世界状态初始化(状态目录).map_err(shihai_fu::世界错误::世界错误::from)
}

/// 收集最近一次验收历史作为本次版本存档的「验收结论」字段。
fn 最近验收结论(状态目录: &Path) -> Vec<String> {
    let 文件 = 状态目录.join("验收.jsonl");
    let 内容 = match std::fs::read_to_string(&文件) {
        Ok(值) => 值,
        Err(_) => return Vec::new(),
    };
    内容
        .lines()
        .filter(|行| !行.trim().is_empty())
        .rev()
        .take(5)
        .map(|行| 行.to_string())
        .collect()
}

/// 验收结论概要：取最近验收行的「要求id + 结论」一句话。
fn 验收概要(验收结论们: &[String]) -> String {
    let mut 概要 = String::new();
    for 行 in 验收结论们.iter().take(2) {
        if let Ok(值) = serde_json::from_str::<serde_json::Value>(行) {
            let 要求id = 值["要求id"].as_str().unwrap_or("?");
            let 结论 = 值["结论"].as_str().unwrap_or("?");
            概要.push_str(&format!("{要求id}:{结论} "));
        }
    }
    if 概要.is_empty() {
        "（最近无验收）".to_string()
    } else {
        概要.trim_end().to_string()
    }
}

/// 版本号自动递增：「v1」→「v2」；无历史返回「v1」。
fn 递进版本号(上一版: Option<&str>) -> String {
    let Some(末) = 上一版 else {
        return "v1".to_string();
    };
    let 数字 = 末.trim_start_matches("v").parse::<u32>().unwrap_or(0);
    format!("v{}", 数字 + 1)
}

/// 增量基：版本库中「源码-快照 文件数最多」的版本目录（最近完整基线）。
/// 不能用「上一版本目录」作基——增量版本目录只存变化文件（不全），以它为基会把未变文件误判为全量变更。
fn 找最近完整基线(版本库路径: &Path) -> Option<PathBuf> {
    let 条目们 = std::fs::read_dir(版本库路径).ok()?;
    let mut 最佳: Option<(usize, PathBuf)> = None;
    for 条目 in 条目们.flatten() {
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if !名.starts_with("版本-") {
            continue;
        }
        let 快照 = 条目.path().join("源码-快照");
        let Ok(文件数) = 计数文件(&快照) else {
            continue;
        };
        if 文件数 == 0 {
            continue;
        }
        if 最佳.as_ref().map(|(n, _)| 文件数 > *n).unwrap_or(true) {
            最佳 = Some((文件数, 快照));
        }
    }
    最佳.map(|(_, 路径)| 路径)
}

/// 递归统计目录内文件数。
fn 计数文件(目录: &Path) -> std::io::Result<usize> {
    let mut 栈 = vec![目录.to_path_buf()];
    let mut 数 = 0;
    while let Some(当前) = 栈.pop() {
        for 条目 in std::fs::read_dir(&当前)? {
            let 路径 = 条目?.path();
            if 路径.is_dir() {
                栈.push(路径);
            } else {
                数 += 1;
            }
        }
    }
    Ok(数)
}

/// 「版本 存档」命令入口：执行完整闭环，落盘后回传结构化摘要。
pub fn 存档版本() -> String {
    let 根 = 工作区根();
    let 状态 = 状态目录();

    // 1) 首次启动初始化世界状态（幂等：已存在则跳过）。
    let 初始状态 = match 确保状态已初始化(&状态) {
        Ok(状态) => 状态,
        Err(错误) => {
            error!(错误 = %错误, "世界状态初始化失败");
            return format!("存档失败：{错误}");
        }
    };

    // 2) 版本号自动递增 + 定位增量基（版本库中最全的版本快照作最近完整基线）。
    let 历史 = tianting_fu::读版本历史(&状态).unwrap_or_default();
    let 上一版号 = 历史.iter().last().map(|记录| 记录.版本号.clone());
    let 版本号 = 递进版本号(上一版号.as_deref());
    let 版本库路径 = 版本库(&根);
    let 快照目标 = 版本库路径.join(format!("版本-{版本号}")).join("源码-快照");
    let 基目录 = 找最近完整基线(&版本库路径);

    // 3) 增量源码快照（无上一版退化为全量，即 v1 场景）。
    let (文件数, 变更清单) = match tianting_fu::增量快照(&根, 基目录.as_deref(), &快照目标)
    {
        Ok(结果) => 结果,
        Err(错误) => {
            error!("增量快照失败：{错误}");
            return format!("存档失败：{错误}");
        }
    };
    info!(版本号, 文件数, 变更件数 = 变更清单.len(), "增量快照完成");

    // 4) 组装版本记录：改了什么 / 对比上一版。
    let 验收结论们 = 最近验收结论(&状态);
    let 概要 = 验收概要(&验收结论们);
    let 清单文本 = 变更清单
        .iter()
        .map(|(路径, 字节)| format!("{路径}·{字节}B"))
        .collect::<Vec<_>>()
        .join("\n");
    let 对比上一版 = if 变更清单.is_empty() {
        format!("无上一版基线，全量快照（{文件数} 件）")
    } else {
        format!("相对上一版增量 {} 件：\n{清单文本}", 变更清单.len())
    };
    let 改了什么 = if 变更清单.is_empty() {
        "版本存档初始基线（全量快照）".to_string()
    } else {
        format!("本轮迭代 {} 件增量变更；最近验收：{概要}", 变更清单.len())
    };

    // 5) 阶段判定：v1 已存档后为乙，否则甲；并把阶段回写世界状态（切换点幂等升级）。
    let 状态当前 = tianting_fu::读世界状态(&状态)
        .ok()
        .flatten()
        .unwrap_or(初始状态);
    let 阶段 = if 状态当前.v1已存档 {
        tianting_fu::阶段::乙
    } else {
        tianting_fu::阶段::甲
    };

    // 6) 版本记录落盘 + 追加世界状态版本历史（原子写回，v1已存档 保持）。
    let 记录 = tianting_fu::生成版本记录(
        &版本号,
        shihai_fu::当前毫秒(),
        阶段.clone(),
        &改了什么,
        &快照目标.display().to_string(),
        "",
        验收结论们,
        &对比上一版,
    );
    if let Err(错误) = tianting_fu::落盘版本记录(&状态, &记录) {
        error!(错误 = %错误, "版本记录落盘失败");
        return format!("存档失败：{错误}");
    }
    let mut 更新状态 = 状态当前;
    更新状态.阶段 = 阶段;
    更新状态.版本历史.push(记录);
    if let Err(错误) = tianting_fu::写世界状态(&状态, &更新状态) {
        error!(错误 = %错误, "世界状态写回失败");
        return format!("存档失败：{错误}");
    }

    // 7) 摘要回传。
    format!(
        "版本已存档\n版本号：{版本号}\n快照目录：{}\n复制文件：{文件数} 件（增量变更 {} 件）\n版本记录：{}/版本.jsonl\n世界状态：阶段={:?}，v1已存档={}\n",
        快照目标.display(),
        变更清单.len(),
        状态.display(),
        更新状态.阶段,
        更新状态.v1已存档
    )
}

pub fn 回退版本(版本号: &str) -> String {
    let 根 = 工作区根();
    let 快照 = 版本库(&根).join(format!("版本-{版本号}")).join("源码-快照");
    if !快照.exists() {
        warn!(版本号, "版本快照不存在");
        return format!("版本 {版本号} 快照不存在：{}", 快照.display());
    }
    let 回退目标 = 版本库(&根).join("回退区");
    match tianting_fu::回退版本(&快照, &回退目标) {
        Ok(数) => {
            info!(版本号, 数, "版本回退完成");
            format!(
                "版本已回退到回退区\n恢复文件：{数} 件\n目标：{}",
                回退目标.display()
            )
        }
        Err(错误) => {
            error!(版本号, "版本回退失败：{错误}");
            format!("回退失败：{错误}")
        }
    }
}
