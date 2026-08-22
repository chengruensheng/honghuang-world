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

/// 角色生命周期状态（对齐 Cordis 插件生命周期：登记→就绪→生效→卸载）。
/// 阶段 3 角色插件化（融合蓝图 §14.10）：登记 = 入册未校验；就绪 = 依赖可用；生效 = 副作用已注册；卸载 = 卡已移除。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 角色状态 {
    已登记,
    已就绪,
    已生效,
    已卸载,
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

/// 产物类别：执行产出的种类（替代裸字符串，集中枚举语义）。
/// 序列化以变体名直出（"代码"/"脚本"/"文档"/"配置"/"其他"），向后兼容旧 jsonl 记录。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum 产物类别 {
    #[default]
    代码,
    脚本,
    文档,
    配置,
    其他,
}

impl std::fmt::Display for 产物类别 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            产物类别::代码 => "代码",
            产物类别::脚本 => "脚本",
            产物类别::文档 => "文档",
            产物类别::配置 => "配置",
            产物类别::其他 => "其他",
        }
        .fmt(f)
    }
}

/// 变化类型：相对本轮执行前基线指纹的变化（新增 | 修改 | 未变）。
/// 序列化以变体名直出，向后兼容旧 jsonl 记录。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum 变化类型 {
    新增,
    修改,
    #[default]
    未变,
}

impl std::fmt::Display for 变化类型 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            变化类型::新增 => "新增",
            变化类型::修改 => "修改",
            变化类型::未变 => "未变",
        }
        .fmt(f)
    }
}

/// 产物条目：执行产出的一个文件 / 脚本 / 文档。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 产物条目 {
    pub 路径: String,
    pub 类别: 产物类别,
    pub 字节数: u64,
    /// 相对本轮执行前基线指纹的变化类型：新增 | 修改 | 未变。
    /// serde 默认「未变」向后兼容旧记录；未变文件不进产物清单。
    #[serde(default)]
    pub 变化类型: 变化类型,
}

/// 执行回执：一次执行的结果、产物清单、token 用量与轮数。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 执行回执 {
    pub 状态: 执行状态,
    pub 产物们: Vec<产物条目>,
    pub 说明: String,
    /// 本次执行累计的 token 用量（读现状 + 工具循环 + 重试全部计入）。
    pub 用量: moxing_fu::用量,
    /// 本次回执消耗的工具循环轮数（跨重试累计；首调起算）。
    pub 轮数: u32,
}
