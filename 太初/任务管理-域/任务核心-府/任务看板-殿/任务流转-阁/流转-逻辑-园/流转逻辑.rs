use hm_cognition::AgentRole;
use crate::任务模型_殿::TaskStatus;

/// 当前状态应由哪个角色操作（承接或提交）
pub fn 状态归属角色(状态: TaskStatus) -> Option<AgentRole> {
    match 状态 {
        TaskStatus::待圣人设计 | TaskStatus::圣人设计中 => Some(AgentRole::圣人),
        TaskStatus::待大罗金仙实现 | TaskStatus::大罗金仙实现中 | TaskStatus::待修复 => {
            Some(AgentRole::大罗金仙)
        }
        TaskStatus::待准圣验收 | TaskStatus::准圣验收中 => Some(AgentRole::准圣),
        TaskStatus::待道祖终审 | TaskStatus::道祖终审中 => Some(AgentRole::道祖),
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
        TaskStatus::待清理 => Some(TaskStatus::清理中),
        _ => None,
    }
}
