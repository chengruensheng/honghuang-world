//! 对话-循环-园：界主发言 → 落对话记录 → 意图判别 → 分流（闲聊/发布任务/追问/干预/点名）。
//! 设计稿 §1.5.5：鸿钧 = 界主的对话伙伴 + 任务总控；界主只跟鸿钧说话。
//! 消息可见性：界主发言非@仅鸿钧可见、@点名才带上该角色；鸿钧答复在界主-鸿钧之间。

use crate::类型_定义_殿::{想法, 想法状态};
use crate::{判别, 判别结果, 对话意图, 摘要主政回执};
use daoshu_fu::任务调度;
use moxing_fu::{调用模型, 对话消息, 模型配置, 精简上限};
use rizhi_fu::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

/// 对话记录（落盘 .上下文/状态/对话.jsonl，追加式事实源）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct 对话记录 {
    pub 发送者: String,
    pub 文本: String,
    /// 可见角色集合（阶段 1 最小实现：界主发言=[鸿钧]+@点名；鸿钧答复=[界主,鸿钧]）。
    pub 可见: Vec<String>,
    pub 时间戳: u64,
}

/// 状态目录：工作区根下的 .上下文/状态（与 世界运行.rs 同款，本园复制以保持跨府引用只走 lib 根符号的边界）。
fn 状态目录() -> std::path::PathBuf {
    let 根 = std::env::var("WORLD_WORKSPACE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    根.join(".上下文").join("状态")
}

/// 界主发言入口：判别意图 → 分流 → 返回鸿钧答复（答复同时落对话记录）。
pub fn 界主发言(
    消息: &str,
    配置: &模型配置,
    存储: &shihai_fu::模型存储,
    调度: &mut 任务调度,
) -> String {
    let 消息 = 消息.trim();
    if 消息.is_empty() {
        return "请说点什么".to_string();
    }
    let 判别结果 = 判别(消息, 配置);
    // 界主消息可见性：默认仅鸿钧；@点名才带上该角色。
    let mut 可见 = vec!["鸿钧".to_string()];
    if let Some(角色) = &判别结果.点名角色 {
        if !可见.contains(角色) {
            可见.push(角色.clone());
        }
    }
    落对话记录("界主", 消息, &可见);

    let 答复 = match 判别结果.意图 {
        对话意图::闲聊 => 闲聊回复(消息, 配置),
        对话意图::发布任务 => 发布任务(消息, &判别结果, 配置, 存储, 调度),
        对话意图::追问进度 => 追问进度回复(),
        对话意图::中途干预 => {
            "中途干预（停止/打回/加急）在阶段 3 开放：当前可通过「号令 想法 投递」走完当前任务。".to_string()
        }
        对话意图::点名角色 => {
            let 角色 = 判别结果.点名角色.as_deref().unwrap_or("该角色");
            format!("@{角色} 已收到你的点名。消息直达在阶段 3 开放，当前已记录本条。")
        }
    };
    落对话记录("鸿钧", &答复, &["界主".to_string(), "鸿钧".to_string()]);
    答复
}

/// 落一条对话记录（追加写，不重写历史）。
fn 落对话记录(发送者: &str, 文本: &str, 可见: &[String]) {
    let 记录 = 对话记录 {
        发送者: 发送者.to_string(),
        文本: 文本.to_string(),
        可见: 可见.to_vec(),
        时间戳: shihai_fu::当前毫秒(),
    };
    let 路径 = 状态目录().join("对话.jsonl");
    if let Ok(行) = serde_json::to_string(&记录) {
        if let Ok(mut 文件) = OpenOptions::new().create(true).append(true).open(&路径) {
            let _ = writeln!(文件, "{行}");
        }
    }
}

/// 追问进度分流：机械汇总真实回执（验收.jsonl 尾部 + 要求.jsonl 状态），不调 LLM、不编造。
fn 追问进度回复() -> String {
    let 目录 = 状态目录();
    // 要求现状：读要求.jsonl 全部，按状态统计。
    let 要求队列 = crate::落盘队列::<crate::要求书>::打开(目录.join("要求.jsonl"));
    let 要求们 = 要求队列.读全部().unwrap_or_default();
    let 状态数 = 要求们.iter().fold(std::collections::BTreeMap::new(), |mut 表, 要求| {
        *表.entry(format!("{:?}", 要求.状态)).or_insert(0usize) += 1;
        表
    });
    let 状态段 = if 状态数.is_empty() {
        "（无要求记录）".to_string()
    } else {
        状态数
            .iter()
            .map(|(状态, 数)| format!("{状态} {数} 条"))
            .collect::<Vec<_>>()
            .join("；")
    };
    // 最近验收：验收.jsonl 尾部 5 条。
    let 验收队列 = crate::落盘队列::<crate::终裁回执>::打开(目录.join("验收.jsonl"));
    let 验收们 = 验收队列.读全部().unwrap_or_default();
    let 尾部 = 验收们.iter().rev().take(5).rev();
    let 验收段 = if 验收们.is_empty() {
        "（无验收记录）".to_string()
    } else {
        尾部
            .map(|回执| format!("{} {:?}", 回执.验收.要求id, 回执.验收.结论))
            .collect::<Vec<_>>()
            .join("；")
    };
    format!("当前世界状态\n要求：{状态段}\n最近验收：{验收段}")
}

/// 闲聊分流：鸿钧人格直接回应（轻量调用，失败兜底不阻塞对话）。
fn 闲聊回复(消息: &str, 配置: &模型配置) -> String {    let 提示 = format!(
        "你是鸿钧，天层主政之神，界主的对话伙伴。界主对你说：{消息}\n\
请自然回应，1-3 句即可。项目相关的话题可以简要说说你的看法；与项目无关的轻松回应。"
    );
    match 调用模型(配置, &[对话消息::用户(&提示)], 精简上限) {
        Ok((回复, _)) => {
            let 回复 = 回复.trim().to_string();
            if 回复.is_empty() { "在听，请继续说。".to_string() } else { 回复 }
        }
        Err(错误) => {
            warn!(错误 = %错误, "闲聊回复调用失败，兜底回应");
            "在听，请继续说。".to_string()
        }
    }
}

/// 发布任务分流：构造想法 → 入池 → 主政一轮（要求→设计→实现→验收→定档）→ 汇总摘要。
/// 与「想法 投递」命令同链路；验收.jsonl 落盘完整终裁回执，想法状态按汇总结论推进。
fn 发布任务(
    消息: &str,
    判别: &判别结果,
    配置: &模型配置,
    存储: &shihai_fu::模型存储,
    调度: &mut 任务调度,
) -> String {
    let 内容 = 拼装任务文本(消息, 判别);
    let 想法 = 想法 {
        id: format!("想法-{}", shihai_fu::当前毫秒()),
        内容,
        时间: shihai_fu::当前毫秒(),
        状态: 想法状态::未处理,
    };
    let 想法路径 = 状态目录().join("想法.jsonl");
    let 想法池 = crate::落盘队列::<想法>::打开(想法路径.clone());
    if let Err(错误) = 想法池.入队(&想法) {
        error!(想法id = %想法.id, "想法入池失败：{错误}");
        return format!("想法入池失败：{错误}");
    }
    info!(想法id = %想法.id, "对话发布任务，想法已受理");

    match crate::主政一轮(&想法, 配置, 存储, 调度) {
        Ok(回执) => {
            let 验收 = crate::落盘队列::<crate::终裁回执>::打开(状态目录().join("验收.jsonl"));
            for 回执 in &回执.回执们 {
                let _ = 验收.入队(回执);
            }
            let _ = 存储.写记录(&shihai_fu::记录::新(
                "事件",
                &format!("对话发布任务完成：子要求 {} 个 · 定档 {} 个 · 结论 {:?}", 回执.子要求数, 回执.定档数, 回执.结论),
                "鸿钧",
                "代码",
            ));
            let 新状态 = match 回执.结论 {
                crate::验收结论::通过 => 想法状态::已化为要求,
                crate::验收结论::打回 => 想法状态::已打回,
            };
            if let Err(错误) = 推进想法状态(&想法.id, 新状态) {
                warn!(想法id = %想法.id, "推进想法状态失败：{错误}");
            }
            format!("{}\n{}", 摘要主政回执(&回执), "（鸿钧）详情可追问，阶段 2 开放。")
        }
        Err(错误) => {
            error!(想法id = %想法.id, "任务执行失败：{错误}");
            let _ = 存储.写记录(&shihai_fu::记录::新("事件", &format!("对话任务执行失败：{错误}"), "鸿钧", "代码"));
            if let Err(推进错误) = 推进想法状态(&想法.id, 想法状态::已打回) {
                warn!(想法id = %想法.id, "推进想法状态失败：{推进错误}");
            }
            format!("任务执行失败：{错误}")
        }
    }
}

/// 拼装任务文本：方向 + 验收标准 + 涉及路径（与 需求拆分 的 合成文本 同思路，路径明文写入防模型丢失）。
fn 拼装任务文本(消息: &str, 判别: &判别结果) -> String {
    let mut 文本 = if 判别.方向.is_empty() { 消息.to_string() } else { 判别.方向.clone() };
    if !判别.验收标准.is_empty() {
        文本.push_str(&format!("。验收标准：{}", 判别.验收标准));
    }
    if !判别.涉及路径.is_empty() {
        文本.push_str(&format!("。涉及路径：{}", 判别.涉及路径.join("、")));
    }
    文本
}

/// 推进想法状态：读全部 → 改目标 → 原子重写（防目标状态被覆盖，与 想法 投递 命令同款）。
fn 推进想法状态(目标id: &str, 新状态: 想法状态) -> Result<(), String> {
    let 想法路径 = 状态目录().join("想法.jsonl");
    let 队列 = crate::落盘队列::<想法>::打开(想法路径.clone());
    let mut 项们 = 队列.读全部().map_err(|错误| format!("读想法队列失败: {错误}"))?;
    let mut 命中 = false;
    for 项 in 项们.iter_mut() {
        if 项.id == 目标id {
            项.状态 = 新状态.clone();
            命中 = true;
            break;
        }
    }
    if !命中 {
        return Err(format!("未找到目标想法：{目标id}"));
    }
    let 临时路径 = 想法路径.with_extension("jsonl.tmp");
    let mut 行们 = Vec::with_capacity(项们.len());
    for 项 in &项们 {
        let 行 = serde_json::to_string(项).map_err(|错误| format!("序列化想法失败: {错误}"))?;
        行们.push(行);
    }
    let 内容 = if 行们.is_empty() { String::new() } else { format!("{}\n", 行们.join("\n")) };
    std::fs::write(&临时路径, &内容).map_err(|错误| format!("写临时文件失败: {错误}"))?;
    std::fs::rename(&临时路径, &想法路径).map_err(|错误| format!("原子改名失败: {错误}"))?;
    info!(目标id, 新状态 = ?新状态, "想法状态已推进");
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 拼装任务文本_方向为空用原话() {
        let 判别 = 判别结果 {
            意图: 对话意图::发布任务,
            方向: String::new(),
            验收标准: String::new(),
            涉及路径: vec![],
            点名角色: None,
        };
        assert_eq!(拼装任务文本("给我写个测试", &判别), "给我写个测试");
    }

    #[test]
    fn 拼装任务文本_带验收与路径() {
        let 判别 = 判别结果 {
            意图: 对话意图::发布任务,
            方向: "新增世界 昼夜 命令".to_string(),
            验收标准: "cargo test 通过".to_string(),
            涉及路径: vec!["乾坤/呈现-域".to_string()],
            点名角色: None,
        };
        let 文本 = 拼装任务文本("忽略", &判别);
        assert!(文本.contains("新增世界 昼夜 命令"));
        assert!(文本.contains("cargo test 通过"));
        assert!(文本.contains("乾坤/呈现-域"));
    }

    /// 兼容回归：真实验收.jsonl（含旧六维历史记录）可被 终裁回执 全量反序列化（追问进度/流水观览依赖）。
    #[test]
    #[ignore = "需真实工作区"]
    fn 兼容_真实验收jsonl全量可解析() {
        std::env::set_var("WORLD_WORKSPACE_ROOT", "D:\\洪荒 - 世界");
        let 目录 = 状态目录();
        let 队列 = crate::落盘队列::<crate::终裁回执>::打开(目录.join("验收.jsonl"));
        let 回执们 = 队列.读全部().expect("历史验收记录（含旧六维）应全量可解析");
        assert!(!回执们.is_empty(), "真实验收.jsonl 不应为空");
    }
}
