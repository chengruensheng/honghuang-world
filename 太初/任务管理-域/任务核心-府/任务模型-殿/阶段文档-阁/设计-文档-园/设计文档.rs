use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 圣人设计文档：边界契约设计（边界/安全区域/契约/文件清单/依赖）
///
/// 「无解声明」是设计层对「契约不可满足」的结构化判定：需求在数学/信息论上不可能满足时，
/// 圣人据实声明（而非把契约降级后当作可满足），系统据此直接把任务推入「已确认无解」终态——
/// 避免下游「降级实现后自报通过」（造假）或「拒绝实现后被验收反复打回至卡死」（缺陷 15-1/15-2）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// 契约不可满足声明：None = 契约可满足（正常进入实现）；Some = 已判定无解并进入终态。
    #[serde(default)]
    pub 无解声明: Option<无解声明>,
    #[serde(default)]
    pub created_at: u64,
}

/// 契约不可满足的结构化声明：给出「为何无解」的判定依据，随终态一并留痕。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct 无解声明 {
    /// 不可满足的契约名（与 契约[].契约名 对应）
    pub 契约名: String,
    /// 判定依据（如：鸽巢原理／奇偶性／信息论下界），须为可复核的逻辑或数学理由
    pub 判定依据: String,
    /// 无解类型：数学无解 / 信息论无解 / 契约自相矛盾 / 其他
    #[serde(default)]
    pub 类型: String,
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