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
        fmt().with_env_filter(filter)
            .try_init()
            .map_err(|e| Error::Log(e.to_string()))?;
        Ok(())
    }
}