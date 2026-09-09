//! 视图-组件-园：左栏视图抽屉（记忆 · 图谱），经「乾坤图标」事件驱动，数据来自真实接口。

/// 左栏视图组件样式：只引用令牌，不写死颜色
pub const 视图样式: &str = include_str!("视图.css");

/// 左栏视图组件脚本：监听乾坤图标事件，fetch /api/memories 与 /api/cognition/graph
pub const 视图脚本: &str = include_str!("视图.js");
