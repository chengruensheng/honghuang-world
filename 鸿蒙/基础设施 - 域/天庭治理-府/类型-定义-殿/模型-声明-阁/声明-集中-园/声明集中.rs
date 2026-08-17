//! 天庭治理-府 · 核心类型：组织编排的数据契约（想法→要求→设计→验收→版本→进化）。
//!
//! 依据：多智能体架构设计 §13。

use serde::{Deserialize, Serialize};

// ── 基础枚举 ──

/// 世界生长阶段。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 阶段 { 甲, 乙 }

/// 世界进入路径。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 进入路径 { 从零创建, 半路接手, 版本回退 }

/// 要求来源。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 要求来源 { 界主, 天道巡世, 鸿钧自主 }

/// 要求类别。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 要求类别 { 功能, 性能, 美观, 优化, 维护, 新能力, 补基础 }

/// 要求书状态机（八态）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 要求状态 {
    待领, 设计中, 待确认, 已确认, 待实现, 实现中, 已验收, 已存档,
}

/// 验收结论。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 验收结论 { 通过, 打回 }

/// 缺陷归属层（打回必填）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 缺陷层 { 设计层, 实现层 }

/// 进化调整级别（①-⑤）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 调整级别 { 调提示词, 调分工, 调流程, 重组团队, 结构级 }

/// 优先级。
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum 优先级 {
    #[default]
    低,
    中,
    高,
}

/// 项目成熟度。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 成熟度 { 成熟完整, 半成品, 损坏需修复 }

/// 想法状态。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 想法状态 { 未处理, 已化为要求, 已打回, 已解决 }

/// 构建状态。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 构建状态 { 可编译, 不可编译(String) }

// ── 核心结构 ──

/// 想法（界主产出）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 想法 {
    pub id: String,
    pub 内容: String,
    pub 时间: u64,
    pub 状态: 想法状态,
}

/// 约束（含路径避让，用于在途冲突规避）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 约束 {
    pub 涉及路径: Vec<String>,
    pub 不允许: Vec<String>,
    pub 优先级: 优先级,
}

/// 要求书（鸿钧产出）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 要求书 {
    pub id: String,
    pub 来源: 要求来源,
    pub 想法id: Option<String>,
    pub 阶段: 阶段,
    pub 方向: String,
    pub 类别: 要求类别,
    pub 验收标准: String,
    pub 约束: 约束,
    pub 状态: 要求状态,
    pub 确认意见: Option<String>,
    pub 验收: Option<验收回执>,
    pub 版本: Option<String>,
}

/// 拆解项（底层任务的输入）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 拆解项 {
    pub 目标: String,
    pub 执行层角色: Vec<String>,
    pub 工作流: String,
}

/// 设计方案（中层产出）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 设计方案 {
    pub 要求id: String,
    pub 设计: String,
    pub 拆解: Vec<拆解项>,
    pub 自评: String,
}

/// 产物条目。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 产物条目 {
    pub 路径: String,
    pub 类别: String,
    pub 字节数: u64,
    /// 相对本轮执行前基线指纹的变化类型：新增 | 修改 | 未变（设计稿 §12 阶段四 P0-1）。
    /// serde 默认「未变」向后兼容旧记录；未变文件不进产物清单。
    #[serde(default = "默认变化类型")]
    pub 变化类型: String,
}

/// 变化类型 默认值：未变（旧记录反序列化兜底）。
fn 默认变化类型() -> String {
    "未变".to_string()
}

/// 验收回执（鸿钧产出）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 验收回执 {
    pub 要求id: String,
    pub 结论: 验收结论,
    pub 验收意见: Option<String>,
    pub 产物: Vec<产物条目>,
    pub 耗时秒: f64,
}

/// 版本记录。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 版本记录 {
    pub 版本号: String,
    pub 时间: u64,
    pub 阶段: 阶段,
    pub 改了什么: String,
    pub 源码快照路径: String,
    pub 构建产物路径: String,
    pub 验收结论: Vec<String>,
    pub 对比上一版: String,
}

/// 巡世候选（改进点）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 巡世候选 {
    pub 目标: String,
    pub 依据: String,
    pub 建议类别: 要求类别,
    pub 优先级: 优先级,
}

/// 法则违逆（六层/命名/契约违规）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 法则违逆 {
    pub 路径: String,
    pub 违逆内容: String,
    pub 依据规则: String,
}

/// 巡世报告（天道产出）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 巡世报告 {
    pub id: String,
    pub 时间: u64,
    pub 候选: Vec<巡世候选>,
    pub 违逆: Vec<法则违逆>,
}

/// 失败条目（供进化环归因）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 失败条目 {
    pub 要求id: String,
    pub 阶段: String,
    pub 原因: String,
    pub 所在层: 缺陷层,
    pub 次数: u32,
    pub 时间: u64,
}

/// 进化记录（世界历史书）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 进化记录 {
    pub id: String,
    pub 级别: 调整级别,
    pub 触发原因: String,
    pub 前后对比: String,
    pub 效果回执: Option<String>,
    pub 时间: u64,
}

/// 规模统计。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 规模统计 {
    pub rs文件数: u32,
    pub 总行数: u64,
    pub crate数: u32,
}

/// 项目档案（半路接手产出）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 项目档案 {
    pub 来源: String,
    pub 接手时间: u64,
    pub 规模: 规模统计,
    pub 结构地图: String,
    pub 关键接口: Vec<String>,
    pub 构建状态: 构建状态,
    pub 风格约定: String,
    pub 已知坑: Vec<String>,
    pub 成熟度: 成熟度,
    pub 基线版本: String,
    /// 最近任务成功率（生产化 2.3）："通过 6/10 · 60%"（最近 10 条验收），serde 默认兼容旧档案。
    #[serde(default)]
    pub 最近任务成功率: String,
}

/// 世界状态（全局唯一，原子落盘）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 世界状态 {
    pub 阶段: 阶段,
    pub v1已存档: bool,
    pub 进入路径: 进入路径,
    pub 长期记忆: String,
    pub 界主想法池: Vec<想法>,
    pub 在途要求: Vec<要求书>,
    pub 验收历史: Vec<验收回执>,
    pub 失败模式: Vec<失败条目>,
    pub 版本历史: Vec<版本记录>,
    pub 巡世候选池: Vec<巡世候选>,
    pub 项目档案: Option<项目档案>,
    pub 天道报告库: Vec<巡世报告>,
}

/// 任务线状态（阶段 3 多任务线机制，设计稿 §1.5.5）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 任务线状态 {
    待执行,
    执行中,
    已完成,
    已中止,
}

/// 任务线：一次对话发布的任务单元，落盘 .上下文/状态/任务线.jsonl。
/// 守护模式消费待执行任务线；状态推进 待执行→执行中 先到先得（锁文件互斥，防并发双跑）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 任务线 {
    /// "任务线-<毫秒>"。
    pub id: String,
    pub 想法id: String,
    pub 想法内容: String,
    /// 执行中回填（主政一轮内部生成的要求 id）。
    pub 要求id: Option<String>,
    pub 状态: 任务线状态,
    /// 已完成时的验收结论（通过/打回）。
    pub 结论: Option<String>,
    /// 完成时的鸿钧汇报文本（落对话记录，界主追问可见）。
    pub 汇报: String,
    pub 时间: u64,
}