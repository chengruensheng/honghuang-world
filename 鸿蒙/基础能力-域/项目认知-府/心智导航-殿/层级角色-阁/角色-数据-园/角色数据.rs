use serde::{Deserialize, Serialize};

/// 层级角色：洪荒五层协作的五个角色，各有格位维度与工具权限。
///
/// 道祖（决策/终审）、圣人（设计）、大罗金仙（实现）、准圣（验收）、太乙金仙（清理）。
/// 该类型为跨府共享类型，置于鸿蒙认知府，供任务府（太初）与上下文管理共同引用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentRole {
    #[default]
    道祖,
    圣人,
    大罗金仙,
    准圣,
    太乙金仙,
}

impl AgentRole {
    /// 角色显示名（用于日志与看板展示）
    pub fn 名称(&self) -> &'static str {
        match self {
            AgentRole::道祖 => "道祖",
            AgentRole::圣人 => "圣人",
            AgentRole::大罗金仙 => "大罗金仙",
            AgentRole::准圣 => "准圣",
            AgentRole::太乙金仙 => "太乙金仙",
        }
    }

    /// 角色职责（如「圣人 · 边界契约设计」）。
    ///
    /// 任务书提示词与界面步骤名共用这一张表：两处各写一份，早晚改了一处忘了另一处。
    pub fn 职责(&self) -> &'static str {
        match self {
            AgentRole::道祖 => "决策与终审",
            AgentRole::圣人 => "边界契约设计",
            AgentRole::大罗金仙 => "代码实现与自检",
            AgentRole::准圣 => "逐项验收",
            AgentRole::太乙金仙 => "清理与归档",
        }
    }

    /// 由显示名反查角色。事件流里只带显示名，消费方据此还原角色类型。
    pub fn 从名称(名: &str) -> Option<AgentRole> {
        Some(match 名 {
            "道祖" => AgentRole::道祖,
            "圣人" => AgentRole::圣人,
            "大罗金仙" => AgentRole::大罗金仙,
            "准圣" => AgentRole::准圣,
            "太乙金仙" => AgentRole::太乙金仙,
            _ => return None,
        })
    }
}
