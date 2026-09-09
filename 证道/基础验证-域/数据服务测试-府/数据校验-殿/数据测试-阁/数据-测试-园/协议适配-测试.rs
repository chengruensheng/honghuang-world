#[cfg(test)]
mod tests {
    use hm_http::{协议适配器, 驱动过程事件, 驱动阶段事件};
    use serde_json::json;

    /// 断言某 AG-UI 事件序列化结果与预期 JSON 逐字节一致
    macro_rules! 断言事件 {
        ($事件:expr, $预期:expr) => {
            assert_eq!(serde_json::to_value($事件).unwrap(), $预期)
        };
    }

    /// 构造一条驱动过程事件
    fn 过程事件(序号: u64, 类型: &str, 内容: &str, 工具名: &str) -> 驱动过程事件 {
        驱动过程事件 {
            序号,
            任务id: Some(5),
            角色: Some("圣人".into()),
            轮次: 0,
            类型: 类型.into(),
            工具名: 工具名.into(),
            内容: 内容.into(),
            时间: 123,
        }
    }

    /// 构造一条驱动阶段事件
    fn 阶段事件(类型: &str, 新状态: Option<&str>, 消息: Option<&str>) -> 驱动阶段事件 {
        驱动阶段事件 {
            序号: 0,
            类型: 类型.into(),
            任务id: Some(5),
            角色: Some("圣人".into()),
            新状态: 新状态.map(|s| s.into()),
            层级: None,
            消息: 消息.map(|s| s.into()),
            时间: 123,
        }
    }

    #[test]
    fn 思考_映射为推理三段式() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(1, "思考", "先分析需求", ""), 42);
        assert_eq!(结果.len(), 3);
        断言事件!(&结果[0], json!({"type": "REASONING_MESSAGE_START", "messageId": "msg-42-1"}));
        断言事件!(&结果[1], json!({"type": "REASONING_MESSAGE_CONTENT", "messageId": "msg-42-1", "delta": "先分析需求"}));
        断言事件!(&结果[2], json!({"type": "REASONING_MESSAGE_END", "messageId": "msg-42-1"}));
    }

    #[test]
    fn 工具调用_映射为工具三段式() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(3, "工具调用", "{\"path\":\"a.rs\"}", "写文件"), 42);
        assert_eq!(结果.len(), 3);
        断言事件!(&结果[0], json!({"type": "TOOL_CALL_START", "toolCallId": "tc-42-3", "toolCallName": "写文件"}));
        断言事件!(&结果[1], json!({"type": "TOOL_CALL_ARGS", "toolCallId": "tc-42-3", "delta": "{\"path\":\"a.rs\"}"}));
        断言事件!(&结果[2], json!({"type": "TOOL_CALL_END", "toolCallId": "tc-42-3"}));
    }

    #[test]
    fn 工具结果_复用最近工具调用id() {
        let mut 适配器 = 协议适配器::新();
        适配器.过程事件(&过程事件(3, "工具调用", "参数", "写文件"), 42);
        let 结果 = 适配器.过程事件(&过程事件(4, "工具结果", "写入成功", ""), 42);
        assert_eq!(结果.len(), 1);
        断言事件!(&结果[0], json!({"type": "TOOL_CALL_RESULT", "toolCallId": "tc-42-3", "content": "写入成功"}));
    }

    #[test]
    fn 孤立工具结果_按序号兜底() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(4, "工具结果", "结果", ""), 42);
        断言事件!(&结果[0], json!({"type": "TOOL_CALL_RESULT", "toolCallId": "tc-42-4", "content": "结果"}));
    }

    #[test]
    fn 任务答复_映射为文本三段式() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(6, "任务答复", "已完成", ""), 42);
        assert_eq!(结果.len(), 3);
        断言事件!(&结果[0], json!({"type": "TEXT_MESSAGE_START", "messageId": "msg-42-6", "role": "assistant"}));
        断言事件!(&结果[1], json!({"type": "TEXT_MESSAGE_CONTENT", "messageId": "msg-42-6", "delta": "已完成"}));
        断言事件!(&结果[2], json!({"type": "TEXT_MESSAGE_END", "messageId": "msg-42-6"}));
    }

    #[test]
    fn 未知类型_返回空序列() {
        let mut 适配器 = 协议适配器::新();
        assert!(适配器.过程事件(&过程事件(1, "未知", "x", ""), 42).is_empty());
    }

    #[test]
    fn 阶段完成_映射为步骤结束() {
        let 适配器 = 协议适配器::新();
        let 结果 = 适配器.阶段事件(&阶段事件("阶段完成", Some("待圣人设计"), None), 42);
        assert_eq!(结果.len(), 1);
        断言事件!(&结果[0], json!({"type": "STEP_FINISHED", "stepName": "待圣人设计"}));
    }

    #[test]
    fn 空闲_映射为运行结束() {
        let 适配器 = 协议适配器::新();
        let 结果 = 适配器.阶段事件(&阶段事件("空闲", None, None), 42);
        断言事件!(&结果[0], json!({"type": "RUN_FINISHED", "threadId": "thread-42", "runId": "run-42"}));
    }

    #[test]
    fn 错误_映射为运行错误() {
        let 适配器 = 协议适配器::新();
        let 结果 = 适配器.阶段事件(&阶段事件("错误", None, Some("LLM 超时")), 42);
        断言事件!(&结果[0], json!({"type": "RUN_ERROR", "message": "LLM 超时", "code": "DRIVE_ERROR"}));
    }
}
