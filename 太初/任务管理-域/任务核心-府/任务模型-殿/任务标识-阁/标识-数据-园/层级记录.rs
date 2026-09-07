use serde::{Deserialize, Serialize};
use hm_cognition::AgentRole;
use crate::任务模型_殿::产物记录;

/// 五行层级：任务处理层级标签（木→火→土→金→水）。
///
/// 木=任务发布层（道祖接待/需求澄清）、火=设计规划层（圣人）、
/// 土=实现执行层（大罗金仙）、金=验收质量层（准圣）、水=清理文档层（太乙金仙）。
/// 五行是层级标签而非固定角色函数：任何 Agent 实例可属于某层级，同层级可多实例并行。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
pub enum 五行层级 {
    #[default]
    木,
    火,
    土,
    金,
    水,
}

impl 五行层级 {
    /// 层级显示名（看板/日志展示）
    pub fn 名(&self) -> &'static str {
        match self {
            五行层级::木 => "木",
            五行层级::火 => "火",
            五行层级::土 => "土",
            五行层级::金 => "金",
            五行层级::水 => "水",
        }
    }

    /// 五行生序序号（木1 火2 土3 金4 水5），用于时间线/流转比较
    pub fn 序(&self) -> u8 {
        match self {
            五行层级::木 => 1,
            五行层级::火 => 2,
            五行层级::土 => 3,
            五行层级::金 => 4,
            五行层级::水 => 5,
        }
    }

    /// 五行相生下一层级（木→火→土→金→水→None）
    pub fn 下一层级(&self) -> Option<五行层级> {
        match self {
            五行层级::木 => Some(五行层级::火),
            五行层级::火 => Some(五行层级::土),
            五行层级::土 => Some(五行层级::金),
            五行层级::金 => Some(五行层级::水),
            五行层级::水 => None,
        }
    }

    /// 全部五行层级（按生序 木→火→土→金→水），供遍历/统计/展示
    pub fn 全部() -> [五行层级; 5] {
        [五行层级::木, 五行层级::火, 五行层级::土, 五行层级::金, 五行层级::水]
    }

    /// 本层级对应的主要职责角色（木→道祖、火→圣人、土→大罗金仙、金→准圣、水→太乙金仙）
    pub fn 归属角色(&self) -> Option<AgentRole> {
        match self {
            五行层级::木 => Some(AgentRole::道祖),
            五行层级::火 => Some(AgentRole::圣人),
            五行层级::土 => Some(AgentRole::大罗金仙),
            五行层级::金 => Some(AgentRole::准圣),
            五行层级::水 => Some(AgentRole::太乙金仙),
        }
    }

    /// 本层级的处理阶段名（看板/日志/提示词展示）
    pub fn 阶段名(&self) -> &'static str {
        match self {
            五行层级::木 => "需求",
            五行层级::火 => "设计",
            五行层级::土 => "实现",
            五行层级::金 => "验收",
            五行层级::水 => "清理",
        }
    }
}

impl From<AgentRole> for 五行层级 {
    /// 角色 → 主要职责对应层级（道祖→木、圣人→火、大罗金仙→土、准圣→金、太乙金仙→水）
    fn from(角色: AgentRole) -> Self {
        match 角色 {
            AgentRole::道祖 => 五行层级::木,
            AgentRole::圣人 => 五行层级::火,
            AgentRole::大罗金仙 => 五行层级::土,
            AgentRole::准圣 => 五行层级::金,
            AgentRole::太乙金仙 => 五行层级::水,
        }
    }
}

/// 层级记录内状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum 层级记录状态 {
    #[default]
    处理中,
    已完成,
    已回退,
}

/// 一次层级处理记录：某层级某处理者在某段时间内的处理痕迹（含产物）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct 层级记录 {
    pub 层级: 五行层级,
    /// 处理者标识（承接角色显示名或实例 id）
    pub 处理者id: String,
    /// 开始时间（秒级时间戳，承接时写入）
    pub 开始时间: u64,
    /// 完成时间（秒级时间戳，提交时写入；未完成 None）
    pub 完成时间: Option<u64>,
    pub 产物: Vec<产物记录>,
    pub 状态: 层级记录状态,
    pub 备注: String,
}

impl 层级记录 {
    pub fn 开始(层级: 五行层级, 处理者id: impl Into<String>, 开始时间: u64) -> Self {
        层级记录 {
            层级,
            处理者id: 处理者id.into(),
            开始时间,
            完成时间: None,
            产物: Vec::new(),
            状态: 层级记录状态::处理中,
            备注: String::new(),
        }
    }

    /// 完成本层处理（写完成时间 + 状态 + 产物），返回 Self（不可变追加后回写由调用方处理）
    pub fn 完成(mut self, 完成时间: u64, 产物: Vec<产物记录>, 备注: impl Into<String>) -> Self {
        self.完成时间 = Some(完成时间);
        self.产物 = 产物;
        self.状态 = 层级记录状态::已完成;
        self.备注 = 备注.into();
        self
    }

    /// 标记回退（保留痕迹，状态改为已回退）
    pub fn 标记回退(mut self, 完成时间: u64) -> Self {
        self.完成时间 = Some(完成时间);
        self.状态 = 层级记录状态::已回退;
        self
    }

    /// 本层是否处理中
    pub fn 处理中(&self) -> bool {
        self.状态 == 层级记录状态::处理中
    }
}
