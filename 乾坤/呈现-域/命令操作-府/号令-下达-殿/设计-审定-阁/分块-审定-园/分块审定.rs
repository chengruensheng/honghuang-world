//! 设计审定：要求 → 模板设计 → 设计方案上呈 / 确认

use crate::状态目录;
use rizhi_fu::{error, info, warn};

pub fn 上呈设计(要求id: &str, 文件: &str) -> String {
    let 队列 = tianting_fu::落盘队列::<tianting_fu::要求书>::打开(状态目录().join("要求.jsonl"));
    let 要求们 = match 队列.读全部() {
        Ok(项们) => 项们,
        Err(错误) => {
            error!(要求id, "读要求队列失败：{错误}");
            return format!("读要求队列失败：{错误}")
        }
    };
    let 要求 = match 要求们.iter().find(|项| 项.id == 要求id) {
        Some(项) => 项.clone(),
        None => {
            warn!(要求id, "要求不在队列中");
            return format!("要求 {要求id} 不在队列中（先用「要求 化为」）")
        }
    };
    let 方案 = tianting_fu::模板设计(&要求);
    let 设计队列 = tianting_fu::落盘队列::<tianting_fu::设计方案>::打开(状态目录().join("设计.jsonl"));
    match 设计队列.入队(&方案) {
        Ok(_) => {
            info!(要求id, 拆解数 = 方案.拆解.len(), "设计方案已上呈");
            format!(
                "设计方案已上呈\n要求id：{要求id}\n设计文件：{文件}\n拆解：{} 项\n状态：待确认",
                方案.拆解.len()
            )
        }
        Err(错误) => {
            error!(要求id, "设计入队失败：{错误}");
            format!("设计入队失败：{错误}")
        }
    }
}

pub fn 确认设计(要求id: &str, 结论: &str, 意见: &str) -> String {
    match 结论 {
        "通过" => {
            info!(要求id, "设计已确认通过");
            format!("设计已确认\n要求id：{要求id}\n结论：通过\n意见：{意见}")
        }
        "打回" => {
            warn!(要求id, "设计已打回");
            format!("设计已打回\n要求id：{要求id}\n结论：打回\n意见：{意见}\n状态：设计中")
        }
        _ => {
            warn!(要求id, 结论, "设计结论非法");
            format!("结论需为 通过|打回，当前：{结论}")
        }
    }
}
