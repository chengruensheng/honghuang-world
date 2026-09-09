use hm_agui::{
    Event, ReasoningMessageContent, ReasoningMessageEnd, ReasoningMessageStart, RunError,
    RunFinished, StepFinished, TextMessageContent, TextMessageEnd, TextMessageStart,
    ToolCallArgs, ToolCallEnd, ToolCallResult, ToolCallStart,
};
use crate::{驱动过程事件, 驱动阶段事件};

/// 协议适配器：把内部驱动事件翻译为 AG-UI 标准事件序列（适配器映射，非破坏）。
///
/// 有状态：维护「最近一次工具调用 toolCallId」游标，供工具结果配对；
/// 会话 id 由调用方传入，保证 runId/threadId/messageId/toolCallId 确定性、可回放。
pub struct 协议适配器 {
    最近工具调用id: Option<String>,
}

impl 协议适配器 {
    /// 新建适配器（游标为空）
    pub fn 新() -> Self {
        协议适配器 { 最近工具调用id: None }
    }

    /// 单条过程事件 → 一条或多条 AG-UI 事件（单条内部事件展开为三段式流）。
    ///
    /// 映射：思考→REASONING_MESSAGE_*；工具调用→TOOL_CALL_START/ARGS/END；
    /// 工具结果→TOOL_CALL_RESULT（复用最近工具调用 id，孤立时按序号兜底）；
    /// 任务答复→TEXT_MESSAGE_*。
    pub fn 过程事件(&mut self, 事件: &驱动过程事件, 会话id: u64) -> Vec<Event> {
        let 序号 = 事件.序号;
        match 事件.类型.as_str() {
            "思考" => {
                let id = format!("msg-{会话id}-{序号}");
                vec![
                    Event::ReasoningMessageStart(ReasoningMessageStart { message_id: id.clone() }),
                    Event::ReasoningMessageContent(ReasoningMessageContent {
                        message_id: id.clone(),
                        delta: 事件.内容.clone(),
                    }),
                    Event::ReasoningMessageEnd(ReasoningMessageEnd { message_id: id }),
                ]
            }
            "工具调用" => {
                let id = format!("tc-{会话id}-{序号}");
                self.最近工具调用id = Some(id.clone());
                vec![
                    Event::ToolCallStart(ToolCallStart {
                        tool_call_id: id.clone(),
                        tool_call_name: 事件.工具名.clone(),
                        parent_message_id: None,
                    }),
                    Event::ToolCallArgs(ToolCallArgs {
                        tool_call_id: id.clone(),
                        delta: 事件.内容.clone(),
                    }),
                    Event::ToolCallEnd(ToolCallEnd { tool_call_id: id }),
                ]
            }
            "工具结果" => {
                let id = self
                    .最近工具调用id
                    .clone()
                    .unwrap_or_else(|| format!("tc-{会话id}-{序号}"));
                vec![Event::ToolCallResult(ToolCallResult {
                    tool_call_id: id,
                    message_id: None,
                    content: 事件.内容.clone(),
                })]
            }
            "任务答复" => {
                let id = format!("msg-{会话id}-{序号}");
                vec![
                    Event::TextMessageStart(TextMessageStart {
                        message_id: id.clone(),
                        role: Some("assistant".into()),
                    }),
                    Event::TextMessageContent(TextMessageContent {
                        message_id: id.clone(),
                        delta: 事件.内容.clone(),
                    }),
                    Event::TextMessageEnd(TextMessageEnd { message_id: id }),
                ]
            }
            _ => vec![],
        }
    }

    /// 单条阶段事件 → 一条 AG-UI 事件。
    ///
    /// 映射：阶段完成→STEP_FINISHED；空闲→RUN_FINISHED；错误→RUN_ERROR。
    pub fn 阶段事件(&self, 事件: &驱动阶段事件, 会话id: u64) -> Vec<Event> {
        match 事件.类型.as_str() {
            "阶段完成" => vec![Event::StepFinished(StepFinished {
                step_name: 事件.新状态.clone().unwrap_or_else(|| "阶段".into()),
            })],
            "空闲" => vec![Event::RunFinished(RunFinished {
                thread_id: format!("thread-{会话id}"),
                run_id: format!("run-{会话id}"),
                result: None,
            })],
            "错误" => vec![Event::RunError(RunError {
                message: 事件.消息.clone().unwrap_or_else(|| "驱动错误".into()),
                code: Some("DRIVE_ERROR".into()),
            })],
            _ => vec![],
        }
    }
}
