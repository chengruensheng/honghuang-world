//! 对外契约：后端对外暴露的「呈现参数」（前端/客户端消费面）。
//!
//! 铁律：后端逻辑不内嵌任何前端专属信息——增删前端只改 `对外契约.toml`，不改后端代码。
//! 边界：事件词表/交互协议属「契约」，由 Rust 契约 crate 单一真相源，另置不入本文件。

use serde::Deserialize;

/// 对外契约文件名（运行时优先从工作目录读取同名文件，实现文件配置化）
pub const 对外契约文件名: &str = "对外契约.toml";

/// 对外契约根：文件配置单源
#[derive(Debug, Clone, Deserialize, Default)]
pub struct 对外契约 {
    #[serde(default)]
    pub 对外: 对外配置,
}

/// 对外呈现参数
#[derive(Debug, Clone, Deserialize)]
pub struct 对外配置 {
    /// 允许跨源访问的来源白名单（空 = 不启用 CORS 中间件，纯脚本/同源客户端）
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// 静态资源托管目录（同源托管前端页面）；空 = 不托管，纯 API
    #[serde(default)]
    pub static_dir: String,
    /// SSE 并发连接上限（0 = 不限制）
    #[serde(default = "默认并发上限")]
    pub sse_max: usize,
}

fn 默认并发上限() -> usize { 10 }

impl Default for 对外配置 {
    fn default() -> Self {
        对外配置 {
            cors_origins: Vec::new(),
            static_dir: String::new(),
            sse_max: 默认并发上限(),
        }
    }
}

// 编译时嵌入默认对外契约，不依赖运行时工作目录（cwd）
const 默认对外契约: &str = include_str!("对外契约.toml");

/// 解析契约文本，失败时告警并回退编译期内置默认（来源仅用于日志）。
fn 解析或默认(内容: &str, 来源: &str) -> 对外契约 {
    match toml::from_str(内容) {
        Ok(契约) => 契约,
        Err(e) => {
            tracing::warn!("{来源} 解析失败，回退内置默认: {e}");
            toml::from_str(默认对外契约).unwrap_or_default()
        }
    }
}

/// 从给定目录起逐级向上查找指定文件名，返回首个存在的文件路径（找不到返回 None）。
///
/// 纯文件系统查询，可单测；起点须为目录（如 `std::env::current_dir()`）。
pub fn 从目录向上查找(起点: &std::path::Path, 文件名: &str) -> Option<std::path::PathBuf> {
    let mut 当前 = Some(起点);
    while let Some(目录) = 当前 {
        let 候选 = 目录.join(文件名);
        if 候选.is_file() {
            return Some(候选);
        }
        当前 = 目录.parent();
    }
    None
}

/// 运行时对外契约：从当前工作目录逐级向上查找 `对外契约.toml`，命中即按文件加载；
/// 找不到、读取失败或解析失败均回退编译期内置默认。
pub fn 对外契约配置() -> 对外契约 {
    let 起点 = match std::env::current_dir() {
        Ok(目录) => 目录,
        Err(e) => {
            tracing::warn!("获取当前工作目录失败，使用内置默认: {e}");
            return toml::from_str(默认对外契约).unwrap_or_default();
        }
    };
    match 从目录向上查找(&起点, 对外契约文件名) {
        Some(路径) => match std::fs::read_to_string(&路径) {
            Ok(内容) => 解析或默认(&内容, &路径.display().to_string()),
            Err(e) => {
                tracing::warn!("{} 读取失败，回退内置默认: {e}", 路径.display());
                toml::from_str(默认对外契约).unwrap_or_default()
            }
        },
        None => toml::from_str(默认对外契约).unwrap_or_default(),
    }
}
