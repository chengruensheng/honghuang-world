//! 页面组装：把背景层（令牌）、布局层（槽位+契约）与功能组件层（样式+脚本）
//! 内联进页面模板，产出自包含 HTML。页面不感知任何文件路径，天然与目录结构解耦。

use std::sync::OnceLock;

use hm_error::{Error, Result};

use crate::{背景样式, 布局样式, 挂载脚本, 组件样式, 组件脚本};

/// 页面模板：占位符 {{背景样式}} / {{布局样式}} / {{组件样式}} / {{挂载脚本}} / {{组件脚本}}
const 页面模板: &str = include_str!("门面.html");

/// 组装并缓存自包含页面
fn 组装() -> &'static str {
    static 页面: OnceLock<String> = OnceLock::new();
    页面.get_or_init(|| {
        页面模板
            .replace("{{背景样式}}", 背景样式)
            .replace("{{布局样式}}", 布局样式)
            .replace("{{组件样式}}", &组件样式())
            .replace("{{挂载脚本}}", 挂载脚本)
            .replace("{{组件脚本}}", &组件脚本())
    })
}

/// 自包含页面源码（背景层 + 布局层 + 组件层已内联）
pub fn 页面源码() -> &'static str {
    组装()
}

/// 把页面落盘到目录（文件名 门面.html），返回完整文件路径
pub fn 落盘页面(目录: &str) -> Result<String> {
    std::fs::create_dir_all(目录)
        .map_err(|e| Error::Other(format!("创建目录 {目录} 失败: {e}")))?;
    let 路径 = std::path::Path::new(目录).join("门面.html");
    std::fs::write(&路径, 页面源码())
        .map_err(|e| Error::Other(format!("写入 {} 失败: {e}", 路径.display())))?;
    Ok(路径.to_string_lossy().into_owned())
}
