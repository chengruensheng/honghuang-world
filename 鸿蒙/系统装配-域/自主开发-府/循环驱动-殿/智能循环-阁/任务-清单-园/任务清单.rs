use hm_error::{Error, Result};

/// 任务清单项的载荷键常量（与工具 schema 的 property 名一致，中文为规范键）
pub const 清单项键_内容: &str = "内容";
pub const 清单项键_状态: &str = "状态";
/// 任务项状态值常量（与 schema 的 enum 一致，中文为规范值）
pub const 状态_待办: &str = "待办";
pub const 状态_进行中: &str = "进行中";
pub const 状态_已完成: &str = "已完成";

/// 任务清单项：LLM 规划的多步任务条目（仅内存态，不落盘）
#[derive(Clone, Debug, PartialEq)]
pub struct 任务项 {
    pub 内容: String,
    pub 状态: 任务状态,
}

/// 任务项状态：待办 / 进行中 / 已完成
#[derive(Clone, Debug, PartialEq)]
pub enum 任务状态 {
    待办,
    进行中,
    已完成,
}

/// 从工具参数 JSON 解析任务清单（数组，每项须含「内容」，状态可缺省为待办）
pub fn 解析任务清单(清单值: &serde_json::Value) -> Result<Vec<任务项>> {
    let 数组 = 清单值
        .as_array()
        .ok_or_else(|| Error::反序列化("任务清单必须是数组".into()))?;
    let mut 结果 = Vec::new();
    for 项 in 数组 {
        let 内容 = 项
            .get(清单项键_内容)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::缺少参数("清单项.内容".into()))?
            .to_string();
        let 状态 = 解析状态(项.get(清单项键_状态).and_then(|v| v.as_str()));
        结果.push(任务项 { 内容, 状态 });
    }
    Ok(结果)
}

/// 解析状态字符串（中文规范 + 英文兼容别名，未知值回退为待办）
fn 解析状态(状态: Option<&str>) -> 任务状态 {
    match 状态 {
        Some(状态_进行中) | Some("in_progress") => 任务状态::进行中,
        Some(状态_已完成) | Some("completed") => 任务状态::已完成,
        _ => 任务状态::待办,
    }
}

/// 将任务清单格式化为回填给 LLM 的可读文本
pub fn 格式化清单(清单: &[任务项]) -> String {
    if 清单.is_empty() {
        return "（任务清单为空）".to_string();
    }
    清单
        .iter()
        .map(|项| {
            let 状态 = match 项.状态 {
                任务状态::待办 => 状态_待办,
                任务状态::进行中 => 状态_进行中,
                任务状态::已完成 => 状态_已完成,
            };
            format!("[{}] {}", 状态, 项.内容)
        })
        .collect::<Vec<_>>()
        .join("\n")
}