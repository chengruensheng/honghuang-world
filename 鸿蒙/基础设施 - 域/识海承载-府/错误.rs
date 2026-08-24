//! §B.2.3 统一世界错误（替代 12 crate 的 `Result<(), String>` 丢 Error 类型）
//!
//! 用 `thiserror::Error` derive 统一错误分类：
//! - 工作区错误（路径解析失败、OnceLock 冲突）
//! - 格位错误（无效名、路径逃逸、超长）
//! - 记录错误（反序列化失败、写入失败）
//! - 存储错误（Io、NotFound）
//! - 抽象错误（Sqlite未实装 — B.3）
//!
//! 后续 B.2.7 注册表 + B.2.9 一致性快照都依赖统一错误。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum 世界错误 {
    #[error("工作区错误：{0}")]
    工作区(String),

    #[error("格位错误：{0}")]
    格位(String),

    #[error("记录错误：{0}")]
    记录(String),

    #[error("存储错误：{0}")]
    存储(String),

    #[error("抽象未实装：{0}")]
    抽象未实装(String),

    #[error("路径逃逸工作区根：{路径}")]
    路径逃逸 { 路径: String },

    #[error("I/O 错误：{0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 反序列化错误：{0}")]
    Json(#[from] serde_json::Error),
}

/// Result 简写（所有 fn 默认 Result<T, 世界错误>）
pub type 世界结果<T> = Result<T, 世界错误>;

impl From<&str> for 世界错误 {
    fn from(s: &str) -> Self {
        世界错误::工作区(s.to_string())
    }
}

impl From<String> for 世界错误 {
    fn from(s: String) -> Self {
        世界错误::工作区(s)
    }
}
