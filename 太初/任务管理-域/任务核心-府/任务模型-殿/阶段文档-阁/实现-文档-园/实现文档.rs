use serde::{Deserialize, Serialize};
use hm_cognition::ToolCallRecord;

/// 大罗金仙实现文档：代码变更清单 + 工具调用记录 + 自检结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImplementationDoc {
    #[serde(default)]
    pub 代码变更: Vec<CodeChange>,
    #[serde(default)]
    pub 工具调用: Vec<ToolCallRecord>,
    pub 自检: SelfCheckResult,
    pub created_at: u64,
}

/// 代码变更：一次文件级改动摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    pub 文件路径: String,
    pub 变更类型: String,
    pub 摘要: String,
}

/// 自检结果：实现者对自己的边界/契约合规检查
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfCheckResult {
    pub 通过: bool,
    pub 边界合规: bool,
    pub 契约合规: bool,
    #[serde(default)]
    pub 问题: Vec<String>,
}