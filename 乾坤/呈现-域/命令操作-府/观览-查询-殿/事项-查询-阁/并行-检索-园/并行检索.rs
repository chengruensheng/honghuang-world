//! 事项查询：想法 / 要求 / 设计 / 验收 列表与详情（读状态目录 jsonl）

use crate::状态目录;
use rizhi_fu::{debug, error, warn};

pub fn 事项列表(域: &str) -> String {
    match 域 {
        "想法" => {
            let 队列 = tianting_fu::落盘队列::<tianting_fu::想法>::打开(状态目录().join("想法.jsonl"));
            match 队列.读全部() {
                Ok(项们) => {
                    debug!(域, 条数 = 项们.len(), "事项列表已读");
                    if 项们.is_empty() {
                        return "想法列表\n（空）".to_string();
                    }
                    let mut 行 = format!("想法列表（{} 条）\n", 项们.len());
                    for 项 in 项们.iter().rev() {
                        行.push_str(&format!("{} {} · {:?}\n", 项.id, 项.内容, 项.状态));
                    }
                    行
                }
                Err(错误) => {
                    error!(域, "读事项列表失败：{错误}");
                    format!("读取失败：{错误}")
                }
            }
        }
        "要求" => {
            let 队列 = tianting_fu::落盘队列::<tianting_fu::要求书>::打开(状态目录().join("要求.jsonl"));
            match 队列.读全部() {
                Ok(项们) => {
                    debug!(域, 条数 = 项们.len(), "事项列表已读");
                    if 项们.is_empty() {
                        return "要求列表\n（空）".to_string();
                    }
                    let mut 行 = format!("要求列表（{} 条）\n", 项们.len());
                    for 项 in 项们.iter().rev() {
                        行.push_str(&format!("{} {} · {:?}\n", 项.id, 项.方向, 项.状态));
                    }
                    行
                }
                Err(错误) => {
                    error!(域, "读事项列表失败：{错误}");
                    format!("读取失败：{错误}")
                }
            }
        }
        "设计" => {
            let 队列 = tianting_fu::落盘队列::<tianting_fu::设计方案>::打开(状态目录().join("设计.jsonl"));
            match 队列.读全部() {
                Ok(项们) => {
                    debug!(域, 条数 = 项们.len(), "事项列表已读");
                    if 项们.is_empty() {
                        return "设计列表\n（空）".to_string();
                    }
                    let mut 行 = format!("设计列表（{} 条）\n", 项们.len());
                    for 项 in 项们.iter().rev() {
                        行.push_str(&format!("{} 拆解 {} 项\n", 项.要求id, 项.拆解.len()));
                    }
                    行
                }
                Err(错误) => {
                    error!(域, "读事项列表失败：{错误}");
                    format!("读取失败：{错误}")
                }
            }
        }
        "验收" => {
            let 队列 = tianting_fu::落盘队列::<tianting_fu::验收回执>::打开(状态目录().join("验收.jsonl"));
            match 队列.读全部() {
                Ok(项们) => {
                    debug!(域, 条数 = 项们.len(), "事项列表已读");
                    if 项们.is_empty() {
                        return "验收历史\n（空）".to_string();
                    }
                    let mut 行 = format!("验收历史（{} 条）\n", 项们.len());
                    for 项 in 项们.iter().rev() {
                        行.push_str(&format!("{} · {:?}\n", 项.要求id, 项.结论));
                    }
                    行
                }
                Err(错误) => {
                    error!(域, "读事项列表失败：{错误}");
                    format!("读取失败：{错误}")
                }
            }
        }
        _ => {
            warn!(域, "未知事项域");
            format!("未知事项域：{域}")
        }
    }
}

pub fn 事项详情(域: &str, id: &str) -> String {
    let 队列路径 = match 域 {
        "想法" => 状态目录().join("想法.jsonl"),
        "要求" => 状态目录().join("要求.jsonl"),
        "设计" => 状态目录().join("设计.jsonl"),
        "验收" => 状态目录().join("验收.jsonl"),
        _ => return format!("未知事项域：{域}"),
    };
    // 简化：读文件，找含 id 的原始行
    let 内容 = std::fs::read_to_string(&队列路径).unwrap_or_default();
    for 行 in 内容.lines() {
        if 行.contains(id) {
            return format!("{域}详情\n{行}");
        }
    }
    format!("{域} {id} 未找到")
}
