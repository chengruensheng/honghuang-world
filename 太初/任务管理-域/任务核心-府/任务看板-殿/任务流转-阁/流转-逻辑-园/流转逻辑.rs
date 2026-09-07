use hm_cognition::AgentRole;
use crate::任务模型_殿::{TaskStatus, 五行层级};

/// 状态 → 五行层级标签（全状态权威映射，作为「状态→五行」唯一事实源）。
///
/// 归一法则：
/// - 火=设计层（圣人）：待圣人设计 / 圣人设计中 / 待重新设计
/// - 土=实现层（大罗金仙）：待大罗金仙实现 / 大罗金仙实现中 / 待修复 / 待重新实现
/// - 金=验收层（准圣）：待准圣验收 / 准圣验收中 / 待道祖终审 / 道祖终审中 / 待重新验收 / 已完成
/// - 水=清理层（太乙金仙）：待清理 / 清理中 / 待重新清理 / 清理完成
/// - 木=需求层（道祖）：待受理 / 进行中 / 已取消 / 待道祖澄清 / 道祖澄清中
#[allow(clippy::match_like_matches_macro)]
pub fn 状态层级标签(状态: TaskStatus) -> 五行层级 {
    match 状态 {
        TaskStatus::待圣人设计
        | TaskStatus::圣人设计中
        | TaskStatus::待重新设计 => 五行层级::火,
        TaskStatus::待大罗金仙实现
        | TaskStatus::大罗金仙实现中
        | TaskStatus::待修复
        | TaskStatus::待重新实现 => 五行层级::土,
        TaskStatus::待准圣验收
        | TaskStatus::准圣验收中
        | TaskStatus::待道祖终审
        | TaskStatus::道祖终审中
        | TaskStatus::待重新验收
        | TaskStatus::已完成 => 五行层级::金,
        TaskStatus::待清理
        | TaskStatus::清理中
        | TaskStatus::待重新清理
        | TaskStatus::清理完成 => 五行层级::水,
        TaskStatus::待受理
        | TaskStatus::进行中
        | TaskStatus::已取消
        | TaskStatus::待道祖澄清
        | TaskStatus::道祖澄清中 => 五行层级::木,
    }
}

/// 当前状态应由哪个角色操作（承接或提交）。
/// 注意：与 `状态层级标签` 非一一对应（如 待道祖终审 归金层但由道祖操作），故保留精确角色映射。
pub fn 状态归属角色(状态: TaskStatus) -> Option<AgentRole> {
    match 状态 {
        TaskStatus::待圣人设计 | TaskStatus::圣人设计中 => Some(AgentRole::圣人),
        TaskStatus::待大罗金仙实现 | TaskStatus::大罗金仙实现中 | TaskStatus::待修复 => {
            Some(AgentRole::大罗金仙)
        }
        TaskStatus::待准圣验收 | TaskStatus::准圣验收中 => Some(AgentRole::准圣),
        TaskStatus::待道祖终审 | TaskStatus::道祖终审中 => Some(AgentRole::道祖),
        TaskStatus::待道祖澄清 | TaskStatus::道祖澄清中 => Some(AgentRole::道祖),
        TaskStatus::待清理 | TaskStatus::清理中 => Some(AgentRole::太乙金仙),
        _ => None,
    }
}

/// 承接后流转到的「进行中」状态
pub fn 承接后状态(状态: TaskStatus) -> Option<TaskStatus> {
    match 状态 {
        TaskStatus::待圣人设计 => Some(TaskStatus::圣人设计中),
        TaskStatus::待大罗金仙实现 => Some(TaskStatus::大罗金仙实现中),
        TaskStatus::待准圣验收 => Some(TaskStatus::准圣验收中),
        TaskStatus::待修复 => Some(TaskStatus::大罗金仙实现中),
        TaskStatus::待道祖终审 => Some(TaskStatus::道祖终审中),
        TaskStatus::待道祖澄清 => Some(TaskStatus::道祖澄清中),
        TaskStatus::待清理 => Some(TaskStatus::清理中),
        _ => None,
    }
}
