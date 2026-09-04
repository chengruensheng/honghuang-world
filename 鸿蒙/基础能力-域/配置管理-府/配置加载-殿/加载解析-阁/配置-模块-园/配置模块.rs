use serde::Deserialize;
use hm_error::{Error, Result};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    /// 启动时是否运行五行自检（默认关闭，生产环境不污染真实数据）
    #[serde(default)]
    pub run_self_test: bool,
    /// 启动时是否进入自主开发智能体入口（默认关闭，需显式开启并配置任务）
    #[serde(default)]
    pub run_dev_agent: bool,
    /// 自主开发智能体的工作区根目录
    #[serde(default = "default_workspace")]
    pub dev_workspace: String,
    /// 自主开发智能体的任务描述
    #[serde(default)]
    pub dev_task: String,
    /// 自主开发智能体的最大循环轮数
    #[serde(default = "default_max_rounds")]
    pub dev_max_rounds: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_level")]
    pub level: String,
}

fn default_name() -> String { "洪荒·世界".into() }
fn default_version() -> String { "0.1.0".into() }
fn default_level() -> String { "info".into() }
fn default_workspace() -> String { "F:/临时工作区".into() }
fn default_max_rounds() -> usize { 20 }

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            name: default_name(),
            version: default_version(),
            run_self_test: false,
            run_dev_agent: false,
            dev_workspace: default_workspace(),
            dev_task: String::new(),
            dev_max_rounds: default_max_rounds(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig { level: default_level() }
    }
}

// 编译时嵌入默认配置，不依赖运行时工作目录（cwd）
const DEFAULT_TOML: &str = include_str!("../../默认配置-阁/默认-配置-园/default.toml");

/// 默认配置（编译时嵌入，任何 cwd 下均可用）
pub fn default_config() -> Config {
    toml::from_str(DEFAULT_TOML).unwrap_or_default()
}

/// 运行时加载外部配置文件（path 为绝对路径或相对调用方 cwd 的路径）
pub fn load(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Config(format!("读取配置 {path} 失败: {e}")))?;
    toml::from_str(&content)
        .map_err(|e| Error::Config(format!("解析配置失败: {e}")))
}
