//! 流水观览：读识海「事件」格位（执行过程）

use crate::{打开存储, 状态目录};
use rizhi_fu::{debug, warn};

pub fn 流水列表() -> String {
    let 存储 = 打开存储();
    match 存储.读格位("事件") {
        Ok(记录们) => {
            debug!(条数 = 记录们.len(), "流水列表已读");
            if 记录们.is_empty() {
                return "流水列表\n（空）".to_string();
            }
            let mut 行 = format!("流水列表（{} 条事件）\n", 记录们.len());
            for 记录 in 记录们.iter().rev().take(30) {
                行.push_str(&format!("- {}\n", 记录.内容));
            }
            行
        }
        Err(错误) => {
            warn!("流水列表读取失败：{错误}");
            format!("读取失败：{错误}")
        }
    }
}

pub fn 流水跟踪(会话id: &str, 全文: bool) -> String {
    let _ = 全文;
    let 存储 = 打开存储();
    match 存储.读格位("事件") {
        Ok(记录们) => {
            debug!(会话id, 条数 = 记录们.len(), "流水跟踪已读");
            let mut 行 = format!("会话 {会话id} · 执行过程（{} 条事件）\n", 记录们.len());
            for 记录 in 记录们.iter().rev().take(30) {
                行.push_str(&format!("- {}\n", 记录.内容));
            }
            行
        }
        Err(错误) => {
            warn!(会话id, "流水跟踪读取失败：{错误}");
            format!("读取失败：{错误}")
        }
    }
}

/// 全流程总览：管道水位 + 最近事件（号令默认视图）
pub fn 全流程总览() -> String {
    let 想法池 = tianting_fu::落盘队列::<tianting_fu::想法>::打开(状态目录().join("想法.jsonl"));
    let 要求队列 = tianting_fu::落盘队列::<tianting_fu::要求书>::打开(状态目录().join("要求.jsonl"));
    let 验收历史 = tianting_fu::落盘队列::<tianting_fu::验收回执>::打开(状态目录().join("验收.jsonl"));
    let 想法数 = 想法池.水位().unwrap_or(0);
    let 要求数 = 要求队列.水位().unwrap_or(0);
    let 验收数 = 验收历史.水位().unwrap_or(0);
    debug!(想法数, 要求数, 验收数, "全流程总览已读");

    let 存储 = 打开存储();
    let 最近 = 存储.读格位("事件").ok().map(|记录们| {
        记录们.iter().rev().take(5).map(|记录| format!("  - {}", 记录.内容)).collect::<Vec<_>>().join("\n")
    }).unwrap_or_default();

    format!(
        "全流程总览\n想法池[{想法数}] → 要求队列[{要求数}] → 验收历史[{验收数}]\n最近事件：\n{最近}\n（号令 帮助 查看全部命令）"
    )
}

/// 跟随：不带会话id = 全流程总览；带会话id = 单会话事件
pub fn 流水跟随(会话id: &str) -> String {
    if 会话id.is_empty() {
        全流程总览()
    } else {
        流水跟踪(会话id, false)
    }
}
