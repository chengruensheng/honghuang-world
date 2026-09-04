use hm_error::Result;

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