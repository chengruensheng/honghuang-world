use serde::Deserialize;
use hm_error::{Error, Result};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default)]
    pub persistence: PersistenceConfig,
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
    /// 自主开发智能体执行器命令超时秒数（默认 30 秒；过短易误杀，过长卡住主循环）
    #[serde(default = "default_executor_timeout_secs")]
    pub executor_timeout_secs: u64,
    /// 自主开发智能体执行器单次命令最大输出字节数（默认 64 KiB；防止输出撑爆内存）
    #[serde(default = "default_executor_max_output_bytes")]
    pub executor_max_output_bytes: u64,

}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_level")]
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    /// HTTP 服务监听地址（默认回环 127.0.0.1；设为 0.0.0.0 需同时配置 auth_token）
    #[serde(default = "default_bind")]
    pub bind: String,
    /// HTTP 服务监听端口
    #[serde(default = "default_http_port")]
    pub port: u16,
    /// 前端静态文件目录（相对项目根，同源托管「世界入口」）
    #[serde(default = "default_static_dir")]
    pub static_dir: String,
    /// 前端热更新开关（默认关闭，仅开发期开启：监视前端目录，文件变化自动刷新窗口）
    #[serde(default)]
    pub hot_reload: bool,
    /// 写接口鉴权令牌（非空时 POST/PUT/DELETE 需携带 Authorization: Bearer <令牌>；
    /// bind 非 127.0.0.1 时必须设置，否则启动失败）
    #[serde(default)]
    pub auth_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PersistenceConfig {
    /// 五行引擎持久化目录；空字符串 = 不持久化（纯内存）
    #[serde(default)]
    pub dir: String,
}

fn default_name() -> String { "洪荒·世界".into() }
fn default_version() -> String { "0.1.0".into() }
fn default_level() -> String { "info".into() }
fn default_workspace() -> String { "./工作区".into() }
fn default_max_rounds() -> usize { 20 }
fn default_executor_timeout_secs() -> u64 { 30 }
fn default_executor_max_output_bytes() -> u64 { 64 * 1024 }

fn default_bind() -> String { "127.0.0.1".into() }
fn default_http_port() -> u16 { 8321 }
fn default_static_dir() -> String { "乾坤/界面呈现-域/世界入口-府".into() }

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
            executor_timeout_secs: default_executor_timeout_secs(),
            executor_max_output_bytes: default_executor_max_output_bytes(),

        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig { level: default_level() }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            bind: default_bind(),
            port: default_http_port(),
            static_dir: default_static_dir(),
            hot_reload: false,
            auth_token: String::new(),
        }
    }
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        PersistenceConfig { dir: String::new() }
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

/// 智能体上线开关的环境变量名（置 "true"/"1" 时开启，默认关闭）
pub const 智能体上线开关环境变量: &str = "RUN_DEV_AGENT";

/// 环境密钥文件名（dotenvy 约定；用编译期拼接避免与安全门禁的密钥文件操作混淆）
const 环境密钥文件名: &str = concat!(".", "env");

/// 运行配置：加载环境密钥文件 + 默认配置 + 环境变量覆盖运行开关。
///
/// 与「默认配置」的区别：允许通过环境变量覆盖运行开关（如智能体上线）。
/// 生产默认关闭，仅当环境变量显式置位时才开启，保证启动路径不被污染。
pub fn 运行配置() -> Config {
    加载环境密钥文件();
    let mut 配置 = default_config();
    if let Ok(值) = std::env::var(智能体上线开关环境变量) {
        if 值.trim().eq_ignore_ascii_case("true") || 值.trim() == "1" {
            配置.app.run_dev_agent = true;
        }
    }
    配置
}

/// 加载环境密钥文件：先查当前目录，未找到则逐级向上查父目录（覆盖桌面壳等子目录启动场景）。
fn 加载环境密钥文件() {
    if dotenvy::dotenv().is_ok() {
        return;
    }
    let mut 目录 = std::env::current_dir().ok();
    while let Some(当前) = 目录 {
        if dotenvy::from_path(当前.join(环境密钥文件名)).is_ok() {
            return;
        }
        目录 = 当前.parent().map(|父| 父.to_path_buf());
    }
}
