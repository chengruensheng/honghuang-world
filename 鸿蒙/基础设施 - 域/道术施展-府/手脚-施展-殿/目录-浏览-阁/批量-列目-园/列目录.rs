//! 批量 - 列目 - 园：列出一个目录下的条目。

use rizhi_fu::{debug, error};
use shihai_fu::世界结果;
use std::fs;
use std::path::Path;

/// 目录条目：名称、是否目录、字节数（目录字节数为 0）。
#[derive(Clone, Debug, PartialEq)]
pub struct 目录条目 {
    pub 名称: String,
    pub 是目录: bool,
    pub 字节数: u64,
}

/// 列出一个目录下的条目，按名称升序。
pub fn 列举目录(路径: &str) -> 世界结果<Vec<目录条目>> {
    let 目录 = Path::new(路径);
    if !目录.is_dir() {
        error!(路径, "列目录失败：目录不存在");
        return Err(format!("目录不存在：{路径}").into());
    }
    let 迭代 = fs::read_dir(目录).map_err(|错误| {
        error!(路径, "列目录失败：{错误}");
        format!("列目录失败：{路径}：{错误}")
    })?;
    let mut 条目们 = Vec::new();
    for 条目 in 迭代.flatten() {
        let 条目路径 = 条目.path();
        let 是目录 = 条目路径.is_dir();
        let 字节数 = if 是目录 {
            0
        } else {
            条目.metadata().map(|元数据| 元数据.len()).unwrap_or(0)
        };
        条目们.push(目录条目 {
            名称: 条目.file_name().to_string_lossy().to_string(),
            是目录,
            字节数,
        });
    }
    条目们.sort_by(|甲, 乙| 甲.名称.cmp(&乙.名称));
    debug!(路径, 条目数 = 条目们.len(), "目录已列举");
    Ok(条目们)
}
