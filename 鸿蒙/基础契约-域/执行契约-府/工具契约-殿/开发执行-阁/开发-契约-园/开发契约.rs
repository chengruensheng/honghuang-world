use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use hm_contract::Component;
use hm_error::Result;

/// 开发事件类型：智能体循环的四个观察点
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 开发事件类型 {
    /// 每轮开始，LLM 正在思考下一步
    思考,
    /// LLM 发起一次工具调用
    工具调用,
    /// 工具执行返回（含失败回填）
    工具结果,
    /// 循环结束，LLM 给出最终答复
    任务答复,
}

/// 开发执行过程中的一个事件（供外部观察循环进度，内容已在智能体侧截断）
#[derive(Debug, Clone)]
pub struct 开发事件 {
    /// 第几轮（从 0 起）
    pub 轮次: usize,
    /// 事件类型
    pub 类型: 开发事件类型,
    /// 工具名（思考/答复类事件为空串）
    pub 工具名: String,
    /// 事件内容（参数、结果摘要或答复文本）
    pub 内容: String,
}

/// 开发执行契约：受理一个开发任务并同步执行 LLM 循环，支持中断。
///
/// 生产路径注入 hm-agent 智能体，测试路径注入 mock，无特权。
/// 同步阻塞设计：调用方（HTTP 受理台）负责放入后台线程。
pub trait 开发执行契约: Component {
    /// 同步执行开发任务，返回 LLM 最终答复（阻塞调用线程直至完成或中断）
    fn 执行开发任务(&self, 任务: String) -> Result<String>;
    /// 中断句柄：外部置位 true 后，智能体在下一轮开头停止
    fn 中断句柄(&self) -> Arc<AtomicBool>;
}