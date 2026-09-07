use hm_error::{Error, Result};

/// 所有可插拔组件的基础标识
pub trait Component: Send + Sync {
    fn name(&self) -> &'static str;
}

/// 可初始化的组件：生产/测试路径都通过它接入
pub trait Initializable: Component {
    fn init(&self) -> Result<()>;
}

/// 可关闭的组件：保证资源释放
pub trait Shutdown: Component {
    fn shutdown(&self);
}

/// 当前秒级时间戳（跨引擎统一提供，避免各引擎重复定义）
pub fn 当前时间戳() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 原子写入文件：先写临时文件再重命名，避免写入中途崩溃导致数据损坏。
///
/// 临时文件路径 = 原路径 + ".tmp"；写入 + 重命名均失败时返回错误。
pub fn 原子写入文件(路径: &str, 内容: &str) -> Result<()> {
    let 临时路径 = format!("{路径}.tmp");
    std::fs::write(&临时路径, 内容).map_err(Error::Io)?;
    std::fs::rename(&临时路径, 路径).map_err(Error::Io)?;
    Ok(())
}