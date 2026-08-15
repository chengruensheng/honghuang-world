//! 道术施展-府 · 核心类型：执行角色、任务、工作流与产物契约。
//!
//! 铁律：只执行、不越级组织；大罗金仙分道执行、不跨道硬扛。

use serde::{Deserialize, Serialize};

/// 工作流级别：任务复杂度四档，决定执行的深度。
///
/// 对应 agent_fu 的 L1_qa / L2_script / L3_program / L4_complex。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 工作流级别 {
    问答,
    脚本,
    程序,
    复杂,
}

/// 执行角色：一张角色卡，声明谁以何道司何职。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 执行角色 {
    pub 身份: String,
    pub 道: String,
    pub 职司: String,
    pub 模型池: String,
    pub 契约: String,
}

/// 执行任务：道术施展-府接收的最小执行单元，不感知组织层。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 执行任务 {
    pub 目标: String,
    pub 工作流: 工作流级别,
    pub 角色们: Vec<String>,
}

/// 执行状态。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 执行状态 {
    运行中,
    成功,
    失败,
}

/// 产物条目：执行产出的一个文件 / 脚本 / 文档。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 产物条目 {
    pub 路径: String,
    pub 类别: String,
    pub 字节数: u64,
}

/// 执行回执：一次执行的结果与产物清单。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 执行回执 {
    pub 状态: 执行状态,
    pub 产物们: Vec<产物条目>,
    pub 说明: String,
}
