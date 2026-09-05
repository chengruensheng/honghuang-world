#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};
    use hm_agent::智能体;
    use hm_contract::Component;
    use hm_content_contract::{工具对话器, 对话消息, 工具调用, 模型响应};
    use hm_error::{Error, Result};
    use hm_execute_contract::{开发事件, 开发事件类型, 执行器};

    /// 模拟对话器：按预设序列依次返回模型响应，验证契约可插拔
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

    /// 模拟执行器：记录每个工具的调用参数并返回预设内容
    struct 模拟执行器 {
        读文件记录: Mutex<Vec<String>>,
        写文件记录: Mutex<Vec<(String, String)>>,
        命令记录: Mutex<Vec<String>>,
        读文件返回: String,
        命令返回: String,
    }

    impl 模拟执行器 {
        fn 新() -> Self {
            模拟执行器 {
                读文件记录: Mutex::new(Vec::new()),
                写文件记录: Mutex::new(Vec::new()),
                命令记录: Mutex::new(Vec::new()),
                读文件返回: "文件内容".to_string(),
                命令返回: "命令输出".to_string(),
            }
        }
    }

    impl Component for 模拟执行器 {
        fn name(&self) -> &'static str { "模拟执行器" }
    }

    impl 执行器 for 模拟执行器 {
        fn 读文件(&self, 路径: &str) -> Result<String> {
            self.读文件记录.lock().expect("模拟执行器读记录 锁中毒").push(路径.to_string());
            Ok(self.读文件返回.clone())
        }

        fn 写文件(&self, 路径: &str, 内容: &str) -> Result<()> {
            self.写文件记录.lock().expect("模拟执行器写记录 锁中毒").push((路径.to_string(), 内容.to_string()));
            Ok(())
        }

        fn 运行命令(&self, 命令: &str) -> Result<String> {
            self.命令记录.lock().expect("模拟执行器命令记录 锁中毒").push(命令.to_string());
            Ok(self.命令返回.clone())
        }
    }

    /// 构造工具调用（参数为 JSON 字符串）
    fn 工具调用(名称: &str, 参数: &str) -> 工具调用 {
        工具调用 { id: "call_1".into(), 名称: 名称.into(), 参数: 参数.into() }
    }

    #[test]
    fn 智能体_工具调用后返回最终答复() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"hello.txt"}"#)] },
            模型响应 { 内容: Some("任务完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("读取文件".into()).expect("运行应成功");
        assert_eq!(答复, "任务完成");
        assert_eq!(执行器_记录.读文件记录.lock().expect("锁").len(), 1);
    }

    #[test]
    fn 智能体_载荷传递_写文件路径与内容正确() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("写文件", r#"{"路径":"a.txt","内容":"你好"}"#)] },
            模型响应 { 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        智能体.运行("写入文件".into()).expect("运行应成功");
        let 写记录 = 执行器_记录.写文件记录.lock().expect("锁");
        assert_eq!(写记录.len(), 1);
        assert_eq!(写记录[0].0, "a.txt");
        assert_eq!(写记录[0].1, "你好");
    }

    #[test]
    fn 智能体_参数键中英兼容_英文path映射到路径() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("读文件", r#"{"path":"hello.txt"}"#)] },
            模型响应 { 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("读取文件".into()).expect("运行应成功");
        assert_eq!(答复, "完成");
        let 读记录 = 执行器_记录.读文件记录.lock().expect("锁");
        assert_eq!(*读记录, vec!["hello.txt".to_string()]);
    }

    #[test]
    fn 智能体_参数键中英兼容_英文content映射到内容() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("写文件", r#"{"path":"a.txt","content":"你好"}"#)] },
            模型响应 { 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        智能体.运行("写入文件".into()).expect("运行应成功");
        let 写记录 = 执行器_记录.写文件记录.lock().expect("锁");
        assert_eq!(写记录.len(), 1);
        assert_eq!(写记录[0].0, "a.txt");
        assert_eq!(写记录[0].1, "你好");
    }

    #[test]
    fn 智能体_参数键中英兼容_英文command映射到命令() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("运行命令", r#"{"command":"cargo build"}"#)] },
            模型响应 { 内容: Some("构建完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("构建项目".into()).expect("运行应成功");
        assert_eq!(答复, "构建完成");
        assert_eq!(*执行器_记录.命令记录.lock().expect("锁"), vec!["cargo build".to_string()]);
    }

    #[test]
    fn 智能体_多轮循环_连续调用多个工具() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 {
                内容: None,
                工具调用: vec![
                    工具调用("读文件", r#"{"路径":"a.txt"}"#),
                    工具调用("运行命令", r#"{"命令":"cargo build"}"#),
                ],
            },
            模型响应 { 内容: Some("构建完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("构建项目".into()).expect("运行应成功");
        assert_eq!(答复, "构建完成");
        assert_eq!(执行器_记录.读文件记录.lock().expect("锁").len(), 1);
        assert_eq!(*执行器_记录.命令记录.lock().expect("锁"), vec!["cargo build".to_string()]);
    }

    #[test]
    fn 智能体_超过最大轮数返回错误() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 2);

        let 结果 = 智能体.运行("永不完成".into());
        assert!(结果.is_err());
        assert!(结果.expect_err("应返回错误").to_string().contains("超过最大轮数"));
    }

    #[test]
    fn 智能体_未知工具_回填错误并继续循环() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("不存在的工具", "{}")] },
            模型响应 { 内容: Some("错误已处理".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        let 答复 = 智能体.运行("未知工具".into()).expect("工具错误不应终止循环");
        assert_eq!(答复, "错误已处理");
    }

    #[test]
    fn 智能体_参数缺失_回填错误并继续循环() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("写文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 内容: Some("参数已修正".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("缺参数".into()).expect("参数缺失不应终止循环");
        assert_eq!(答复, "参数已修正");
        assert!(执行器_记录.写文件记录.lock().expect("锁").is_empty());
    }

    #[test]
    fn 智能体_收到中断请求停止循环() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);
        智能体.中断句柄().store(true, Ordering::SeqCst);

        let 结果 = 智能体.运行("测试中断".into());
        assert!(结果.is_err());
        assert!(结果.expect_err("应返回错误").to_string().contains("中断"));
    }

    #[test]
    fn 智能体_事件回调按序发出且内容非空() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 内容: Some("任务完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 事件收集: Arc<Mutex<Vec<开发事件>>> = Arc::new(Mutex::new(Vec::new()));
        let 收集句柄 = 事件收集.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10)
            .设置事件回调(Arc::new(move |事件: &开发事件| {
                收集句柄.lock().expect("事件收集锁").push(事件.clone());
            }));

        let 答复 = 智能体.运行("读取文件并答复".into()).expect("运行应成功");
        assert_eq!(答复, "任务完成");

        let 事件 = 事件收集.lock().expect("事件收集锁");
        assert_eq!(事件.len(), 5, "应有 轮0(思考+调用+结果) + 轮1(思考+答复) 五条事件");

        assert_eq!(事件[0].类型, 开发事件类型::思考);
        assert_eq!(事件[0].轮次, 0);
        assert!(事件[0].工具名.is_empty());

        assert_eq!(事件[1].类型, 开发事件类型::工具调用);
        assert_eq!(事件[1].工具名, "读文件");
        assert!(事件[1].内容.contains("a.txt"));

        assert_eq!(事件[2].类型, 开发事件类型::工具结果);
        assert_eq!(事件[2].工具名, "读文件");
        assert!(事件[2].内容.contains("文件内容"));

        assert_eq!(事件[3].类型, 开发事件类型::思考);
        assert_eq!(事件[3].轮次, 1);

        assert_eq!(事件[4].类型, 开发事件类型::任务答复);
        assert_eq!(事件[4].轮次, 1);
        assert_eq!(事件[4].内容, "任务完成");

        assert!(事件.iter().all(|e| !e.内容.is_empty()), "全部事件内容应非空");
    }
}