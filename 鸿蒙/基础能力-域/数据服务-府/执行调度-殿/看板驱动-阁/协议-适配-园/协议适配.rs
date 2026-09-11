use hm_agui::{
    Event, ReasoningMessageContent, ReasoningMessageEnd, ReasoningMessageStart, RunError,
    RunFinished, RunStarted, StepFinished, TextMessageContent, TextMessageEnd, TextMessageStart,
    ToolCallArgs, ToolCallEnd, ToolCallResult, ToolCallStart,
};
use crate::{驱动过程事件, 驱动阶段事件};

/// 协议适配器：把内部驱动事件翻译为 AG-UI 标准事件序列（适配器映射，非破坏）。
///
/// 有状态：维护「最近一次工具调用 toolCallId」游标，供工具结果配对；
/// 维护「当前角色」，角色一变即开一根新棒（发 RUN_STARTED）；
/// 会话 id 由调用方传入，保证 runId/threadId/messageId/toolCallId 确定性、可回放。
pub struct 协议适配器 {
    最近工具调用id: Option<String>,
    当前角色: Option<String>,
    角色序号: u64,
}

impl 协议适配器 {
    /// 新建适配器（游标为空）
    pub fn 新() -> Self {
        协议适配器 { 最近工具调用id: None, 当前角色: None, 角色序号: 0 }
    }

    /// 单条过程事件 → 一条或多条 AG-UI 事件（单条内部事件展开为三段式流）。
    ///
    /// 映射：角色切换→RUN_STARTED（一次 run = 一个角色的一段任期，五层接力即五棒）；
    /// 思考→REASONING_MESSAGE_*；工具调用→TOOL_CALL_START/ARGS/END；
    /// 工具结果→TOOL_CALL_RESULT（复用最近工具调用 id，孤立时按序号兜底）；
    /// 任务答复→TEXT_MESSAGE_*。
    pub fn 过程事件(&mut self, 事件: &驱动过程事件, 会话id: u64) -> Vec<Event> {
        let 序号 = 事件.序号;
        let mut 出: Vec<Event> = Vec::new();
        // 角色一变就是一根新棒：上一棒由「下一棒开头」隐式收尾，本棒以 RUN_STARTED 显式开头。
        // 缺此事件时客户端只能靠「角色字段变化」猜分段——协议里 run 有始无终，不合 AG-UI 词表。
        if let Some(角色) = 事件.角色.as_deref().filter(|r| !r.is_empty()) {
            if self.当前角色.as_deref() != Some(角色) {
                self.当前角色 = Some(角色.to_string());
                self.角色序号 += 1;
                self.最近工具调用id = None;   // 新棒不复用上一棒的工具 id
                出.push(Event::RunStarted(RunStarted {
                    thread_id: format!("thread-{会话id}"),
                    run_id: format!("run-{会话id}-{}", self.角色序号),
                    parent_run_id: None,
                    input: None,
                }));
            }
        }
        match 事件.类型.as_str() {
            "思考" => {
                // 两层过滤各管一段：动画标记是「假内容」，思考链标签是「模型草稿」。
                // 剥完为空就不下发——宁可这一层空着，也不下发杂质。
                let 内容 = 去思考链(&事件.内容);
                if !是思考标记(&事件.内容) && !内容.is_empty() {
                    let id = format!("msg-{会话id}-{序号}");
                    出.push(Event::ReasoningMessageStart(ReasoningMessageStart { message_id: id.clone() }));
                    出.push(Event::ReasoningMessageContent(ReasoningMessageContent {
                        message_id: id.clone(),
                        delta: 内容,
                    }));
                    出.push(Event::ReasoningMessageEnd(ReasoningMessageEnd { message_id: id }));
                }
            }
            "工具调用" => {
                let id = format!("tc-{会话id}-{序号}");
                self.最近工具调用id = Some(id.clone());
                出.push(Event::ToolCallStart(ToolCallStart {
                    tool_call_id: id.clone(),
                    tool_call_name: 事件.工具名.clone(),
                    parent_message_id: None,
                }));
                出.push(Event::ToolCallArgs(ToolCallArgs { tool_call_id: id.clone(), delta: 事件.内容.clone() }));
                出.push(Event::ToolCallEnd(ToolCallEnd { tool_call_id: id }));
            }
            "工具结果" => {
                let id = self
                    .最近工具调用id
                    .clone()
                    .unwrap_or_else(|| format!("tc-{会话id}-{序号}"));
                出.push(Event::ToolCallResult(ToolCallResult {
                    tool_call_id: id,
                    message_id: None,
                    content: 事件.内容.clone(),
                }));
            }
            "任务答复" => {
                // 推理模型（MiniMax-M3 等）把思考链直接写进 content，会随答复原样下发，
                // 界面上就是一段 `<think>The user wants me to act as...` 的模型草稿。
                // 骨架照发，内容必须是洗过的正文；洗空了就不发，免得多一个空气泡。
                let 内容 = 去思考链(&事件.内容);
                if !内容.is_empty() {
                    let id = format!("msg-{会话id}-{序号}");
                    出.push(Event::TextMessageStart(TextMessageStart {
                        message_id: id.clone(),
                        role: Some("assistant".into()),
                    }));
                    出.push(Event::TextMessageContent(TextMessageContent {
                        message_id: id.clone(),
                        delta: 内容,
                    }));
                    出.push(Event::TextMessageEnd(TextMessageEnd { message_id: id }));
                }
            }
            _ => {}
        }
        出
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

/// 循环模块发出的思考动画标记（「开始/结束」与每轮占位），不是 LLM 的真实思考内容。
///
/// 它们只用于界面转「思考中」的转圈动画；当作推理内容下发给客户端，
/// 会让「推理」这一层全是噪音——宁可这一层空着，也不下发假内容。
fn 是思考标记(内容: &str) -> bool {
    matches!(内容, "__思考开始__" | "__思考结束__" | "正在思考下一步行动")
}

/// 去掉模型写进正文的思考链（`<think>…</think>`），只留对用户可见的部分。
///
/// 大小写不敏感、支持多段。**未闭合时连同其后内容一并丢弃**：`<think>` 一旦没有闭合标签，
/// 其后就全是模型的思考区（上游按字节截断时最先被切掉的恰恰是闭合标签与正文），
/// 此时"保留正文"等于把草稿原样端给用户。宁可这一条内容空着，也不显示杂质。
/// 全程只在 ASCII 边界切分（标签是纯 ASCII），中文等多字节字符不受影响。
fn 去思考链(文本: &str) -> String {
    let mut 出 = String::with_capacity(文本.len());
    let mut 剩 = 文本;
    while let Some(起) = 找小写片段(剩, "<think>") {
        出.push_str(&剩[..起]);
        let 标签后 = 起 + "<think>".len();
        match 找小写片段(&剩[标签后..], "</think>") {
            Some(闭) => 剩 = &剩[标签后 + 闭 + "</think>".len()..],
            None => {
                剩 = "";
                break;
            }
        }
    }
    出.push_str(剩);
    出.trim().to_string()
}

/// 大小写不敏感地查找 ASCII 片段，返回字节起点。
///
/// 用 `to_ascii_lowercase` 而非 `to_lowercase`：前者只改 ASCII 字节，长度与原文一致，
/// 据此得到的偏移可直接用于原文字节切片；后者可能改变多字节字符长度，偏移会错位。
fn 找小写片段(文本: &str, 片段: &str) -> Option<usize> {
    文本.to_ascii_lowercase().find(片段)
}
