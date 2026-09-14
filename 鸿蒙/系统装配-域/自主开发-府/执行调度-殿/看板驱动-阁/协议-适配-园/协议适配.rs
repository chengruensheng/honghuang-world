use hm_agui::{
    Event, ReasoningMessageContent, ReasoningMessageEnd, ReasoningMessageStart, RunError,
    RunFinished, RunStarted, StateDelta, StepFinished, TextMessageContent, TextMessageEnd,
    TextMessageStart, ToolCallArgs, ToolCallEnd, ToolCallResult, ToolCallStart,
};
use tc_task::AgentRole;
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
                // 推理模型（MiniMax-M3 等）把思考链直接写进 content。它不该混在答复里，
                // 但也不该丢——转进推理通道，正文只留对用户可见的部分：各归其位。
                let mut 滤器 = 思考链过滤器::新();
                let 前段 = 滤器.喂(&事件.内容);
                let 尾段 = 滤器.收尾();
                let mut 思 = 前段.思考;
                思.push_str(&尾段.思考);
                let mut 正 = 前段.正文;
                正.push_str(&尾段.正文);
                let 思考 = 思.trim();
                let 正文 = 正.trim();
                if !思考.is_empty() {
                    let id = format!("推理-{会话id}-{序号}");
                    出.push(Event::ReasoningMessageStart(ReasoningMessageStart { message_id: id.clone() }));
                    出.push(Event::ReasoningMessageContent(ReasoningMessageContent {
                        message_id: id.clone(),
                        delta: 思考.to_string(),
                    }));
                    出.push(Event::ReasoningMessageEnd(ReasoningMessageEnd { message_id: id }));
                }
                if !正文.is_empty() {
                    let id = format!("msg-{会话id}-{序号}");
                    出.push(Event::TextMessageStart(TextMessageStart {
                        message_id: id.clone(),
                        role: Some("assistant".into()),
                    }));
                    出.push(Event::TextMessageContent(TextMessageContent {
                        message_id: id.clone(),
                        delta: 正文.to_string(),
                    }));
                    出.push(Event::TextMessageEnd(TextMessageEnd { message_id: id }));
                }
            }
            _ => {}
        }
        出
    }

    /// 单条阶段事件 → 一条或多条 AG-UI 事件。
    ///
    /// 映射：阶段完成→STATE_DELTA（新状态流转）+ STEP_FINISHED（步骤名）；
    /// 空闲→RUN_FINISHED；错误→RUN_ERROR。
    pub fn 阶段事件(&self, 事件: &驱动阶段事件, 会话id: u64) -> Vec<Event> {
        match 事件.类型.as_str() {
            "阶段完成" => {
                let mut 出: Vec<Event> = Vec::new();
                // 「新状态」是状态机（执行推进 + TaskStatus）的确定性产物，正是 STATE_DELTA 的语义。
                // 它此前被塞进 stepName，害得阶段条把状态名当步骤名（演示语料在这一位是「圣人 · 设计」）。
                // 新状态改由此处承载，前端「→ 下一步」直接读 value；取不到就不发，不编。
                if let (Some(任务id), Some(新状态)) = (事件.任务id, 事件.新状态.as_deref()) {
                    出.push(Event::StateDelta(StateDelta {
                        delta: vec![serde_json::json!({
                            "op": "replace",
                            "path": format!("/任务/{任务id}/status"),
                            "value": 新状态,
                        })],
                    }));
                }
                出.push(Event::StepFinished(StepFinished {
                    step_name: 步骤名(事件.角色.as_deref()),
                }));
                出
            }
            "空闲" => vec![Event::RunFinished(RunFinished {
                thread_id: format!("thread-{会话id}"),
                run_id: format!("run-{会话id}"),
                result: None,
            })],
            "错误" => vec![Event::RunError(RunError {
                message: 错误消息(事件),
                code: Some("DRIVE_ERROR".into()),
            })],
            _ => vec![],
        }
    }
}

/// 阶段步骤名：`角色 · 职责`（如「圣人 · 边界契约设计」）。
///
/// 职责取自 `AgentRole::职责`（与任务书提示词同一张表，不另抄一份）；
/// 角色缺失或不在五层之列时退回「阶段」，保持非空——AG-UI 要求步骤名非空。
fn 步骤名(角色名: Option<&str>) -> String {
    match 角色名.and_then(AgentRole::从名称) {
        Some(角色) => format!("{} · {}", 角色.名称(), 角色.职责()),
        None => "阶段".into(),
    }
}

/// 循环模块发出的思考动画标记（「开始/结束」与每轮占位），不是 LLM 的真实思考内容。
///
/// 它们只用于界面转「思考中」的转圈动画；当作推理内容下发给客户端，
/// 会让「推理」这一层全是噪音——宁可这一层空着，也不下发假内容。
fn 是思考标记(内容: &str) -> bool {
    matches!(内容, "__思考开始__" | "__思考结束__" | "正在思考下一步行动")
}

/// 错误消息带发生时刻前缀：事件流保留历史，
/// 无时刻的「运行错误」让人分不清是「现在坏了」还是「曾经坏过」。
fn 错误消息(事件: &驱动阶段事件) -> String {
    let 消息 = 事件.消息.clone().unwrap_or_else(|| "驱动错误".into());
    format!("[{}] {}", 时刻文本(事件.时间), 消息)
}

/// Unix 秒 → 北京时间 HH:mm:ss（UTC+8 固定偏移，不引外部时间库）。
fn 时刻文本(unix秒: u64) -> String {
    let 当日 = unix秒.rem_euclid(86_400) + 8 * 3600;
    let 当日 = 当日 % 86_400;
    format!("{:02}:{:02}:{:02}", 当日 / 3600, (当日 % 3600) / 60, 当日 % 60)
}

/// 去掉模型写进正文的思考链（`<think>…</think>`），只留对用户可见的部分。
///
/// 大小写不敏感、支持多段。**未闭合时连同其后内容一并丢弃**：`<think>` 一旦没有闭合标签，
/// 其后就全是模型的思考区（上游按字节截断时最先被切掉的恰恰是闭合标签与正文），
/// 此时"保留正文"等于把草稿原样端给用户。宁可这一条内容空着，也不显示杂质。
/// 口径与流式版 `思考链过滤器` 共用一份实现，防止两处剥离规则各自漂移。
fn 去思考链(文本: &str) -> String {
    let mut 滤器 = 思考链过滤器::新();
    let mut 出 = 滤器.喂(文本).正文;
    出.push_str(&滤器.收尾().正文);
    出.trim().to_string()
}

/// 一次投喂的分流结果：思考与正文各归其道。
///
/// 分流是为了「各归其位」，不是「择优保留」——思考有独立的协议通道
/// （`REASONING_MESSAGE_*`），丢掉等于把过程证据一并丢了；混进正文则等于把草稿端给用户。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct 分流结果 {
    /// 思考段内容（应走 REASONING_MESSAGE_*）
    pub 思考: String,
    /// 正文内容（应走 TEXT_MESSAGE_*）
    pub 正文: String,
}

/// 流式思考链过滤器：逐块喂入 LLM 增量，把思考与正文分开吐出。
///
/// 与 `去思考链` 同一口径（大小写不敏感、支持多段、未闭合即视其后为思考），
/// 区别只在「逐块」。上游是按块切的，标签本身可能被切在两个块里（`…<thi` + `nk>…`），
/// 见一块发一块必然漏标签，故把「疑似标签前缀」的尾巴暂存到下一块再判定；
/// 流结束用 `收尾` 让出最后一段。
pub struct 思考链过滤器 {
    在思考段: bool,
    暂存: String,
}

impl 思考链过滤器 {
    /// 新建（不在思考段、无暂存）
    pub fn 新() -> Self {
        思考链过滤器 { 在思考段: false, 暂存: String::new() }
    }

    /// 喂入一块增量 → 分流结果（某一侧为空串＝本块没有该侧内容）
    pub fn 喂(&mut self, 块: &str) -> 分流结果 {
        let mut 输入 = std::mem::take(&mut self.暂存);
        输入.push_str(块);
        let mut 出 = 分流结果::default();
        let mut 剩: &str = &输入;
        loop {
            if self.在思考段 {
                match 找小写片段(剩, "</think>") {
                    Some(闭) => {
                        出.思考.push_str(&剩[..闭]);
                        剩 = &剩[闭 + "</think>".len()..];
                        self.在思考段 = false;
                    }
                    None => {
                        // 除「可能是闭合标签前缀」的尾巴外，整段归思考
                        match 标签前缀尾(剩, "</think>") {
                            Some(尾) => {
                                出.思考.push_str(&剩[..剩.len() - 尾.len()]);
                                self.暂存 = 尾.to_string();
                            }
                            None => 出.思考.push_str(剩),
                        }
                        break;
                    }
                }
            } else if let Some(起) = 找小写片段(剩, "<think>") {
                出.正文.push_str(&剩[..起]);
                剩 = &剩[起 + "<think>".len()..];
                self.在思考段 = true;
            } else {
                match 标签前缀尾(剩, "<think>") {
                    Some(尾) => {
                        出.正文.push_str(&剩[..剩.len() - 尾.len()]);
                        self.暂存 = 尾.to_string();
                    }
                    None => 出.正文.push_str(剩),
                }
                break;
            }
        }
        出
    }

    /// 流结束：暂存的尾巴已确定不是标签前缀，按当前所处段归位
    pub fn 收尾(&mut self) -> 分流结果 {
        let 尾 = std::mem::take(&mut self.暂存);
        let mut 出 = 分流结果::default();
        if self.在思考段 { 出.思考 = 尾; } else { 出.正文 = 尾; }
        出
    }
}

/// 文本尾是否恰为 `标签` 的一个前缀（`<`、`<t`、…、`<thin`），返回该尾巴。
///
/// 要求切片落在字符边界上：多字节字符（中文）若被当成标签前缀滞留，
/// 那部分正文会白白扣在暂存里，直到下一块才吐出——短则错位，长则丢字。
fn 标签前缀尾<'a>(文本: &'a str, 标签: &str) -> Option<&'a str> {
    for 长 in (1..标签.len()).rev() {
        if 文本.len() < 长 { continue; }
        let 起 = 文本.len() - 长;
        if 文本.is_char_boundary(起) && 文本[起..].eq_ignore_ascii_case(&标签[..长]) {
            return Some(&文本[起..]);
        }
    }
    None
}

/// 大小写不敏感地查找 ASCII 片段，返回字节起点。
///
/// 用 `to_ascii_lowercase` 而非 `to_lowercase`：前者只改 ASCII 字节，长度与原文一致，
/// 据此得到的偏移可直接用于原文字节切片；后者可能改变多字节字符长度，偏移会错位。
fn 找小写片段(文本: &str, 片段: &str) -> Option<usize> {
    文本.to_ascii_lowercase().find(片段)
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 时刻_零点即北京八点() {
        assert_eq!(时刻文本(0), "08:00:00");
    }

    #[test]
    fn 时刻_跨日自动回绕() {
        // UTC 16:00:01（当日秒 57601）+ 8h → 次日 00:00:01
        assert_eq!(时刻文本(57_601), "00:00:01");
    }

    #[test]
    fn 错误消息_带时刻前缀且缺省文案() {
        let 事件 = 驱动阶段事件 {
            序号: 1,
            类型: "错误".into(),
            任务id: None,
            角色: None,
            新状态: None,
            层级: None,
            消息: None,
            时间: 0,
        };
        assert_eq!(错误消息(&事件), "[08:00:00] 驱动错误");
    }
}
