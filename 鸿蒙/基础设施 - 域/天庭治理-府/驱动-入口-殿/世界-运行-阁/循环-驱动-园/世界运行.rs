//! 循环 - 驱动 - 园：鸿钧主循环（想法 → 要求 → 设计 → 实现 → 验收 → 定档）。

use crate::类型_定义_殿::{验收结论, 验收回执, 想法};
use daoshu_fu::任务调度;
use moxing_fu::模型配置;

/// 运行一轮：把一个想法走完「要求 → 设计 → 实现 → 验收 → 定档」。
pub fn 运行一轮(
    想法: &想法,
    配置: &模型配置,
    存储: &shihai_fu::模型存储,
    调度: &mut 任务调度,
) -> Result<验收回执, String> {
    let 背景 = shihai_fu::拼装投影(存储, &shihai_fu::全部格位(), 8000).unwrap_or_default();
    let 要求 = crate::解析想法("要求-1", &想法.内容, &背景, 配置)?;

    let 方案 = crate::模板设计(&要求);

    if crate::确认设计(&方案) == 验收结论::打回 {
        return Err("设计被打回".to_string());
    }

    let 任务们 = crate::拆解为任务(&方案);
    let mut 产物们: Vec<crate::产物条目> = Vec::new();
    for (序号, 任务) in 任务们.iter().enumerate() {
        let 任务id = format!("{}-{}", 要求.id, 序号);
        let 回执 = 调度.派遣执行(&任务id, 任务, &背景)?;
        for 产物 in 回执.产物们 {
            产物们.push(crate::产物条目 {
                路径: 产物.路径,
                类别: 产物.类别,
                字节数: 产物.字节数,
            });
        }
    }

    let 回执 = crate::验收裁决(&要求.id, &产物们, 0.0);
    crate::定档(存储, &回执)?;
    Ok(回执)
}
