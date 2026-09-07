use serde::{Deserialize, Serialize};

/// 澄清记录：道祖（用户）对回退到木层的任务做出的澄清结论，写入任务作为需求纠偏决策。
///
/// 当定向回退把任务重置到「待道祖澄清」（需求偏差/回退超限升级），系统等人工澄清：
/// 道祖给出结论（澄清/变更需求或终止任务），随后重新进入设计或取消。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct 澄清记录 {
    /// 澄清发生秒级时间戳
    pub 时间: u64,
    /// 澄清结论（用户输入的需求纠偏/终止说明）
    pub 结论: String,
    /// true=澄清后继续（重新进入设计，道祖澄清中 → 待圣人设计）；false=终止任务（→ 已取消）
    pub 继续: bool,
    /// 操作者角色（缺省由装配写入道祖）
    pub 操作者: String,
}

impl 澄清记录 {
    pub fn 新(时间: u64, 结论: String, 继续: bool) -> Self {
        澄清记录 {
            时间,
            结论,
            继续,
            操作者: "道祖".to_string(),
        }
    }
}
