//! 版本定档：源码快照存档 / 回退

use crate::工作区根;
use std::path::Path;

fn 版本库(根: &Path) -> std::path::PathBuf {
    根.join(".上下文").join("版本库")
}

pub fn 存档版本() -> String {
    let 根 = 工作区根();
    let 目标 = 版本库(&根).join("版本-v1").join("源码-快照");
    match tianting_fu::源码快照(&根, &目标) {
        Ok(数) => format!("版本已存档\n快照目录：{}\n复制文件：{数} 件", 目标.display()),
        Err(错误) => format!("存档失败：{错误}"),
    }
}

pub fn 回退版本(版本号: &str) -> String {
    let 根 = 工作区根();
    let 快照 = 版本库(&根).join(format!("版本-{版本号}")).join("源码-快照");
    if !快照.exists() {
        return format!("版本 {版本号} 快照不存在：{}", 快照.display());
    }
    let 回退目标 = 版本库(&根).join("回退区");
    match tianting_fu::回退版本(&快照, &回退目标) {
        Ok(数) => format!("版本已回退到回退区\n恢复文件：{数} 件\n目标：{}", 回退目标.display()),
        Err(错误) => format!("回退失败：{错误}"),
    }
}
