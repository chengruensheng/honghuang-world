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

/// 时序·历史子页 HTML（§13.f.2a）—— 对标 Chrome Network 面板形态。
pub const 时序HTML: &str = include_str!("trajectory.html");

/// 时序·历史子页 CSS。
pub const 时序CSS: &str = include_str!("trajectory.css");

/// 时序·历史子页 JS——表格行 + Turn 分组 + 7 种事件类型 + 思考折叠。
pub const 时序JS: &str = include_str!("trajectory.js");

/// 星图·星空子页 HTML（§13.f.10.3b）—— 函数级调用图谱。
pub const 星图HTML: &str = include_str!("starmap.html");

/// 星图·星空子页 CSS。
pub const 星图CSS: &str = include_str!("starmap.css");

/// 星图·星空子页 JS——SVG 力导向布局 + 节点交互。
pub const 星图JS: &str = include_str!("starmap.js");

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
        // §13.f 轨迹表格白箱界面：id="应用" 根挂载 + id="时间线" 时间线色块 + id="三栏" 三栏布局
        assert!(主页HTML.contains("id=\"应用\""));
        assert!(主页HTML.contains("id=\"时间线\""));
        assert!(主页HTML.contains("id=\"三栏\""));
    }

    #[test]
    fn css含状态色变量() {
        // 新调色板：洪荒青绿 + 琥珀 + 警示红 + 冷蓝 + 弱字层级
        assert!(样式CSS.contains("--活"));
        assert!(样式CSS.contains("--警"));
        assert!(样式CSS.contains("--败"));
        assert!(样式CSS.contains("--弱"));
    }

    #[test]
    fn js含sse订阅() {
        // §13.f.11 轨迹 SSE 端点：/api/trajectory/stream
        assert!(脚本JS.contains("EventSource"));
        assert!(脚本JS.contains("/api/trajectory/stream"));
    }
}