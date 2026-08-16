//! 落回-解析-园 · 模型调用测试：验证请求体构造与响应解析。

#[cfg(test)]
mod 测试 {
    use moxing_fu::{模型配置, 模型回复, 工具定义, 对话消息};
    use moxing_fu::{构造工具请求体, 构造请求体, 解析回复, 解析工具回复};

    #[test]
    fn 构造请求体含模型与消息() {
        let 配置 = 模型配置 {
            密钥: "k".to_string(),
            地址: "https://example.com/v1/chat/completions".to_string(),
            模型: "MiniMax-M3".to_string(),
        };
        let 体 = 构造请求体(&配置, &[对话消息::用户("你好")], moxing_fu::常规上限);
        assert!(体.contains("MiniMax-M3"));
        assert!(体.contains("你好"));
    }

    #[test]
    fn 解析回复取内容() {
        let 文本 = r#"{"choices":[{"message":{"content":"回答"}}]}"#;
        assert_eq!(解析回复(文本).unwrap(), "回答");
    }

    #[test]
    fn 解析回复空内容返回错误() {
        // 空串与纯空白 content 都视为空回复，不得静默放行（防写空记忆/空 JSON 报错/空文本收敛）。
        let 空串 = r#"{"choices":[{"message":{"content":""}}]}"#;
        assert!(解析回复(空串).unwrap_err().contains("空内容"), "空串应报空内容错误");
        let 纯空白 = r#"{"choices":[{"message":{"content":"  \n  "}}]}"#;
        assert!(解析回复(纯空白).unwrap_err().contains("空内容"), "纯空白应报空内容错误");
        let 缺字段 = r#"{"choices":[{"message":{}}]}"#;
        assert!(解析回复(缺字段).is_err(), "缺 content 字段应报错");
    }

    #[test]
    fn 构造工具请求体含工具() {
        let 配置 = 模型配置 {
            密钥: "k".to_string(),
            地址: "https://example.com/v1/chat/completions".to_string(),
            模型: "MiniMax-M3".to_string(),
        };
        let 工具 = 工具定义 {
            名字: "落盘文件".to_string(),
            描述: "写入文件".to_string(),
            参数: serde_json::json!({"type": "object"}),
        };
        let 体 = 构造工具请求体(&配置, &[对话消息::用户("写文件")], &[工具], moxing_fu::常规上限);
        assert!(体.contains("tools"));
        assert!(体.contains("落盘文件"));
        assert!(体.contains("type"));
    }

    #[test]
    fn 解析工具回复取调用带标识() {
        let 文本 = r#"{"choices":[{"message":{"role":"assistant","content":"<think>思考</think>","tool_calls":[{"id":"1","function":{"name":"落盘文件","arguments":"{\"文件们\":[]}"}}]}}]}"#;
        let 回复 = 解析工具回复(文本).unwrap();
        assert!(matches!(&回复, 模型回复::工具调用(内容, 调用们) if 内容 == "<think>思考</think>" && 调用们.len() == 1 && 调用们[0].名字 == "落盘文件" && 调用们[0].标识 == "1"));
    }

    #[test]
    fn 解析工具回复空参返回参数缺失() {
        let 文本 = r#"{"choices":[{"message":{"role":"assistant","tool_calls":[{"id":"1","function":{"name":"读文件","arguments":"{}"}},{"id":"2","function":{"name":"读文件","arguments":""}}]}}]}"#;
        let 回复 = 解析工具回复(文本).unwrap();
        assert!(matches!(&回复, 模型回复::参数缺失(名字们) if 名字们 == &["读文件", "读文件"]));
    }

    #[test]
    fn 工具请求体回传调用与结果() {
        let 配置 = 模型配置 {
            密钥: "k".to_string(),
            地址: "https://example.com/v1/chat/completions".to_string(),
            模型: "MiniMax-M3".to_string(),
        };
        let 工具 = 工具定义 {
            名字: "写文件".to_string(),
            描述: "写入文件".to_string(),
            参数: serde_json::json!({"type": "object"}),
        };
        // 模拟一轮工具往返：助手回传调用 + tool 角色回传结果
        let 消息们 = vec![
            对话消息::助手_带工具调用("", vec![moxing_fu::工具调用 {
                标识: "call_1".to_string(),
                名字: "写文件".to_string(),
                参数: r#"{"路径":"a.rs","内容":"x"}"#.to_string(),
            }]),
            对话消息::工具结果("call_1", "已写入 a.rs"),
        ];
        let 体 = 构造工具请求体(&配置, &消息们, &[工具], moxing_fu::常规上限);
        assert!(体.contains("tool_calls"));
        assert!(体.contains("call_1"));
        assert!(体.contains("\"role\":\"tool\"") || 体.contains("tool_call_id"));
        assert!(体.contains("已写入 a.rs"));
    }

    #[test]
    fn 解析工具回复取调用() {
        let 文本 = r#"{"choices":[{"message":{"role":"assistant","tool_calls":[{"id":"1","function":{"name":"落盘文件","arguments":"{\"文件们\":[]}"}}]}}]}"#;
        let 回复 = 解析工具回复(文本).unwrap();
        assert!(matches!(&回复, 模型回复::工具调用(_, 调用们) if 调用们.len() == 1 && 调用们[0].名字 == "落盘文件"));
    }

    #[test]
    fn 解析工具回复无工具时取文本() {
        let 文本 = r#"{"choices":[{"message":{"content":"直接回答"}}]}"#;
        let 回复 = 解析工具回复(文本).unwrap();
        assert!(matches!(&回复, 模型回复::文本(段) if 段 == "直接回答"));
    }
}
