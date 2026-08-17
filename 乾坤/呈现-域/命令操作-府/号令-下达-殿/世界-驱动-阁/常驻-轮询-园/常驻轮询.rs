//! 常驻-轮询-园：任务线消费——「世界 守护」常驻循环 +「世界 驱动」单条执行。
//! 设计稿 §1.5.5 任务线机制：对话发布的任务线由守护消费，防 CLI 退出丢任务。

use crate::{读模型配置, 打开存储};
use rizhi_fu::{info, warn};
use std::time::Duration;

/// 「世界 守护」：常驻循环消费待执行任务线（间隔 2 秒）。
/// 界主开一个终端跑它，世界的"心脏"就跳起来了——对话发布的任务全部异步执行，完成后鸿钧汇报进对话记录。
/// 注意：本命令不退出；单实例运行（多实例由任务线锁文件互斥，先到先得）。
pub fn 世界守护() -> String {
    let 配置 = 读模型配置();
    let 存储 = 打开存储();
    info!("世界守护启动（常驻）");
    loop {
        match tianting_fu::执行一条待执行任务线(&配置, &存储) {
            Ok(Some(汇报)) => {
                info!(汇报 = %汇报.chars().take(120).collect::<String>(), "任务线完成");
            }
            Ok(None) => std::thread::sleep(Duration::from_secs(2)),
            Err(错误) => {
                warn!(错误 = %错误, "任务线执行异常，稍后继续轮询");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

/// 「世界 驱动」：立即执行一条待执行任务线（手动驱动，供未跑守护时使用）。
pub fn 世界驱动() -> String {
    let 配置 = 读模型配置();
    let 存储 = 打开存储();
    match tianting_fu::执行一条待执行任务线(&配置, &存储) {
        Ok(Some(汇报)) => format!("已执行一条任务线\n{汇报}"),
        Ok(None) => "当前无待执行任务线（锁被占用时本轮跳过，稍后再试）".to_string(),
        Err(错误) => format!("执行失败：{错误}"),
    }
}

/// 「世界 中止」：中止指定任务线（待执行不再被领取；执行中任务完成后撤销产物不汇报）。
pub fn 世界中止(任务线id: &str) -> String {
    match tianting_fu::中止任务线(任务线id.trim()) {
        Ok(文本) => 文本,
        Err(错误) => format!("中止失败：{错误}"),
    }
}
