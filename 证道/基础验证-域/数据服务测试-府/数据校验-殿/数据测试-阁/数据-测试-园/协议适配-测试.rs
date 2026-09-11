#[cfg(test)]
mod tests {
    use hm_http::{协议适配器, 思考链过滤器, 驱动过程事件, 驱动阶段事件};
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
    fn 任务答复_思考转入推理通道正文只留可见部分() {
        let mut 适配器 = 协议适配器::新();
        let 输入 = "<think>The user wants me to act as 圣人。\n内部推理草稿</think>\n{\"结论\":\"完成\"}";
        let 结果 = 适配器.过程事件(&过程事件(7, "任务答复", 输入, ""), 42);
        assert_eq!(结果.len(), 7, "思考与正文各成一条消息：推理三段 + 文本三段 + 开棒");
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
        断言事件!(&结果[1], json!({"type": "REASONING_MESSAGE_START", "messageId": "推理-42-7"}));
        断言事件!(&结果[2], json!({"type": "REASONING_MESSAGE_CONTENT", "messageId": "推理-42-7", "delta": "The user wants me to act as 圣人。\n内部推理草稿"}));
        断言事件!(&结果[3], json!({"type": "REASONING_MESSAGE_END", "messageId": "推理-42-7"}));
        断言事件!(&结果[5], json!({"type": "TEXT_MESSAGE_CONTENT", "messageId": "msg-42-7", "delta": "{\"结论\":\"完成\"}"}));
    }

    #[test]
    fn 任务答复_全是思考链则只发推理段不发空答复() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(8, "任务答复", "<think>只有草稿，没有正文</think>", ""), 42);
        assert_eq!(结果.len(), 4, "正文为空则不发文本消息；思考仍进推理通道，不丢弃");
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
        断言事件!(&结果[1], json!({"type": "REASONING_MESSAGE_START", "messageId": "推理-42-8"}));
        断言事件!(&结果[2], json!({"type": "REASONING_MESSAGE_CONTENT", "messageId": "推理-42-8", "delta": "只有草稿，没有正文"}));
    }

    #[test]
    fn 任务答复_未闭合思考链其后内容归入推理段不作正文() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(10, "任务答复", "<think>草稿被截断，闭合标签也一起没了", ""), 42);
        assert_eq!(结果.len(), 4, "未闭合即视其后为思考：不得当正文下发，但也须留证");
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
        断言事件!(&结果[2], json!({"type": "REASONING_MESSAGE_CONTENT", "messageId": "推理-42-10", "delta": "草稿被截断，闭合标签也一起没了"}));
    }

    #[test]
    fn 思考内容_剥除思考链后方下发() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(9, "思考", "<THINK>大写标签也要剥</THINK>真正要说的", ""), 42);
        assert_eq!(结果.len(), 4);
        断言事件!(&结果[2], json!({"type": "REASONING_MESSAGE_CONTENT", "messageId": "msg-42-9", "delta": "真正要说的"}));
    }

    /// 逐块喂入流式过滤器，拼出（思考、正文）两侧的完整分流结果（含收尾）
    fn 流式分流(块s: &[&str]) -> (String, String) {
        let mut 滤器 = 思考链过滤器::新();
        let mut 思 = String::new();
        let mut 正 = String::new();
        for 块 in 块s {
            let 出 = 滤器.喂(块);
            思.push_str(&出.思考);
            正.push_str(&出.正文);
        }
        let 尾 = 滤器.收尾();
        思.push_str(&尾.思考);
        正.push_str(&尾.正文);
        (思, 正)
    }

    #[test]
    fn 流式分流_标签跨块时思考与正文各归其位() {
        // 上游按块切：`<think>` 成了 `<thi`+`nk>`，`</think>` 成了 `</thi`+`nk>`
        assert_eq!(
            流式分流(&["<thi", "nk>内部草稿", "</thi", "nk>正文在此"]),
            ("内部草稿".into(), "正文在此".into())
        );
    }

    #[test]
    fn 流式分流_未闭合思考链其后内容归入思考不作正文() {
        assert_eq!(
            流式分流(&["正文", "<think>草稿被截断", "仍属草稿"]),
            ("草稿被截断仍属草稿".into(), "正文".into())
        );
    }

    #[test]
    fn 流式分流_多段思考链与大小写标签() {
        assert_eq!(
            流式分流(&["<THINK>甲</THINK>正文一", "<think>乙</think>正文二"]),
            ("甲乙".into(), "正文一正文二".into())
        );
    }

    #[test]
    fn 流式分流_块内思考与正文并存时互不混入() {
        assert_eq!(
            流式分流(&["<think>甲</think>正文"]),
            ("甲".into(), "正文".into())
        );
    }

    #[test]
    fn 流式分流_疑似标签前缀并非标签时按正文吐出() {
        // `<t` 是 `<think>` 的前缀，须暂存待定；下一块证明它其实是 `<tag>`
        assert_eq!(
            流式分流(&["看这个 <t", "ag> 是标签"]),
            (String::new(), "看这个 <tag> 是标签".into())
        );
    }

    #[test]
    fn 流式分流_流结束时残留的前缀照常吐出() {
        // 流停在 `<th` 就结束了：它已确定不是标签，收尾须把它还给用户，不能吞掉
        assert_eq!(流式分流(&["正文<th"]), (String::new(), "正文<th".into()));
    }

    #[test]
    fn 未知类型_只开棒不产内容() {
        let mut 适配器 = 协议适配器::新();
        let 结果 = 适配器.过程事件(&过程事件(1, "未知", "x", ""), 42);
        assert_eq!(结果.len(), 1);
        断言事件!(&结果[0], json!({"type": "RUN_STARTED", "threadId": "thread-42", "runId": "run-42-1"}));
    }

    #[test]
    fn 阶段完成_映射为状态流转与步骤结束() {
        let 适配器 = 协议适配器::新();
        let 结果 = 适配器.阶段事件(&阶段事件("阶段完成", Some("待大罗金仙实现"), None), 42);
        assert_eq!(结果.len(), 2);
        // 新状态是状态机的确定性产物，归 STATE_DELTA：补丁指向该任务的 status
        断言事件!(&结果[0], json!({
            "type": "STATE_DELTA",
            "delta": [{"op": "replace", "path": "/任务/5/status", "value": "待大罗金仙实现"}],
        }));
        // 步骤名归「角色 · 职责」（不再是冒充步骤名的状态名）
        断言事件!(&结果[1], json!({"type": "STEP_FINISHED", "stepName": "圣人 · 边界契约设计"}));
    }

    #[test]
    fn 阶段完成_缺新状态时只发步骤结束() {
        // 补丁指向不了任务的状态就是编的：宁可只报步骤结束，也不发一条无目标的 STATE_DELTA
        let 适配器 = 协议适配器::新();
        let 结果 = 适配器.阶段事件(&阶段事件("阶段完成", None, None), 42);
        assert_eq!(结果.len(), 1);
        断言事件!(&结果[0], json!({"type": "STEP_FINISHED", "stepName": "圣人 · 边界契约设计"}));
    }

    #[test]
    fn 阶段完成_角色不在五层之列时步骤名兜底为非空() {
        let 适配器 = 协议适配器::新();
        let mut 事件 = 阶段事件("阶段完成", None, None);
        事件.角色 = Some("无此角色".into());
        let 结果 = 适配器.阶段事件(&事件, 42);
        断言事件!(&结果[0], json!({"type": "STEP_FINISHED", "stepName": "阶段"}));
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
