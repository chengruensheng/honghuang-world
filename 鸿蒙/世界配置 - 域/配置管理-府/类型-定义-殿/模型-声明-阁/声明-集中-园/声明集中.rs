//! 配置管理-府 · 核心类型：密钥与模型配置。

use serde::{Deserialize, Serialize};

/// 模型配置：一次模型连接所需的完整参数。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 模型配置 {
    pub 密钥: String,
    pub 地址: String,
    pub 模型: String,
}

/// 配置来源。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 配置来源 {
    环境文件,
    环境变量,
    内置默认,
}
