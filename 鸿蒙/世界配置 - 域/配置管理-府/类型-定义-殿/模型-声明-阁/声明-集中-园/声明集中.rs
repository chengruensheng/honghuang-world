//! 配置管理-府 · 核心类型：密钥、模型配置与装配配置。

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

/// 世界阶段（装配配置用；与天庭治理-府 阶段 枚举同语义，此处独立类型避免跨府依赖）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 阶段值 {
    甲,
    乙,
}

/// 扩展开关：可启用的世界扩展点（阶段 4 Profile 装配）。
/// 甲阶段装配不启用 巡世/进化（自动优化仅乙阶段）；观测 为可观测扩展点。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum 扩展开关 {
    巡世,
    进化,
    观测,
}

/// 装配配置（阶段 4 · 融合蓝图 §14.11）：`.上下文/装配.json` 声明世界启动装配。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 装配配置 {
    /// 世界阶段（甲 = 功能优先关自动优化；乙 = 开 巡世/进化）。
    pub 阶段: 阶段值,
    /// 启用的扩展点。
    pub 启用扩展: Vec<扩展开关>,
    /// 模型提供者键（全局提供者注册表）。
    pub 模型提供者: String,
    /// 角色册 json 路径（阶段 3 全局角色册 装载源）。
    pub 角色册路径: String,
}

impl Default for 装配配置 {
    fn default() -> Self {
        Self {
            阶段: 阶段值::乙,
            启用扩展: vec![扩展开关::巡世, 扩展开关::进化, 扩展开关::观测],
            模型提供者: "http".to_string(),
            角色册路径:
                "鸿蒙/基础设施 - 域/道术施展-府/角色-卡册-殿/角色-登记-阁/登记-落册-园/角色册.json"
                    .to_string(),
        }
    }
}
