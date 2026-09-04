use serde::{Deserialize, Serialize};

/// 规则：金之规范，零散经验收敛而成的判定单元
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: u64,
    pub 名称: String,
    /// 条件：前提键值对列表，规则命中的事实要求
    pub 条件: Vec<(String, String)>,
    pub 结论: String,
    /// 优先级：数值越大越优先（金之收敛，用于冲突消解）
    pub 优先级: u32,
}

impl Rule {
    pub fn 新建(id: u64, 名称: String, 条件: Vec<(String, String)>, 结论: String, 优先级: u32) -> Self {
        Rule { id, 名称, 条件, 结论, 优先级 }
    }
}