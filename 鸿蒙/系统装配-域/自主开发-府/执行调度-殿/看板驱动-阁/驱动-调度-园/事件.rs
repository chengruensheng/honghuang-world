use serde::{Deserialize, Serialize};

/// 一条驱动阶段事件记录（事件接口可查询；预留新一轮时清空）
#[derive(Debug, Clone, Serialize)]
pub struct 驱动阶段事件 {
    /// 本次驱动会话内单调序号（从 1 起，预留时重置）
    pub 序号: u64,
    /// 空闲 / 阶段完成 / 错误
    pub 类型: String,
    /// 被推进的任务 id（空闲/错误为 None）
    pub 任务id: Option<u64>,
    /// 承接角色显示名
    pub 角色: Option<String>,
    /// 提交后的新状态显示名
    pub 新状态: Option<String>,
    /// 提交后的五行层级标签（木/火/土/金/水，权威映射）
    pub 层级: Option<String>,
    /// 补充消息（错误详情等）
    pub 消息: Option<String>,
    /// 事件发生秒级时间戳
    pub 时间: u64,
}

/// 一次驱动的结果摘要（状态接口可查询）
#[derive(Debug, Clone, Serialize)]
pub struct 驱动阶段摘要 {
    /// 空闲 / 阶段完成 / 错误
    pub 类型: String,
    /// 被推进的任务 id（空闲/错误为 None）
    pub 任务id: Option<u64>,
    /// 承接角色显示名（道祖/圣人/大罗金仙/准圣）
    pub 角色: Option<String>,
    /// 提交后的新状态显示名
    pub 新状态: Option<String>,
    /// 提交后的五行层级标签（木/火/土/金/水，权威映射）
    pub 层级: Option<String>,
    /// 补充消息（错误详情等）
    pub 消息: Option<String>,
}

/// 一条驱动过程事件（智能体循环每步：思考/工具调用/工具结果/任务答复）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 驱动过程事件 {
    /// 本次驱动会话内单调序号（从 1 起，预留时重置）
    pub 序号: u64,
    /// 被推进的任务 id
    pub 任务id: Option<u64>,
    /// 承接角色显示名
    pub 角色: Option<String>,
    /// 承接阶段的五行层级标签（木/火/土/金/水）：由承接状态经 状态层级标签 权威映射得出，
    /// 供按层结构化统计；旧会话文件无此字段，反序列化缺省为 None
    #[serde(default)]
    pub 层级: Option<String>,
    /// 智能体循环第几轮（从 0 起）
    pub 轮次: usize,
    /// 事件类型：思考 / 工具调用 / 工具结果 / 任务答复
    pub 类型: String,
    /// 工具名（思考/答复类为空串）
    pub 工具名: String,
    /// 事件内容（参数、结果摘要或答复文本，已截断200字符）
    pub 内容: String,
    /// 事件发生秒级时间戳
    pub 时间: u64,
}

/// 看板驱动台状态汇总（状态接口返回体）
#[derive(Debug, Serialize)]
pub struct 看板驱动状态 {
    /// 五层协作驱动器是否已装配
    pub 就绪: bool,
    /// 是否有一轮驱动执行中
    pub 运行中: bool,
    /// 最近一次驱动结果摘要
    pub 最近阶段: Option<驱动阶段摘要>,
    /// 最近一次驱动的结果说明
    pub 最近结果: Option<String>,
}
