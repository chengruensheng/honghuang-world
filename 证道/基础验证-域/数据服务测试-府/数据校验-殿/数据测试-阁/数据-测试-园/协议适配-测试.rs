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

    /// 构造一条驱动过程事件（角色固定为「圣人」）
    fn 过程事件(序号: u64, 类型: &str, 内容: &str, 工具名: &str) -> 驱动过程事件 {
        某角色过程事件(序号, "圣人", 类型, 内容, 工具名)
    }

    /// 构造一条指定角色的驱动过程事件
    fn 某角色过程事件(
        序号: u64,
        角色: &str,
        类型: &str,
        内容: &str,
        工具名: &str,
    ) -> 驱动过程事件 {
        驱动过程事件 {
            序号,
            任务id: Some(5),
            角色: Some(角色.into()),
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
    fn 角色首事件_先开棒再推理三段式() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(1, "思考", "先分析需求", ""), 42);
        assert_eq!(结果.len(), 4);
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
        断言事件!(&结果[1], json!({"type": "REASONING_MESSAGE_START", "messageId": "msg-42-1"}));
        断言事件!(&结果[2], json!({"type": "REASONING_MESSAGE_CONTENT", "messageId": "msg-42-1", "delta": "先分析需求"}));
        断言事件!(&结果[3], json!({"type": "REASONING_MESSAGE_END", "messageId": "msg-42-1"}));
    }

    #[test]
    fn 角色不变_不重复开棒() {
        let mut 适配器 = 协议适配器::新();
        适配器.过程事件(&过程事件(1, "思考", "甲", ""), 42);
        let 结果 = 适配器.过程事件(&过程事件(2, "思考", "乙", ""), 42);
        assert_eq!(结果.len(), 3);
        断言事件!(&结果[0], json!({"type": "REASONING_MESSAGE_START", "messageId": "msg-42-2"}));
    }

    #[test]
    fn 角色切换_开新棒且序号递增() {
        let mut 适配器 = 协议适配器::新();
        适配器.过程事件(&过程事件(1, "思考", "甲", ""), 42);
        let 结果 = 适配器.过程事件(&某角色过程事件(2, "大罗金仙", "工具调用", "{}", "写文件"), 42);
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-2"}));
        断言事件!(&结果[1], json!({"type": "TOOL_CALL_START", "toolCallId": "tc-42-2", "toolCallName": "写文件"}));
    }

    #[test]
    fn 思考标记_只开棒不下发假推理() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(1, "思考", "__思考开始__", ""), 42);
        assert_eq!(结果.len(), 1, "思考标记是动画信号，不得当推理内容下发");
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
    }

    #[test]
    fn 工具调用_映射为工具三段式() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(3, "工具调用", "{\"path\":\"a.rs\"}", "写文件"), 42);
        assert_eq!(结果.len(), 4);
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
        断言事件!(&结果[1], json!({"type": "TOOL_CALL_START", "toolCallId": "tc-42-3", "toolCallName": "写文件"}));
        断言事件!(&结果[2], json!({"type": "TOOL_CALL_ARGS", "toolCallId": "tc-42-3", "delta": "{\"path\":\"a.rs\"}"}));
        断言事件!(&结果[3], json!({"type": "TOOL_CALL_END", "toolCallId": "tc-42-3"}));
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
        assert_eq!(结果.len(), 2);
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
        断言事件!(&结果[1], json!({"type": "TOOL_CALL_RESULT", "toolCallId": "tc-42-4", "content": "结果"}));
    }

    #[test]
    fn 任务答复_映射为文本三段式() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(6, "任务答复", "已完成", ""), 42);
        assert_eq!(结果.len(), 4);
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
        断言事件!(&结果[1], json!({"type": "TEXT_MESSAGE_START", "messageId": "msg-42-6", "role": "assistant"}));
        断言事件!(&结果[2], json!({"type": "TEXT_MESSAGE_CONTENT", "messageId": "msg-42-6", "delta": "已完成"}));
        断言事件!(&结果[3], json!({"type": "TEXT_MESSAGE_END", "messageId": "msg-42-6"}));
    }

    #[test]
    fn 任务答复_剥除思考链标签() {
        let mut 适配器 = 协议适配器::新();
        let 输入 = "<think>The user wants me to act as 圣人。\n内部推理草稿</think>\n{\"结论\":\"完成\"}";
        let 结果 = 适配器.过程事件(&过程事件(7, "任务答复", 输入, ""), 42);
        assert_eq!(结果.len(), 4);
        断言事件!(&结果[2], json!({"type": "TEXT_MESSAGE_CONTENT", "messageId": "msg-42-7", "delta": "{\"结论\":\"完成\"}"}));
    }

    #[test]
    fn 任务答复_全是思考链则不下发空答复() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(8, "任务答复", "<think>只有草稿，没有正文</think>", ""), 42);
        assert_eq!(结果.len(), 1, "洗空后只剩开棒，不得下发一个空答复");
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
    }

    #[test]
    fn 任务答复_未闭合思考链整段丢弃() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(10, "任务答复", "<think>草稿被截断，闭合标签也一起没了", ""), 42);
        assert_eq!(结果.len(), 1, "未闭合的思考链此后全是草稿，不得当正文下发");
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
    }

    #[test]
    fn 思考内容_剥除思考链后方下发() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(9, "思考", "<THINK>大写标签也要剥</THINK>真正要说的", ""), 42);
        assert_eq!(结果.len(), 4);
        断言事件!(&结果[2], json!({"type": "REASONING_MESSAGE_CONTENT", "messageId": "msg-42-9", "delta": "真正要说的"}));
    }

    #[test]
    fn 未知类型_只开棒不产内容() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(1, "未知", "x", ""), 42);
        assert_eq!(结果.len(), 1);
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
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
