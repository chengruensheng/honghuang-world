use serde::{Deserialize, Serialize};
use crate::任务模型_殿::五行层级;

/// 驳回原因：最终审核不通过时的具体原因，映射到定向回退的根源层级。
///
/// 木=需求层（需求不清 / 扩大范围 / 缩小范围）、火=设计层（设计不符）、
/// 土=实现层（实现错误 / 测试不足 / 产出不完整）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum 驳回原因 {
    需求不清,
    设计不符,
    实现错误,
    测试不足,
    产出不完整,
    扩大范围,
    缩小范围,
}

impl 驳回原因 {
    /// 驳回原因 → 定向回退根源层级（供 TaskBoard 定向回退复用同一映射）
    pub fn 回退层级(&self) -> 五行层级 {
        match self {
            驳回原因::需求不清 | 驳回原因::扩大范围 | 驳回原因::缩小范围 => 五行层级::木,
            驳回原因::设计不符 => 五行层级::火,
            驳回原因::实现错误 | 驳回原因::测试不足 | 驳回原因::产出不完整 => 五行层级::土,
        }
    }
}

/// 审核来源：区分 LLM 自动审核与人工覆盖（默认 LLM，人可看可不看）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum 审核来源 {
    /// LLM 默认自动审核（全自动驱动，人可看可不看）
    自动,
    /// 人工经 review API 覆盖
    人工,
}

/// 审核记录：最终审核（道祖终审后插入）阶段的审核结论，写入任务作为交付证据链。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct 审核记录 {
    /// 审核发生秒级时间戳
    pub 审核时间: u64,
    /// 是否通过（通过→待清理；驳回→按驳回原因定向回退）
    pub 通过: bool,
    /// 驳回原因（通过时为 None）
    pub 驳回原因: Option<驳回原因>,
    /// 审核评语（通过/驳回的说明）
    pub 评语: String,
    /// 审核来源（自动=LLM 默认审核；人工=经 review API 覆盖）
    pub 来源: 审核来源,
}

impl 审核记录 {
    pub fn 新(审核时间: u64, 通过: bool, 驳回原因: Option<驳回原因>, 评语: String, 来源: 审核来源) -> Self {
        审核记录 {
            审核时间,
            通过,
            驳回原因,
            评语,
            来源,
        }
    }
}
