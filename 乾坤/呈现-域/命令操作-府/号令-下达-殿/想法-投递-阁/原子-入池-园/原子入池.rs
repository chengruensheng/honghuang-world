//! 想法投递：界主想法 → 鸿钧主循环（运行一轮：要求→设计→实现→验收→定档）。
//! 执行完成后按验收结论推进想法状态（未处理→已化为要求/已打回），
//! 落盘写回想法.jsonl 对应记录，防同一意图被反复重复投递。

use crate::{打开存储, 状态目录};
use rizhi_fu::{error, info, warn};
use shihai_fu::世界结果;
use shihai_fu::世界错误::世界错误;
use std::fs;

pub fn 投递想法(内容: &str) -> String {
    if 内容.is_empty() {
        warn!("想法为空");
        return "想法为空，请提供内容".to_string();
    }
    let 存储 = 打开存储();
    let 想法 = tianting_fu::想法 {
        id: format!("想法-{}", shihai_fu::当前毫秒()),
        内容: 内容.to_string(),
        时间: shihai_fu::当前毫秒(),
        状态: tianting_fu::想法状态::未处理,
    };
    info!(想法id = %想法.id, "想法已受理");

    // 初始化全局状态共享（供主循环写入当前想法id/要求id，观览查询读取）
    let _ = zhuangtai_fu::初始化全局状态();
    // 初始化全局插件上下文（识海→道术→天庭，按依赖顺序）
    let ctx = chajian_fu::初始化全局上下文(vec![
        Box::new(shihai_fu::识海插件),
        Box::new(daoshu_fu::道术插件),
        Box::new(tianting_fu::天庭插件),
    ]);
    info!(
        想法id = %想法.id,
        已注册 = ?ctx.已注册(),
        "三府插件已注册"
    );

    // 想法入池
    let 想法路径 = 状态目录().join("想法.jsonl");
    let 想法池 = tianting_fu::落盘队列::<tianting_fu::想法>::打开(想法路径.clone());
    if let Err(错误) = 想法池.入队(&想法) {
        error!(想法id = %想法.id, "想法入池失败：{错误}");
        return format!("想法入池失败：{错误}");
    }
    let _ = 存储.写记录(&shihai_fu::记录::新(
        "事件",
        &format!("想法投递：{}", 想法.内容),
        "号令",
        "代码",
    ));

    let 天庭服务 = match ctx.查找服务::<std::sync::Arc<dyn tianting_fu::天庭服务>>() {
        Some(天庭) => 天庭,
        None => {
            error!(想法id = %想法.id, "天庭服务未注册");
            return "天庭服务未注册".to_string();
        }
    };
    match 天庭服务.调度要求(&想法) {
        Ok(主政回执) => {
            // 验收.jsonl 落盘完整终裁回执（每个子要求/单要求各一条，含六准圣意见/终裁依据/用量），
            // 历史读取方按旧验收回执解析自动兼容。
            let 验收 = tianting_fu::落盘队列::<tianting_fu::终裁回执>::打开(
                状态目录().join("验收.jsonl"),
            );
            for 回执 in &主政回执.回执们 {
                let _ = 验收.入队(回执);
            }
            let _ = 存储.写记录(&shihai_fu::记录::新(
                "事件",
                &format!(
                    "执行完成：子要求 {} 个 · 定档 {} 个 · 结论 {:?}",
                    主政回执.子要求数, 主政回执.定档数, 主政回执.结论
                ),
                "号令",
                "代码",
            ));
            info!(想法id = %想法.id, 子要求数 = 主政回执.子要求数, 定档数 = 主政回执.定档数, 结论 = ?主政回执.结论, "想法执行完成");
            // 推进想法状态：按想法级汇总结论推进（全部通过→已化为要求 / 任一打回→已打回）。
            let 新状态 = match 主政回执.结论 {
                tianting_fu::验收结论::通过 => tianting_fu::想法状态::已化为要求,
                tianting_fu::验收结论::打回 => tianting_fu::想法状态::已打回,
            };
            if let Err(错误) = 推进想法状态(&想法.id, 新状态) {
                warn!(想法id = %想法.id, "推进想法状态失败：{错误}");
            }
            let 明细 = 主政回执
                .回执们
                .iter()
                .map(|回执| format!("{} {:?}", 回执.验收.要求id, 回执.验收.结论))
                .collect::<Vec<_>>()
                .join("；");
            let 产物数: usize = 主政回执
                .回执们
                .iter()
                .map(|回执| 回执.验收.产物.len())
                .sum();
            let 耗时秒: f64 = 主政回执.回执们.iter().map(|回执| 回执.验收.耗时秒).sum();
            format!(
                "想法已执行\n子要求：{} 个\n定档：{} 个\n结论：{:?}\n明细：{}\n产物：{} 件\n耗时：{:.2} 秒",
                主政回执.子要求数, 主政回执.定档数, 主政回执.结论, 明细, 产物数, 耗时秒
            )
        }
        Err(错误) => {
            let _ = 存储.写记录(&shihai_fu::记录::新(
                "事件",
                &format!("执行失败：{错误}"),
                "号令",
                "代码",
            ));
            error!(想法id = %想法.id, "想法执行失败：{错误}");
            // 推进想法状态：执行失败按已打回处理，防止同一意图被反复重复投递。
            if let Err(推进错误) = 推进想法状态(&想法.id, tianting_fu::想法状态::已打回)
            {
                warn!(想法id = %想法.id, "推进想法状态失败：{推进错误}");
            }
            format!("想法执行失败：{错误}")
        }
    }
}

/// 推进想法状态：读全部 → 改目标 → 重写整个想法.jsonl。
/// 落盘队列无原地更新接口，须在原子入池园内做读改写（防目标状态被覆盖）。
fn 推进想法状态(目标id: &str, 新状态: tianting_fu::想法状态) -> 世界结果<()> {
    let 想法路径 = 状态目录().join("想法.jsonl");
    let 队列 = tianting_fu::落盘队列::<tianting_fu::想法>::打开(想法路径.clone());
    let mut 项们 = 队列
        .读全部()
        .map_err(|错误| 世界错误::from(format!("读想法队列失败: {错误}")))?;
    let mut 命中 = false;
    for 项 in 项们.iter_mut() {
        if 项.id == 目标id {
            项.状态 = 新状态.clone();
            命中 = true;
            break;
        }
    }
    if !命中 {
        return Err(世界错误::from(format!("未找到目标想法：{目标id}")));
    }
    // 写临时文件再原子改名，避免写一半导致 jsonl 损坏。
    let 临时路径 = 想法路径.with_extension("jsonl.tmp");
    let mut 行们 = Vec::with_capacity(项们.len());
    for 项 in &项们 {
        let 行 = serde_json::to_string(项)
            .map_err(|错误| 世界错误::from(format!("序列化想法失败: {错误}")))?;
        行们.push(行);
    }
    let 内容 = if 行们.is_empty() {
        String::new()
    } else {
        format!("{}\n", 行们.join("\n"))
    };
    fs::write(&临时路径, &内容)
        .map_err(|错误| 世界错误::from(format!("写临时文件失败: {错误}")))?;
    fs::rename(&临时路径, &想法路径)
        .map_err(|错误| 世界错误::from(format!("原子改名失败: {错误}")))?;
    info!(目标id, 新状态 = ?新状态, "想法状态已推进");
    Ok(())
}
