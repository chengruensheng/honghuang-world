use serde::{Deserialize, Serialize};
use hm_cognition::AgentRole;
use crate::任务模型_殿::{
    TaskStatus, TaskScene, TaskPriority, StatusChange,
    RequirementDoc, DesignDoc, ImplementationDoc, VerificationDoc, FinalAcceptanceDoc,
    任务标识, 层级记录, 回退记录, 五行层级, 扫尾记录, 澄清记录, 审核记录,
};

/// 任务：太初之木，从无到有的存在。
///
/// 扩展后包含洪荒五层流转的完整元信息：场景/优先级/发起人/承接历史 + 五阶段产物 + 状态历史与统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    // 原有字段
    pub id: u64,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: u64,
    // 新增：洪荒五层元信息
    #[serde(default)]
    pub 场景: TaskScene,
    #[serde(default)]
    pub 优先级: TaskPriority,
    #[serde(default)]
    pub 发起人: AgentRole,
    #[serde(default)]
    pub 当前承接人: AgentRole,
    #[serde(default)]
    pub 承接历史: Vec<AgentRole>,
    // 新增：各阶段产物
    #[serde(default)]
    pub 需求文档: Option<RequirementDoc>,
    #[serde(default)]
    pub 设计文档: Option<DesignDoc>,
    #[serde(default)]
    pub 实现文档: Option<ImplementationDoc>,
    #[serde(default)]
    pub 验收文档: Option<VerificationDoc>,
    #[serde(default)]
    pub 终审文档: Option<FinalAcceptanceDoc>,
    // 新增：元数据
    #[serde(default)]
    pub 状态历史: Vec<StatusChange>,
    #[serde(default)]
    pub 总令牌: u64,
    #[serde(default)]
    pub 修复轮次: u32,
    #[serde(default)]
    pub updated_at: u64,
    // 新增：任务标识驱动（唯一可追溯：UUID + 时间戳 + 层级历史 + 产物 + 召回）
    #[serde(default)]
    pub 任务标识: 任务标识,
    #[serde(default)]
    pub 层级历史: Vec<层级记录>,
    #[serde(default)]
    pub 回退来源: Option<回退记录>,
    #[serde(default)]
    pub 召回标记: bool,
    #[serde(default)]
    pub 当前层级: 五行层级,
    // 新增：扫尾记录（太乙金仙清理后交付证据链的机器核验结论，None=未检查）
    #[serde(default)]
    pub 扫尾记录: Option<扫尾记录>,
    // 新增：澄清记录（回退到木层后道祖的需求纠偏结论，None=未澄清）
    #[serde(default)]
    pub 澄清记录: Option<澄清记录>,
    // 新增：审核记录（道祖终审后最终审核结论，None=未审核）
    #[serde(default)]
    pub 审核记录: Option<审核记录>,
    // 新增：任务级临时规则（流态第三态：驱动执行期间注入智能体上下文，任务终态后清除）
    #[serde(default)]
    pub 临时规则: Vec<String>,
}

impl Task {
    pub fn 新建(id: u64, title: String, description: String, created_at: u64) -> Self {
        Task {
            id,
            title,
            description,
            status: TaskStatus::待受理,
            created_at,
            场景: TaskScene::default(),
            优先级: TaskPriority::default(),
            发起人: AgentRole::default(),
            当前承接人: AgentRole::default(),
            承接历史: Vec::new(),
            需求文档: None,
            设计文档: None,
            实现文档: None,
            验收文档: None,
            终审文档: None,
            状态历史: Vec::new(),
            总令牌: 0,
            修复轮次: 0,
            updated_at: created_at,
            任务标识: 任务标识::default(),
            层级历史: Vec::new(),
            回退来源: None,
            召回标记: false,
            当前层级: 五行层级::木,
            扫尾记录: None,
            澄清记录: None,
            审核记录: None,
            临时规则: Vec::new(),
        }
    }
}
