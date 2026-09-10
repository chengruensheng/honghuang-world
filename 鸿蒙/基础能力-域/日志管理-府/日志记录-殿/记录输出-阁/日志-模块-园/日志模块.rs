use hm_contract::{Component, Initializable};
use hm_config::LogConfig;
use hm_error::{Error, Result};

/// 日志组件：持有日志等级，实现府间契约
pub struct Logger {
    level: String,
}

impl Logger {
    pub fn new(config: &LogConfig) -> Self {
        Logger { level: config.level.clone() }
    }
}

impl Component for Logger {
    fn name(&self) -> &'static str { "日志系统" }
}

impl Initializable for Logger {
    fn init(&self) -> Result<()> {
        use tracing_subscriber::{fmt, EnvFilter};
        let filter = EnvFilter::try_new(&self.level)
            .map_err(|e| Error::Log(e.to_string()))?;
        // 输出到 stderr（无缓冲，崩溃时也能保留最后一条日志）；与 panic 的默认 stderr
        // 输出同流，避免「日志写 stdout、崩溃信息写 stderr」分家导致的 err 文件 0 字节假象。
        fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(filter)
            .try_init()
            .map_err(|e| Error::Log(e.to_string()))?;
        // 自定义 panic hook：panic（unwind）时额外经 tracing::error! 落一条结构化记录，
        // 使 panic 类崩溃在默认 stderr 输出之外仍可追踪。注意：abort 类（栈溢出/段错误/OOM）
        // 无法被 hook 捕获，需靠运行时插桩定位——本 hook 无法覆盖那类静默退出。
        let 默认钩子 = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |信息| {
            let 位置 = 信息
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "未知位置".to_string());
            let 载荷 = 信息
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| 信息.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "非字符串 panic 载荷".to_string());
            tracing::error!("panic 捕获 @ {位置}: {载荷}");
            默认钩子(信息);
        }));
        Ok(())
    }
}