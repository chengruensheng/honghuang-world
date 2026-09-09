#[cfg(test)]
mod tests {
    use hm_agui::*;
    use serde_json::{json, Value};

    /// 序列化为 JSON 值，便于逐字段断言
    fn 序列化(事件: &Event) -> Value {
        serde_json::to_value(事件).unwrap()
    }

    // ---- 逐字节断言：type 词表 + camelCase 字段 + None 字段省略 ----

    #[test]
    fn 文本消息内容_输出标准词表与驼峰字段() {
        let 事件 = Event::TextMessageContent(TextMessageContent {
            message_id: "msg-接待-1".into(),
            delta: "好的".into(),
        });
        assert_eq!(
            序列化(&事件),
            json!({"type": "TEXT_MESSAGE_CONTENT", "messageId": "msg-接待-1", "delta": "好的"})
        );
    }

    #[test]
    fn 运行开始_省略空的可选字段() {
        let 事件 = Event::RunStarted(RunStarted {
            thread_id: "thread-0".into(),
            run_id: "run-接待".into(),
            parent_run_id: None,
            input: None,
        });
        assert_eq!(
            序列化(&事件),
            json!({"type": "RUN_STARTED", "threadId": "thread-0", "runId": "run-接待"})
        );
    }

    #[test]
    fn 工具调用开始_携带工具名() {
        let 事件 = Event::ToolCallStart(ToolCallStart {
            tool_call_id: "tc-1".into(),
            tool_call_name: "写文件".into(),
            parent_message_id: None,
        });
        assert_eq!(
            序列化(&事件),
            json!({"type": "TOOL_CALL_START", "toolCallId": "tc-1", "toolCallName": "写文件"})
        );
    }

    #[test]
    fn 推理消息内容_输出推理词表() {
        let 事件 = Event::ReasoningMessageContent(ReasoningMessageContent {
            message_id: "m1".into(),
            delta: "思考中".into(),
        });
        assert_eq!(
            序列化(&事件),
            json!({"type": "REASONING_MESSAGE_CONTENT", "messageId": "m1", "delta": "思考中"})
        );
    }

    #[test]
    fn 状态增量_承载嵌套补丁() {
        let 事件 = Event::StateDelta(StateDelta {
            delta: vec![json!({"op": "replace", "path": "/progress", "value": 75})],
        });
        assert_eq!(
            序列化(&事件),
            json!({"type": "STATE_DELTA", "delta": [{"op": "replace", "path": "/progress", "value": 75}]})
        );
    }

    #[test]
    fn 状态快照_承载任意嵌套() {
        let 事件 = Event::StateSnapshot(StateSnapshot {
            snapshot: json!({"五行": {"当前": "木"}, "任务数": 3}),
        });
        assert_eq!(
            序列化(&事件),
            json!({"type": "STATE_SNAPSHOT", "snapshot": {"五行": {"当前": "木"}, "任务数": 3}})
        );
    }

    // ---- 往返一致性 ----

    #[test]
    fn 反序列化_往返一致() {
        let 原文 = json!({"type": "TEXT_MESSAGE_CONTENT", "messageId": "m", "delta": "你好"});
        let 事件: Event = serde_json::from_value(原文.clone()).unwrap();
        assert_eq!(serde_json::to_value(&事件).unwrap(), 原文);
    }

    #[test]
    fn 反序列化_工具结果带可选消息id() {
        let 原文 = json!({"type": "TOOL_CALL_RESULT", "toolCallId": "tc-9", "messageId": "msg-9", "content": "成功"});
        let 事件: Event = serde_json::from_value(原文.clone()).unwrap();
        assert_eq!(serde_json::to_value(&事件).unwrap(), 原文);
    }

    // ---- 全 17 variant 词表映射 ----

    #[test]
    fn 全部事件词表_映射标准() {
        let 词表: Vec<(Event, &str)> = vec![
            (Event::RunStarted(RunStarted { thread_id: "t".into(), run_id: "r".into(), parent_run_id: None, input: None }), "RUN_STARTED"),
            (Event::RunFinished(RunFinished { thread_id: "t".into(), run_id: "r".into(), result: None }), "RUN_FINISHED"),
            (Event::RunError(RunError { message: "m".into(), code: None }), "RUN_ERROR"),
            (Event::StepStarted(StepStarted { step_name: "s".into() }), "STEP_STARTED"),
            (Event::StepFinished(StepFinished { step_name: "s".into() }), "STEP_FINISHED"),
            (Event::TextMessageStart(TextMessageStart { message_id: "m".into(), role: None }), "TEXT_MESSAGE_START"),
            (Event::TextMessageContent(TextMessageContent { message_id: "m".into(), delta: "d".into() }), "TEXT_MESSAGE_CONTENT"),
            (Event::TextMessageEnd(TextMessageEnd { message_id: "m".into() }), "TEXT_MESSAGE_END"),
            (Event::ToolCallStart(ToolCallStart { tool_call_id: "t".into(), tool_call_name: "n".into(), parent_message_id: None }), "TOOL_CALL_START"),
            (Event::ToolCallArgs(ToolCallArgs { tool_call_id: "t".into(), delta: "d".into() }), "TOOL_CALL_ARGS"),
            (Event::ToolCallEnd(ToolCallEnd { tool_call_id: "t".into() }), "TOOL_CALL_END"),
            (Event::ToolCallResult(ToolCallResult { tool_call_id: "t".into(), message_id: None, content: "c".into() }), "TOOL_CALL_RESULT"),
            (Event::ReasoningMessageStart(ReasoningMessageStart { message_id: "m".into() }), "REASONING_MESSAGE_START"),
            (Event::ReasoningMessageContent(ReasoningMessageContent { message_id: "m".into(), delta: "d".into() }), "REASONING_MESSAGE_CONTENT"),
            (Event::ReasoningMessageEnd(ReasoningMessageEnd { message_id: "m".into() }), "REASONING_MESSAGE_END"),
            (Event::StateSnapshot(StateSnapshot { snapshot: json!({"a": 1}) }), "STATE_SNAPSHOT"),
            (Event::StateDelta(StateDelta { delta: vec![] }), "STATE_DELTA"),
        ];
        for (事件, 预期词) in 词表 {
            assert_eq!(序列化(&事件)["type"], json!(预期词), "type 词表不符: {}", 预期词);
        }
    }
}
