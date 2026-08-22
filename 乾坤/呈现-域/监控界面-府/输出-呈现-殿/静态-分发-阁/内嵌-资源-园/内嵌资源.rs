//! 内嵌资源 —— 编译期内嵌 HTML/CSS/JS 三件套。
//!
//! 依据：融合蓝图-设计稿.md §9.6 静态资源清单。
//! 经 `include_str!` / `include_bytes!` 编译期内嵌，无运行时文件 IO。
//! 三件套保持极简：HTML < 50 行，CSS < 150 行，JS < 200 行。

/// 主页 HTML——暗色主题骨架，左 aside 任务列表，右区步骤面板。
pub const 主页HTML: &str = include_str!("index.html");

/// CSS 样式——暗色主题，响应式，状态色映射（绿/黄/红）。
pub const 样式CSS: &str = include_str!("style.css");

/// 前端逻辑——SSE 订阅 + 事件渲染 + LOD 三级展开 + 双视图切换。
pub const 脚本JS: &str = include_str!("app.js");

/// HTML MIME 类型。
pub const HTML_MIME: &str = "text/html; charset=utf-8";

/// CSS MIME 类型。
pub const CSS_MIME: &str = "text/css; charset=utf-8";

/// JS MIME 类型。
pub const JS_MIME: &str = "application/javascript; charset=utf-8";

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 三件套非空() {
        assert!(!主页HTML.is_empty());
        assert!(!样式CSS.is_empty());
        assert!(!脚本JS.is_empty());
    }

    #[test]
    fn html含挂载点() {
        assert!(主页HTML.contains("id=\"app\""));
        assert!(主页HTML.contains("id=\"事件流\""));
    }

    #[test]
    fn css含状态色变量() {
        assert!(样式CSS.contains("--ok"));
        assert!(样式CSS.contains("--warn"));
        assert!(样式CSS.contains("--err"));
    }

    #[test]
    fn js含sse订阅() {
        assert!(脚本JS.contains("EventSource"));
        assert!(脚本JS.contains("/api/events/stream"));
    }
}
