#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};
    use hm_agent::{智能体, 五层协作驱动器, 驱动结果};
    use hm_contract::Component;
    use hm_content_contract::{工具对话器, 对话消息, 工具调用, 模型响应};
    use hm_error::{Error, Result};
    use hm_execute_contract::{开发事件, 开发事件类型, 执行器};
    use hm_cognition::ContextManager;
    use tc_task::{Task, TaskBoard, TaskStatus, AgentRole};

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

        fn 列目录(&self, _路径: &str) -> Result<String> {
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

    #[test]
    fn 智能体_任务清单_覆盖式更新状态() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 {
                内容: None,
                工具调用: vec![工具调用(
                    "任务清单",
                    r#"{"清单":[{"内容":"设计接口","状态":"进行中"},{"内容":"写测试","状态":"待办"}]}"#,
                )],
            },
            模型响应 {
                内容: None,
                工具调用: vec![工具调用("任务清单", r#"{"清单":[{"内容":"全部完成","状态":"已完成"}]}"#)],
            },
            模型响应 { 内容: Some("完成".into()), 工具调用: vec![] },
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
            模型响应 {
                内容: None,
                工具调用: vec![工具调用("任务清单", r#"{"todos":[{"内容":"一项","状态":"completed"}]}"#)],
            },
            模型响应 { 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        智能体.运行("更新任务清单".into()).expect("运行应成功");
        let 清单 = 智能体.当前任务清单();
        assert!(清单.contains("一项"), "英文键 todos 应识别，实际: {清单}");
        assert!(清单.contains("已完成"), "英文状态 completed 应映射为已完成，实际: {清单}");
    }

    #[test]
    fn 智能体_任务清单_缺失内容回填错误并继续() {
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 {
                内容: None,
                工具调用: vec![工具调用("任务清单", r#"{"清单":[{"状态":"待办"}]}"#)],
            },
            模型响应 { 内容: Some("已修正".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        let 答复 = 智能体.运行("缺内容".into()).expect("缺失内容不应终止循环");
        assert_eq!(答复, "已修正");
        assert_eq!(智能体.当前任务清单(), "（任务清单为空）");
    }

    // ==================== 五层协作驱动器 ====================

    fn 临时路径(名: &str) -> String {
        let 目录 = std::env::temp_dir().join("zd-agent-驱动器");
        let _ = std::fs::create_dir_all(&目录);
        目录.join(名).to_string_lossy().to_string()
    }

    fn 造任务(标题: &str) -> Task {
        Task::新建(0, 标题.to_string(), "需求描述".to_string(), 0)
    }

    fn 新驱动器(看板: TaskBoard, 序列: Vec<模型响应>, 上下文路径: &str) -> (五层协作驱动器, Arc<Mutex<TaskBoard>>) {
        let 看板 = Arc::new(Mutex::new(看板));
        let 上下文 = Arc::new(Mutex::new(ContextManager::新(上下文路径)));
        let 对话器 = Arc::new(模拟对话器::新(序列));
        let 执行器 = Arc::new(模拟执行器::新());
        let 驱动器 = 五层协作驱动器::新(看板.clone(), 上下文, 对话器, 执行器, 10);
        (驱动器, 看板)
    }

    fn 设计样例() -> &'static str {
        r#"{"边界定义":{"模块":"a"},"安全区域":[],"契约":[{"契约名":"测试契约","方法":[{"名称":"方法一","签名":"fn 方法一()","描述":"做某事"}],"描述":"契约描述"}],"修改文件":[],"新建文件":[],"依赖":[]}"#
    }

    fn 实现样例() -> &'static str {
        r#"{"代码变更":[{"文件路径":"src/x.rs","变更类型":"修改","摘要":"实现功能"}],"自检":{"通过":true,"边界合规":true,"契约合规":true,"问题":[]}}"#
    }

    fn 验收样例(结果: bool) -> String {
        format!(
            r#"{{"轮次":[{{"轮次":1,"通过":{0},"边界检查":true,"契约检查":true,"安全检查":true,"事实检查":true,"完整性检查":true,"问题":[],"建议":"无"}}],"最终结果":{0}}}"#,
            结果
        )
    }

    fn 终审样例(通过: bool) -> String {
        format!(
            r#"{{"通过":{0},"需求满足度":9,"可维护性":8,"代码质量":8,"风险评估":"低","评语":"通过"}}"#,
            通过
        )
    }

    #[test]
    fn 驱动器_圣人设计阶段产出设计文档() {
        let mut 看板 = TaskBoard::新建(临时路径("圣人"));
        let mut task = 造任务("设计任务");
        task.status = TaskStatus::待圣人设计;
        看板.发布任务(task).expect("发布应成功");
        let (驱动器, 看板) = 新驱动器(
            看板,
            vec![模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] }],
            &临时路径("ctx-圣人"),
        );

        let 结果 = 驱动器.执行一轮().expect("驱动应成功");
        assert_eq!(
            结果,
            驱动结果::阶段完成 { 任务id: 1, 角色: AgentRole::圣人, 新状态: TaskStatus::待大罗金仙实现 }
        );

        let 看板 = 看板.lock().expect("看板锁");
        let 任务 = 看板.查询(1).expect("任务应存在");
        let 设计 = 任务.设计文档.as_ref().expect("设计文档应写入");
        assert_eq!(设计.契约[0].契约名, "测试契约");
        assert_eq!(设计.契约[0].方法[0].名称, "方法一");
    }

    #[test]
    fn 驱动器_大罗金仙实现阶段产出实现文档() {
        let mut 看板 = TaskBoard::新建(临时路径("大罗金仙"));
        let mut task = 造任务("实现任务");
        task.status = TaskStatus::待大罗金仙实现;
        看板.发布任务(task).expect("发布应成功");
        let (驱动器, 看板) = 新驱动器(
            看板,
            vec![模型响应 { 内容: Some(实现样例().into()), 工具调用: vec![] }],
            &临时路径("ctx-大罗金仙"),
        );

        let 结果 = 驱动器.执行一轮().expect("驱动应成功");
        assert_eq!(
            结果,
            驱动结果::阶段完成 { 任务id: 1, 角色: AgentRole::大罗金仙, 新状态: TaskStatus::待准圣验收 }
        );

        let 看板 = 看板.lock().expect("看板锁");
        let 任务 = 看板.查询(1).expect("任务应存在");
        let 实现 = 任务.实现文档.as_ref().expect("实现文档应写入");
        assert!(实现.自检.通过);
        assert_eq!(实现.代码变更[0].文件路径, "src/x.rs");
    }

    #[test]
    fn 驱动器_准圣验收不通过流转到待修复() {
        let mut 看板 = TaskBoard::新建(临时路径("准圣"));
        let mut task = 造任务("验收任务");
        task.status = TaskStatus::待准圣验收;
        看板.发布任务(task).expect("发布应成功");
        let (驱动器, 看板) = 新驱动器(
            看板,
            vec![模型响应 { 内容: Some(验收样例(false).into()), 工具调用: vec![] }],
            &临时路径("ctx-准圣"),
        );

        let 结果 = 驱动器.执行一轮().expect("驱动应成功");
        assert_eq!(结果, 驱动结果::阶段完成 { 任务id: 1, 角色: AgentRole::准圣, 新状态: TaskStatus::待修复 });

        let 看板 = 看板.lock().expect("看板锁");
        let 任务 = 看板.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待修复);
        let 验收 = 任务.验收文档.as_ref().expect("验收文档应写入");
        assert!(!验收.最终结果);
    }

    #[test]
    fn 驱动器_道祖终审通过进入待清理() {
        let mut 看板 = TaskBoard::新建(临时路径("道祖"));
        let mut task = 造任务("终审任务");
        task.status = TaskStatus::待道祖终审;
        看板.发布任务(task).expect("发布应成功");
        let (驱动器, 看板) = 新驱动器(
            看板,
            vec![模型响应 { 内容: Some(终审样例(true).into()), 工具调用: vec![] }],
            &临时路径("ctx-道祖"),
        );

        let 结果 = 驱动器.执行一轮().expect("驱动应成功");
        assert_eq!(结果, 驱动结果::阶段完成 { 任务id: 1, 角色: AgentRole::道祖, 新状态: TaskStatus::待清理 });

        let 看板 = 看板.lock().expect("看板锁");
        let 任务 = 看板.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待清理);
        assert!(任务.终审文档.as_ref().expect("终审文档应写入").通过);
    }

    #[test]
    fn 驱动器_完整链路六阶段到清理完成() {
        let mut 看板 = TaskBoard::新建(临时路径("完整链路"));
        let mut task = 造任务("完整任务");
        task.status = TaskStatus::待圣人设计;
        看板.发布任务(task).expect("发布应成功");
        let 序列 = vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
            模型响应 { 内容: Some(实现样例().into()), 工具调用: vec![] },
            模型响应 { 内容: Some(验收样例(true).into()), 工具调用: vec![] },
            模型响应 { 内容: Some(终审样例(true).into()), 工具调用: vec![] },
            模型响应 { 内容: Some(r#"{"清理项":[{"项":"临时文件","结果":"已清理"}],"归档完成":true}"#.into()), 工具调用: vec![] },
        ];
        let (驱动器, 看板) = 新驱动器(看板, 序列, &临时路径("ctx-完整"));

        let 结果 = 驱动器.执行到空闲(8).expect("驱动应成功");
        assert_eq!(结果.len(), 5, "应完成五个阶段（设计/实现/验收/终审/清理）");

        let 看板 = 看板.lock().expect("看板锁");
        let 任务 = 看板.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::清理完成);
        assert!(任务.设计文档.is_some(), "设计文档应写入");
        assert!(任务.实现文档.is_some(), "实现文档应写入");
        assert!(任务.验收文档.is_some(), "验收文档应写入");
        assert!(任务.终审文档.is_some(), "终审文档应写入");
        assert_eq!(任务.承接历史.len(), 5, "五角色各承接一次");
    }

    #[test]
    fn 驱动器_无任务返回空闲() {
        let 看板 = TaskBoard::新建(临时路径("空闲"));
        let (驱动器, _) = 新驱动器(看板, vec![], &临时路径("ctx-空闲"));
        assert_eq!(驱动器.执行一轮().expect("空看板应返回空闲"), 驱动结果::空闲);
    }

    #[test]
    fn 驱动器_非法产出返回错误且状态不变() {
        let mut 看板 = TaskBoard::新建(临时路径("坏产出"));
        let mut task = 造任务("坏产出任务");
        task.status = TaskStatus::待圣人设计;
        看板.发布任务(task).expect("发布应成功");
        let (驱动器, 看板) = 新驱动器(
            看板,
            vec![模型响应 { 内容: Some("抱歉我无法输出 JSON".into()), 工具调用: vec![] }],
            &临时路径("ctx-坏产出"),
        );

        let 结果 = 驱动器.执行一轮();
        assert!(结果.is_err(), "非 JSON 产出应返回错误");

        let 看板 = 看板.lock().expect("看板锁");
        let 任务 = 看板.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待圣人设计, "任务状态应保持不变（未承接未提交）");
        assert!(任务.设计文档.is_none(), "文档不应写入");
        assert!(任务.承接历史.is_empty(), "不应产生承接记录");
    }

}
