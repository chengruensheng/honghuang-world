// hm-agui：AG-UI（Agent-User Interaction Protocol）事件契约镜像层。
//
// 本 crate 是对 AG-UI 协议标准事件的 1:1 类型映射，Rust 标识符（enum/struct/字段）
// 对齐协议英文词（threadId/messageId/toolCallId/delta），序列化后与官方 SSE 帧逐字节一致；
// 文件与目录命名仍遵循项目中文命名规范（域/府/殿/阁/园）。

#[path = "事件词表-殿/模块.rs"]
mod 事件词表_殿;
#[path = "思考推理-殿/模块.rs"]
mod 思考推理_殿;
#[path = "状态同步-殿/模块.rs"]
mod 状态同步_殿;

pub use 事件词表_殿::*;
pub use 思考推理_殿::*;
pub use 状态同步_殿::*;

use serde::{Deserialize, Serialize};

/// AG-UI 事件：以 `type` 字段做判别（邻接标记枚举）。
///
/// `rename_all = "SCREAMING_SNAKE_CASE"` 使 variant 名自动映射为标准事件词，
/// 例如 `TextMessageContent` → `"TEXT_MESSAGE_CONTENT"`，序列化无需手动 rename。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    /// 一次 agent run 开始（建立 run/thread 上下文）
    RunStarted(RunStarted),
    /// 一次 agent run 成功结束
    RunFinished(RunFinished),
    /// 不可恢复错误，终止 run
    RunError(RunError),
    /// 命名步骤开始（可选，进度可见性）
    StepStarted(StepStarted),
    /// 命名步骤结束
    StepFinished(StepFinished),
    /// 流式文本消息开始
    TextMessageStart(TextMessageStart),
    /// 文本消息内容增量
    TextMessageContent(TextMessageContent),
    /// 文本消息结束
    TextMessageEnd(TextMessageEnd),
    /// 工具调用开始
    ToolCallStart(ToolCallStart),
    /// 工具调用参数增量
    ToolCallArgs(ToolCallArgs),
    /// 工具调用结束
    ToolCallEnd(ToolCallEnd),
    /// 工具执行结果
    ToolCallResult(ToolCallResult),
    /// 推理（思考）消息开始
    ReasoningMessageStart(ReasoningMessageStart),
    /// 推理消息内容增量
    ReasoningMessageContent(ReasoningMessageContent),
    /// 推理消息结束
    ReasoningMessageEnd(ReasoningMessageEnd),
    /// 状态全量快照
    StateSnapshot(StateSnapshot),
    /// 状态增量（RFC 6902 JSON Patch）
    StateDelta(StateDelta),
}
