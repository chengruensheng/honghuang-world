//! 要求提审：想法 → 解析想法 → 化为要求书 → 入队

use crate::{打开存储, 状态目录, 读模型配置};
use rizhi_fu::{error, info, warn};

pub fn 化为要求(想法id: &str) -> String {
    let 想法池 =
        tianting_fu::落盘队列::<tianting_fu::想法>::打开(状态目录().join("想法.jsonl"));
    let 想法们 = match 想法池.读全部() {
        Ok(项们) => 项们,
        Err(错误) => {
            error!(想法id, "读想法池失败：{错误}");
            return format!("读想法池失败：{错误}");
        }
    };
    let 想法 = match 想法们.iter().find(|项| 项.id == 想法id) {
        Some(项) => 项.clone(),
        None => {
            warn!(想法id, "想法不在池中");
            return format!("想法 {想法id} 不在池中（先用「想法 投递」）");
        }
    };
    let 配置 = 读模型配置();
    let 存储 = 打开存储();
    let 背景 = shihai_fu::拼装投影(&存储, "鸿钧", shihai_fu::全部格位(), 8000).unwrap_or_default();
    match tianting_fu::解析想法(&想法.id, &想法.内容, &背景, &配置) {
        Ok((要求, 用量)) => {
            let 队列 = tianting_fu::落盘队列::<tianting_fu::要求书>::打开(
                状态目录().join("要求.jsonl"),
            );
            match 队列.入队(&要求) {
                Ok(_) => {
                    info!(想法id, 要求id = %要求.id, 提示词 = 用量.提示词, 缓存命中 = 用量.缓存命中, "已化为要求并入队");
                    format!(
                        "已化为要求并入队\n想法id：{想法id}\n要求id：{}\n方向：{}\n状态：{:?}",
                        要求.id, 要求.方向, 要求.状态
                    )
                }
                Err(错误) => {
                    error!(想法id, "要求入队失败：{错误}");
                    format!("要求入队失败：{错误}")
                }
            }
        }
        Err(错误) => {
            error!(想法id, "化为要求失败：{错误}");
            format!("化为要求失败：{错误}")
        }
    }
}
