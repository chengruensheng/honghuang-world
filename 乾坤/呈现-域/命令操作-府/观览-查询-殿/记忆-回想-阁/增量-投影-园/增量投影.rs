//! 记忆回想：读识海格位记录（投影）

use crate::打开存储;
use rizhi_fu::{error, info};

pub fn 记忆投影(格位: &str) -> String {
    let 存储 = 打开存储();
    match 存储.读格位(格位) {
        Ok(记录们) => {
            info!(格位, 条数 = 记录们.len(), "记忆投影完成");
            let mut 行 = format!("格位「{格位}」共 {} 条记录\n", 记录们.len());
            for 记录 in 记录们.iter().rev().take(20) {
                let 内容: String = 记录.内容.chars().take(60).collect();
                行.push_str(&format!("- {}（{:?}）\n", 内容, 记录.来源));
            }
            行
        }
        Err(错误) => {
            error!(格位, "记忆投影失败：{错误}");
            format!("读取失败：{错误}")
        }
    }
}
