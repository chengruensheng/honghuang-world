use serde::{Deserialize, Serialize};

/// 准圣验收文档：多轮验收 + 最终结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationDoc {
    #[serde(default)]
    pub 轮次: Vec<VerificationRound>,
    pub 最终结果: bool,
    pub created_at: u64,
}

/// 单轮验收记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRound {
    pub 轮次: u32,
    pub 通过: bool,
    pub 边界检查: bool,
    pub 契约检查: bool,
    pub 安全检查: bool,
    pub 事实检查: bool,
    pub 完整性检查: bool,
    #[serde(default)]
    pub 问题: Vec<String>,
    pub 建议: String,
}

/// 道祖终审文档：最终验收（需求满足度/可维护性/代码质量/风险）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalAcceptanceDoc {
    pub 通过: bool,
    pub 需求满足度: u8,
    pub 可维护性: u8,
    pub 代码质量: u8,
    pub 风险评估: String,
    pub 评语: String,
    pub created_at: u64,
}