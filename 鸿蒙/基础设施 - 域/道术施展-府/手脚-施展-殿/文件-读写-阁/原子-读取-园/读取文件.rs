//! 原子 - 读取 - 园：读文件全部内容。

use rizhi_fu::{debug, error};
use shihai_fu::世界结果;

/// 读文件全部文本，失败返回中文错误。
pub fn 读文件(路径: &str) -> 世界结果<String> {
    let 内容 = std::fs::read_to_string(路径).map_err(|错误| {
        error!(路径, "读文件失败：{错误}");
        format!("读文件失败：{路径}：{错误}")
    })?;
    debug!(路径, 长度 = 内容.len(), "文件已读取");
    Ok(内容)
}
