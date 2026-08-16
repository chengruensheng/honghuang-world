//! 原子 - 写入 - 园：写文件全部内容，自动创建父目录。
//! 写前经 回滚垫 备份旧内容：任务失败时可单文件撤销恢复。

use rizhi_fu::{debug, error};
use shihai_fu::{当前任务, 回滚垫, 工作区};
use std::path::Path;

/// 写文件全部内容，父目录不存在时自动创建；写前先备份进回滚垫（失败只警告不阻断）。
pub fn 写文件(路径: &str, 内容: &str) -> Result<(), String> {
    let 垫 = 回滚垫::在工作区(&工作区::定位());
    if let Err(错误) = 垫.备份(&当前任务(), 路径) {
        debug!(路径, "回滚垫备份跳过：{错误}");
    }
    let 目标 = Path::new(路径);
    if let Some(父) = 目标.parent() {
        if !父.as_os_str().is_empty() {
            std::fs::create_dir_all(父)
                .map_err(|错误| format!("创建父目录失败：{}：{错误}", 父.display()))?;
        }
    }
    std::fs::write(目标, 内容).map_err(|错误| {
        error!(路径, "写文件失败：{错误}");
        format!("写文件失败：{路径}：{错误}")
    })?;
    debug!(路径, 字节数 = 内容.len(), "写文件完成");
    Ok(())
}

