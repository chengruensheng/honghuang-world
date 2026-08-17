//! 即时-应答-园：界主对话发言命令实现（对话 发言 → 鸿钧对话循环）。
//! 免令牌（读命令区）：对话是界主与鸿钧的日常交流，不应被写命令令牌拦住；
//! 发布任务的能力由鸿钧对话循环内部走主政链路（设计稿 §1.5.5 拍板）。

use crate::{读模型配置, 打开存储, 工作区根};
use rizhi_fu::info;

/// 「对话 发言」命令：界主一句话 → 鸿钧答复。
pub fn 对话发言(内容: &str) -> String {
    if 内容.trim().is_empty() {
        return "请说点什么".to_string();
    }
    let 配置 = 读模型配置();
    let 存储 = 打开存储();
    let mut 调度 = daoshu_fu::任务调度::新(配置.clone(), 工作区根());
    info!(内容 = %内容.chars().take(60).collect::<String>(), "对话发言受理");
    tianting_fu::界主发言(内容, &配置, &存储, &mut 调度)
}

/// 「对话 历史」命令：倒序展示最近 20 条对话记录（界主 ↔ 鸿钧，含任务汇报）。
pub fn 对话历史() -> String {
    let 路径 = 工作区根().join(".上下文").join("状态").join("对话.jsonl");
    let Ok(内容) = std::fs::read_to_string(&路径) else {
        return "（无对话记录）".to_string();
    };
    let 行们: Vec<&str> = 内容.lines().filter(|行| !行.trim().is_empty()).collect();
    if 行们.is_empty() {
        return "（无对话记录）".to_string();
    }
    let 尾部 = 行们.iter().rev().take(20).rev();
    let 段们 = 尾部
        .map(|行| {
            serde_json::from_str::<serde_json::Value>(行)
                .ok()
                .map(|值| {
                    let 发送者 = 值["发送者"].as_str().unwrap_or("?");
                    let 文本 = 值["文本"].as_str().unwrap_or("");
                    let 可见 = 值["可见"]
                        .as_array()
                        .map(|数组| {
                            数组
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<_>>()
                                .join(",")
                        })
                        .unwrap_or_default();
                    format!("【{发送者}】（可见：{可见}）{文本}")
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    format!("对话历史（最近 {} 条）\n{}", 段们.len(), 段们.join("\n"))
}
