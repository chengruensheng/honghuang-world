//! 成镜组装：骨架 + 样式 + 四脚本（长河/河口/流光/组件）→ 自包含单页。
//! 落盘 = 纯新增文件，不触碰现有页面资产（零影响现有）。

use hm_error::{Error, Result};

pub const 水镜样式: &str = include_str!("水镜.css");
pub const 水镜骨架: &str = include_str!("水镜.html");

/// 页面源码：骨架注入样式与全部脚本（自包含，零外部依赖）。
/// 内联铁律：脚本入 HTML 前必须中和 HTML 毒序列——
///   `</script` 会提前闭合脚本标签（流光样张实测触发），`<!--` 可诱发解析器
///   双重转义态；`\/` 与 `\!` 在 JS 字符串/正则中值不变，故转换无损语义。
pub fn 页面源码() -> String {
    let 脚本 = format!(
        "{}\n{}\n{}\n{}",
        crate::长河脚本,
        crate::适配脚本,
        crate::流光脚本,
        crate::组件脚本,
    )
    .replace("</script", "<\\/script")
    .replace("<!--", "<\\!--");
    水镜骨架
        .replace("{{水镜样式}}", 水镜样式)
        .replace("{{水镜脚本}}", &脚本)
}

/// 落盘：目录/水镜.html（与 门面.html 并列，供同源 A/B）
pub fn 落盘页面(目录: &str) -> Result<String> {
    std::fs::create_dir_all(目录)
        .map_err(|e| Error::Other(format!("创建目录 {目录} 失败: {e}")))?;
    let 路径 = std::path::Path::new(目录).join("水镜.html");
    std::fs::write(&路径, 页面源码())
        .map_err(|e| Error::Other(format!("写入 {} 失败: {e}", 路径.display())))?;
    Ok(路径.to_string_lossy().into_owned())
}
