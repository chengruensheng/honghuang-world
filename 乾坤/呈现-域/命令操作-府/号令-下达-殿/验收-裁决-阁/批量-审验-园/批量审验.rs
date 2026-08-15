//! 验收裁决：构造验收回执 → 入验收历史

use crate::状态目录;

pub fn 裁决验收(要求id: &str, 结论: &str, 意见: &str) -> String {
    let 结论值 = match 结论 {
        "通过" => tianting_fu::验收结论::通过,
        "打回" => tianting_fu::验收结论::打回,
        _ => return format!("结论需为 通过|打回，当前：{结论}"),
    };
    let 回执 = tianting_fu::验收回执 {
        要求id: 要求id.to_string(),
        结论: 结论值,
        验收意见: Some(意见.to_string()),
        产物: Vec::new(),
        耗时秒: 0.0,
    };
    let 队列 = tianting_fu::落盘队列::<tianting_fu::验收回执>::打开(状态目录().join("验收.jsonl"));
    match 队列.入队(&回执) {
        Ok(_) => format!("验收已裁决\n要求id：{要求id}\n结论：{结论}\n意见：{意见}\n已入验收历史"),
        Err(错误) => format!("入队失败：{错误}"),
    }
}
