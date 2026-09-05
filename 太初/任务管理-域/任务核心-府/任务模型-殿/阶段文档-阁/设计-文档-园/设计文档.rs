use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 圣人设计文档：边界契约设计（边界/安全区域/契约/文件清单/依赖）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignDoc {
    #[serde(default)]
    pub 边界定义: HashMap<String, String>,
    #[serde(default)]
    pub 安全区域: Vec<String>,
    #[serde(default)]
    pub 契约: Vec<ContractDef>,
    #[serde(default)]
    pub 修改文件: Vec<String>,
    #[serde(default)]
    pub 新建文件: Vec<String>,
    #[serde(default)]
    pub 依赖: Vec<DependencyDef>,
    pub created_at: u64,
}

/// 共识契约：一个接口定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDef {
    pub 契约名: String,
    #[serde(default)]
    pub 方法: Vec<MethodDef>,
    pub 描述: String,
}

/// 契约方法定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDef {
    pub 名称: String,
    pub 签名: String,
    pub 描述: String,
}

/// 依赖关系定义（必须单向）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyDef {
    pub 来源模块: String,
    pub 目标模块: String,
    pub 描述: String,
}