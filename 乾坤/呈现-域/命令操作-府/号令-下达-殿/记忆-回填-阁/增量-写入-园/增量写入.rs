//! 记忆回填 / 记忆播种：写记录进识海格位

use crate::{工作区根, 打开存储};

pub fn 记忆回填(格位: &str, 内容: &str) -> String {
    let 存储 = 打开存储();
    let 记录 = shihai_fu::记录::新(格位, 内容, "号令回填", "人类");
    match 存储.写记录(&记录) {
        Ok(_) => format!("记忆已回填\n格位：{格位}"),
        Err(错误) => format!("回填失败：{错误}"),
    }
}

pub fn 记忆播种() -> String {
    let 根 = 工作区根();
    let 存储 = 打开存储();
    match shihai_fu::扫描(&存储, &根) {
        Ok(条数) => format!("记忆已播种（代码扫描）\n写入：{条数} 条记录"),
        Err(错误) => format!("播种失败：{错误}"),
    }
}
