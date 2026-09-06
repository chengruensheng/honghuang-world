#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use hm_agent::{道祖接待, 会话阶段};
    use hm_content_contract::{工具对话器, 对话消息, 工具调用, 模型响应};
    use hm_contract::Component;
    use hm_error::{Error, Result};

    /// 模拟对话器：按预设序列依次返回模型响应，验证道祖接待契约可插拔
    struct 模拟对话器 {
        响应序列: Mutex<VecDeque<模型响应>>,
    }

    impl 模拟对话器 {
        fn 新(序列: Vec<模型响应>) -> Self {
            模拟对话器 { 响应序列: Mutex::new(序列.into()) }
        }
    }

    impl Component for 模拟对话器 {
        fn name(&self) -> &'static str { "模拟对话器" }
    }

    impl 工具对话器 for 模拟对话器 {
        fn 对话(&self, _消息: Vec<对话消息>, _工具: Vec<serde_json::Value>) -> Result<模型响应> {
            let mut 序列 = self.响应序列.lock().expect("模拟对话器 锁中毒");
            序列.pop_front().ok_or_else(|| Error::Other("对话序列已耗尽".into()))
        }
    }

    /// 构造工具调用（参数为 JSON 字符串）
    fn 工具调用(名称: &str, 参数: &str) -> 工具调用 {
        工具调用 { id: "call_1".into(), 名称: 名称.into(), 参数: 参数.into() }
    }

    /// 构造「对齐总结」模型响应（结构化需求摘要）
    fn 对齐总结响应(标题: &str, 描述: &str, 场景: &str, 优先级: &str) -> 模型响应 {
        let 参数 = serde_json::json!({
            "标题": 标题,
            "描述": 描述,
            "场景": 场景,
            "优先级": 优先级,
        })
        .to_string();
        模型响应 { 内容: None, 工具调用: vec![工具调用("对齐总结", &参数)] }
    }

    /// 构造「追问澄清」模型响应
    fn 追问澄清响应(问题: &str) -> 模型响应 {
        let 参数 = serde_json::json!({ "问题": 问题 }).to_string();
        模型响应 { 内容: None, 工具调用: vec![工具调用("追问澄清", &参数)] }
    }

    /// 构造「闲聊」模型响应
    fn 闲聊响应(回复: &str) -> 模型响应 {
        let 参数 = serde_json::json!({ "回复": 回复 }).to_string();
        模型响应 { 内容: None, 工具调用: vec![工具调用("闲聊", &参数)] }
    }

    /// 唯一临时持久化路径（避免多测试共享临时文件导致竞态）
    fn 临时路径(标记: &str) -> std::path::PathBuf {
        let 毫秒 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("系统时间异常")
            .as_nanos();
        std::env::temp_dir().join(format!("道祖接待测试_{}_{}", 标记, 毫秒))
    }

    #[test]
    fn 道祖接待_对齐总结生成待确认需求摘要() {
        let 模拟 = Arc::new(模拟对话器::新(vec![对齐总结响应(
            "实现登录功能",
            "为用户提供账号密码登录，并校验会话",
            "设计",
            "P0",
        )]));
        let 接待 = 道祖接待::新(模拟);

        let 结果 = 接待.接待("我要一个登录功能".into()).expect("接待应成功");

        assert_eq!(结果.阶段, 会话阶段::待确认);
        let 需求 = 结果.需求.expect("对齐总结后应有需求摘要");
        assert_eq!(需求.标题, "实现登录功能");
        assert!(需求.描述.contains("账号密码登录"));
        assert_eq!(需求.场景.as_deref(), Some("设计"));
        assert_eq!(需求.优先级.as_deref(), Some("P0"));
        assert!(结果.回复.contains("实现登录功能"), "回复应包含任务标题");

        let 待确认 = 接待.待确认需求().expect("会话应保留待确认需求");
        assert_eq!(待确认.标题, "实现登录功能");
    }

    #[test]
    fn 道祖接待_追问澄清不进待确认且返回问题() {
        let 模拟 = Arc::new(模拟对话器::新(vec![追问澄清响应(
            "这个功能需要支持哪些登录方式？",
        )]));
        let 接待 = 道祖接待::新(模拟);

        let 结果 = 接待.接待("我想做个登录".into()).expect("接待应成功");

        assert_eq!(结果.阶段, 会话阶段::接待中);
        assert!(结果.需求.is_none(), "追问澄清不应生成需求摘要");
        assert!(结果.回复.contains("登录方式"), "回复应包含追问的问题内容");
        assert!(接待.待确认需求().is_none());
    }

    #[test]
    fn 道祖接待_闲聊不识别任务() {
        let 模拟 = Arc::new(模拟对话器::新(vec![闲聊响应("道友今日可好？")]));
        let 接待 = 道祖接待::新(模拟);

        let 结果 = 接待.接待("你好".into()).expect("接待应成功");

        assert_eq!(结果.阶段, 会话阶段::接待中);
        assert!(结果.需求.is_none());
        assert!(结果.回复.contains("道友今日可好"));
    }

    #[test]
    fn 道祖接待_确认发布后清空待确认需求() {
        let 模拟 = Arc::new(模拟对话器::新(vec![对齐总结响应(
            "修复崩溃问题",
            "定位并修复启动崩溃",
            "调试",
            "P1",
        )]));
        let 接待 = 道祖接待::新(模拟);
        接待.接待("修复崩溃".into()).expect("接待应成功");
        assert!(接待.待确认需求().is_some());

        let 需求 = 接待.确认发布().expect("确认发布应返回待确认需求");
        assert_eq!(需求.标题, "修复崩溃问题");
        assert!(接待.待确认需求().is_none(), "发布后待确认需求应清空");
    }

    #[test]
    fn 道祖接待_持久化往返保留待确认需求() {
        let 路径 = 临时路径("持久化");
        {
            let 模拟 = Arc::new(模拟对话器::新(vec![对齐总结响应(
                "数据导出",
                "导出任务数据为 CSV",
                "设计",
                "P2",
            )]));
            let mut 接待 = 道祖接待::新(模拟);
            接待.设置存储路径(路径.clone());
            接待.接待("做一个导出功能".into()).expect("接待应成功");
        }

        let 模拟 = Arc::new(模拟对话器::新(vec![]));
        let 恢复 = 道祖接待::加载(模拟, 路径.clone()).expect("加载应成功");
        let 待确认 = 恢复.待确认需求().expect("加载后应恢复待确认需求");
        assert_eq!(待确认.标题, "数据导出");
        assert_eq!(待确认.场景.as_deref(), Some("设计"));
        assert_eq!(待确认.优先级.as_deref(), Some("P2"));

        let _ = std::fs::remove_file(路径);
    }

    #[test]
    fn 道祖接待_加载不存在的路径返回空会话() {
        let 模拟 = Arc::new(模拟对话器::新(vec![]));
        let 路径 = 临时路径("不存在");
        let 接待 = 道祖接待::加载(模拟, 路径).expect("加载不存在的路径应返回空会话");
        assert!(接待.待确认需求().is_none());
    }

    #[test]
    fn 道祖接待_空消息返回错误() {
        let 模拟 = Arc::new(模拟对话器::新(vec![]));
        let 接待 = 道祖接待::新(模拟);
        let 结果 = 接待.接待("   ".into());
        assert!(结果.is_err(), "空消息应返回错误");
    }

    #[test]
    fn 道祖接待_对齐总结缺标题返回错误() {
        let 参数 = serde_json::json!({ "描述": "只有描述没有标题" }).to_string();
        let 模拟 = Arc::new(模拟对话器::新(vec![模型响应 {
            内容: None,
            工具调用: vec![工具调用("对齐总结", &参数)],
        }]));
        let 接待 = 道祖接待::新(模拟);
        let 结果 = 接待.接待("来个任务".into());
        assert!(结果.is_err(), "对齐总结缺标题应返回错误");
    }
}