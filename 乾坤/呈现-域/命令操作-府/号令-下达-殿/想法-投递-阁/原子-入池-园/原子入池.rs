//! 想法投递：界主想法 → 鸿钧主循环（运行一轮：要求→设计→实现→验收→定档）

use crate::{读模型配置, 状态目录, 打开存储, 工作区根};

pub fn 投递想法(内容: &str) -> String {
    if 内容.is_empty() {
        return "想法为空，请提供内容".to_string();
    }
    let 配置 = 读模型配置();
    let 存储 = 打开存储();
    let mut 调度 = daoshu_fu::任务调度::新(配置.clone(), 工作区根());
    let 想法 = tianting_fu::想法 {
        id: format!("想法-{}", shihai_fu::当前毫秒()),
        内容: 内容.to_string(),
        时间: shihai_fu::当前毫秒(),
        状态: tianting_fu::想法状态::未处理,
    };

    // 想法入池
    let 想法池 = tianting_fu::落盘队列::<tianting_fu::想法>::打开(状态目录().join("想法.jsonl"));
    if let Err(错误) = 想法池.入队(&想法) {
        return format!("想法入池失败：{错误}");
    }
    let _ = 存储.写记录(&shihai_fu::记录::新("事件", &format!("想法投递：{}", 想法.内容), "号令", "代码"));

    match tianting_fu::运行一轮(&想法, &配置, &存储, &mut 调度) {
        Ok(回执) => {
            let 验收 = tianting_fu::落盘队列::<tianting_fu::验收回执>::打开(状态目录().join("验收.jsonl"));
            let _ = 验收.入队(&回执);
            let _ = 存储.写记录(&shihai_fu::记录::新(
                "事件",
                &format!("执行完成：要求 {} · 结论 {:?}", 回执.要求id, 回执.结论),
                "号令",
                "代码",
            ));
            format!(
                "想法已执行\n要求：{}\n结论：{:?}\n产物：{} 件\n耗时：{:.2} 秒",
                回执.要求id, 回执.结论, 回执.产物.len(), 回执.耗时秒
            )
        }
        Err(错误) => {
            let _ = 存储.写记录(&shihai_fu::记录::新("事件", &format!("执行失败：{错误}"), "号令", "代码"));
            format!("想法执行失败：{错误}")
        }
    }
}
