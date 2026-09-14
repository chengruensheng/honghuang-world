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
        fn 对话(&self, 消息: Vec<对话消息>, _工具: Vec<serde_json::Value>) -> Result<模型响应> {
            // 命令安全审计是旁路独立会话（system 提示含「命令安全审查员」）：
            // 固定回一条安全结论，不消耗主序列——审计只做观察，不得改变主循环的对话节奏。
            let 是审计请求 = 消息
                .iter()
                .any(|条| 条.内容.as_deref().map_or(false, |文| 文.contains("命令安全审查员")));
            if 是审计请求 {
                return Ok(模型响应 {
                    思考: None,
                    内容: Some(r#"{"安全": true, "风险": "安全", "理由": "模拟放行"}"#.into()),
                    工具调用: vec![],
                });
            }
            let mut 序列 = self.响应序列.lock().expect("模拟对话器 锁中毒");
            序列.pop_front().ok_or_else(|| Error::Other("对话序列已耗尽".into()))
        }
    }

    /// 模拟执行器：记录每个工具的调用参数并返回预设内容
    struct 模拟执行器 {
        读文件记录: Mutex<Vec<String>>,
        写文件记录: Mutex<Vec<(String, String)>>,
        命令记录: Mutex<Vec<String>>,
        删除文件记录: Mutex<Vec<String>>,
        列目录记录: Mutex<Vec<String>>,
        读文件返回: String,
        命令返回: String,
    }

    impl 模拟执行器 {
        fn 新() -> Self {
            模拟执行器 {
                读文件记录: Mutex::new(Vec::new()),
                写文件记录: Mutex::new(Vec::new()),
                命令记录: Mutex::new(Vec::new()),
                删除文件记录: Mutex::new(Vec::new()),
                列目录记录: Mutex::new(Vec::new()),
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

        fn 列目录(&self, 路径: &str) -> Result<String> {
            self.列目录记录.lock().expect("模拟执行器列目录记录 锁中毒").push(路径.to_string());
            Ok("（空目录）".to_string())
        }

        fn 按名找文件(&self, _模式: &str) -> Result<String> {
            Ok("（无匹配）".to_string())
        }

        fn 搜索内容(&self, _关键词: &str) -> Result<String> {
            Ok("（无匹配）".to_string())
        }

        fn 精确编辑(&self, _路径: &str, _旧: &str, _新: &str) -> Result<String> {
            Ok("替换成功（1 处）".to_string())
        }

        fn 删除文件(&self, 路径: &str) -> Result<String> {
            self.删除文件记录.lock().expect("模拟执行器删除记录 锁中毒").push(路径.to_string());
            Ok(format!("已删除: {路径}"))
        }
    }

    /// 构造工具调用（参数为 JSON 字符串）
    fn 工具调用(名称: &str, 参数: &str) -> 工具调用 {
        工具调用 { id: "call_1".into(), 名称: 名称.into(), 参数: 参数.into() }
    }

    #[test]
    fn 智能体_工具调用后返回最终答复() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"hello.txt"}"#)] },
            模型响应 { 思考: None, 内容: Some("任务完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("读取文件".into()).expect("运行应成功");
        assert_eq!(答复, "任务完成");
        assert_eq!(执行器_记录.读文件记录.lock().expect("锁").len(), 1);
    }

    #[test]
    fn 智能体_删除文件工具调用分发成功() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("删除文件", r#"{"路径":"src/lib.rs.bak"}"#)] },
            模型响应 { 思考: None, 内容: Some("清理完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("删除残留备份".into()).expect("运行应成功");
        assert_eq!(答复, "清理完成");
        let 删除记录 = 执行器_记录.删除文件记录.lock().expect("锁");
        assert_eq!(*删除记录, vec!["src/lib.rs.bak".to_string()], "删除文件调用应分发到执行器");
    }

    #[test]
    fn 智能体_非备份删除须过审查_拒绝不放行() {
        // 非备份文件（.rs）删除须过独立 LLM 审查对话；拒绝后删除不得执行，主循环收到拒绝错误继续收尾
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("删除文件", r#"{"路径":"src/main.rs"}"#)] },
            模型响应 { 思考: None, 内容: Some(r#"{"可删": false, "理由": "源代码不可删"}"#.into()), 工具调用: vec![] },
            模型响应 { 思考: None, 内容: Some("遵命不删".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("删除源码".into()).expect("循环不应因审查拒绝终止");
        assert_eq!(答复, "遵命不删");
        assert!(执行器_记录.删除文件记录.lock().expect("锁").is_empty(), "审查拒绝应拦截删除");
    }

    #[test]
    fn 智能体_非备份删除审查通过后放行() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("删除文件", r#"{"路径":"产物/输出.tmp2"}"#)] },
            模型响应 { 思考: None, 内容: Some(r#"{"可删": true, "理由": "构建产物"}"#.into()), 工具调用: vec![] },
            模型响应 { 思考: None, 内容: Some("已清理".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        智能体.运行("清理产物".into()).expect("运行应成功");
        assert_eq!(
            *执行器_记录.删除文件记录.lock().expect("锁"),
            vec!["产物/输出.tmp2".to_string()],
            "审查通过后删除应分发到执行器"
        );
    }

    #[test]
    fn 智能体_载荷传递_写文件路径与内容正确() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("写文件", r#"{"路径":"a.txt","内容":"你好"}"#)] },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
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
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{"path":"hello.txt"}"#)] },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
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
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("写文件", r#"{"path":"a.txt","content":"你好"}"#)] },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
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
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("运行命令", r#"{"command":"cargo build"}"#)] },
            模型响应 { 思考: None, 内容: Some("构建完成".into()), 工具调用: vec![] },
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
            模型响应 { 思考: None,
                内容: None,
                工具调用: vec![
                    工具调用("读文件", r#"{"路径":"a.txt"}"#),
                    工具调用("运行命令", r#"{"命令":"cargo build"}"#),
                ],
            },
            模型响应 { 思考: None, 内容: Some("构建完成".into()), 工具调用: vec![] },
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
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 2);

        let 结果 = 智能体.运行("永不完成".into());
        assert!(结果.is_err());
        assert!(结果.expect_err("应返回错误").to_string().contains("超过最大轮数"));
    }

    #[test]
    fn 智能体_未知工具_回填错误并继续循环() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("不存在的工具", "{}")] },
            模型响应 { 思考: None, 内容: Some("错误已处理".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        let 答复 = 智能体.运行("未知工具".into()).expect("工具错误不应终止循环");
        assert_eq!(答复, "错误已处理");
    }

    #[test]
    fn 智能体_参数缺失_回填错误并继续循环() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("写文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 思考: None, 内容: Some("参数已修正".into()), 工具调用: vec![] },
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
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
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
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 思考: None, 内容: Some("任务完成".into()), 工具调用: vec![] },
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
        // 每轮事件序：思考(正在思考)→思考开始→思考结束→(工具轮: 工具调用+工具结果 | 答复轮: 任务答复)
        assert_eq!(事件.len(), 9, "应有 轮0(思考x2+思考结束+调用+结果) + 轮1(思考x2+思考结束+答复) 九条事件");

        assert_eq!(事件[0].类型, 开发事件类型::思考);
        assert_eq!(事件[0].轮次, 0);
        assert!(事件[0].工具名.is_empty());

        assert_eq!(事件[1].类型, 开发事件类型::思考);
        assert_eq!(事件[1].内容, "__思考开始__");

        assert_eq!(事件[2].类型, 开发事件类型::思考);
        assert_eq!(事件[2].内容, "__思考结束__");

        assert_eq!(事件[3].类型, 开发事件类型::工具调用);
        assert_eq!(事件[3].工具名, "读文件");
        assert!(事件[3].内容.contains("a.txt"));

        assert_eq!(事件[4].类型, 开发事件类型::工具结果);
        assert_eq!(事件[4].工具名, "读文件");
        assert!(事件[4].内容.contains("文件内容"));

        assert_eq!(事件[5].类型, 开发事件类型::思考);
        assert_eq!(事件[5].轮次, 1);

        assert_eq!(事件[6].类型, 开发事件类型::思考);
        assert_eq!(事件[6].内容, "__思考开始__");

        assert_eq!(事件[7].类型, 开发事件类型::思考);
        assert_eq!(事件[7].内容, "__思考结束__");

        assert_eq!(事件[8].类型, 开发事件类型::任务答复);
        assert_eq!(事件[8].轮次, 1);
        assert_eq!(事件[8].内容, "任务完成");

        assert!(事件.iter().all(|e| !e.内容.is_empty()), "全部事件内容应非空");
    }

    #[test]
    fn 智能体_任务清单_覆盖式更新状态() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None,
                内容: None,
                工具调用: vec![工具调用(
                    "任务清单",
                    r#"{"清单":[{"内容":"设计接口","状态":"进行中"},{"内容":"写测试","状态":"待办"}]}"#,
                )],
            },
            模型响应 { 思考: None,
                内容: None,
                工具调用: vec![工具调用("任务清单", r#"{"清单":[{"内容":"全部完成","状态":"已完成"}]}"#)],
            },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        智能体.运行("更新任务清单".into()).expect("运行应成功");
        let 清单 = 智能体.当前任务清单();
        assert!(清单.contains("全部完成"), "第二次覆盖应生效，实际: {清单}");
        assert!(清单.contains("已完成"), "状态应格式化为已完成，实际: {清单}");
        assert!(!清单.contains("设计接口"), "覆盖式更新应丢弃旧条目，实际: {清单}");
    }

    #[test]
    fn 智能体_任务清单_英文键todos与状态兼容() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None,
                内容: None,
                工具调用: vec![工具调用("任务清单", r#"{"todos":[{"内容":"一项","状态":"completed"}]}"#)],
            },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        智能体.运行("更新任务清单".into()).expect("运行应成功");
        let 清单 = 智能体.当前任务清单();
        assert!(清单.contains("一项"), "英文键 todos 应识别，实际: {清单}");
        assert!(清单.contains("已完成"), "英文状态 completed 应映射为已完成，实际: {清单}");
    }

    #[test]
    fn 智能体_任务清单_内容字段英文别名兼容() {
        // 回归：LLM 偶在清单项内用英文 `content` 而非规范键 `内容`，解析层应容错（与状态字段别名同策略）
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None,
                内容: None,
                工具调用: vec![工具调用("任务清单", r#"{"清单":[{"content":"内容用英文键","状态":"待办"}]}"#)],
            },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        智能体.运行("更新任务清单".into()).expect("运行应成功");
        let 清单 = 智能体.当前任务清单();
        assert!(清单.contains("内容用英文键"), "内容字段英文别名 content 应识别，实际: {清单}");
    }

    #[test]
    fn 智能体_任务清单_状态字段英文键别名兼容() {
        // 回归：LLM 偶用英文键 `status`（值仍为中文「已完成」）而非规范键 `状态`，状态应正确解析为已完成而非回退待办
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None,
                内容: None,
                工具调用: vec![工具调用("任务清单", r#"{"清单":[{"内容":"完成项","status":"已完成"}]}"#)],
            },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        智能体.运行("更新任务清单".into()).expect("运行应成功");
        let 清单 = 智能体.当前任务清单();
        assert!(清单.contains("[已完成] 完成项"), "状态键英文别名 status 应识别为已完成，实际: {清单}");
    }

    #[test]
    fn 智能体_任务清单_缺失内容回填错误并继续() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None,
                内容: None,
                工具调用: vec![工具调用("任务清单", r#"{"清单":[{"状态":"待办"}]}"#)],
            },
            模型响应 { 思考: None, 内容: Some("已修正".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        let 答复 = 智能体.运行("缺内容".into()).expect("缺失内容不应终止循环");
        assert_eq!(答复, "已修正");
        assert_eq!(智能体.当前任务清单(), "（任务清单为空）");
    }

    #[test]
    fn 智能体_列目录_空参数缺省列根() {
        // 回归：LLM 常以空参数 `{}` 调用列目录表示「列出工作区根」，路径应缺省为根（点号）
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("列目录", r#"{}"#)] },
            模型响应 { 思考: None, 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 智能体 = 智能体::new(对话器, 执行器.clone(), 10);

        智能体.运行("列根".into()).expect("空参数列目录不应失败");
        let 记录 = 执行器.列目录记录.lock().expect("列目录记录锁中毒");
        assert_eq!(记录.as_slice(), &[".".to_string()], "空参数应缺省为列工作区根（.），实际: {记录:?}");
    }

    #[test]
    fn 智能体_参数缺失错误回喂含可接受键与实收键() {
        // 回归：只报「缺少参数: 命令」时模型无从得知该改什么，会反复以同一份空参数重试直到熔断
        // （2026-09-14 实证：#51 准圣连续 3 次以 `{}` 调用「运行命令」）。错误文案必须给出可接受键与实收键。
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("运行命令", r#"{}"#)] },
            模型响应 { 思考: None, 内容: Some("已修正".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 事件 = Arc::new(Mutex::new(Vec::<开发事件>::new()));
        let 事件_克隆 = 事件.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10)
            .设置事件回调(Arc::new(move |e| 事件_克隆.lock().expect("事件锁中毒").push(e.clone())));

        let 答复 = 智能体.运行("执行命令".into()).expect("参数缺失不应终止循环");
        assert_eq!(答复, "已修正");
        assert!(执行器_记录.命令记录.lock().expect("锁").is_empty(), "参数缺失时不得把空命令下发给执行器");
        let 回喂: Vec<String> = 事件
            .lock()
            .expect("事件锁中毒")
            .iter()
            .filter(|e| matches!(e.类型, 开发事件类型::工具结果))
            .map(|e| e.内容.clone())
            .collect();
        assert!(
            回喂
                .iter()
                .any(|文| 文.contains("可接受键") && 文.contains("命令 / command") && 文.contains("无")),
            "错误回喂须含可接受键与实收键，否则模型会以空参数重试至熔断：{回喂:?}"
        );
    }
}
