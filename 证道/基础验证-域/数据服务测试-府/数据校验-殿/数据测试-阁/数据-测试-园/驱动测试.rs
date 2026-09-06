#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use axum::{Json, extract::{Query, State}, http::StatusCode};
    use hm_http::{
        数据服务状态, 看板驱动台, 看板驱动接口, 看板驱动到空闲接口, 驱动到空闲请求,
        看板驱动状态接口, 看板驱动事件接口, 驱动事件响应, 事件游标,
        受理错误响应, 发布任务请求, 看板发布, 受理开发任务, 受理失败,
    };
    use hm_agent::{五层协作驱动器, 认知注入};
    use hm_cognition::{ContextManager, 上下文库, 三态存储, 图谱, 心智地图, 过程上下文, 消息角色};
    use hm_contract::Component;
    use hm_content_contract::{工具对话器, 对话消息, 工具调用, 模型响应};
    use hm_domain_contract::{任务仓库契约, 迭代日志契约, 记忆库契约, 规则库契约, 事件总线契约};
    use hm_error::{Error, Result};
    use hm_execute_contract::执行器;
    use hm_log::运行日志记录器;
    use tc_task::{Task, TaskStatus, TaskStore, TaskBoard, AgentRole};
    use lj_iteration::{Iteration, Version, IterationLog};
    use qk_memory::{Memory, MemoryStore};
    use dy_rule::{Rule, RuleSet};
    use hd_event::{Event, EventBus};

    static 看板序号: AtomicU64 = AtomicU64::new(0);

    /// 模拟对话器：按预设序列返回模型响应；可带延迟模拟慢响应（并发测试用）
    struct 模拟对话器 {
        响应序列: Mutex<VecDeque<模型响应>>,
        延迟毫秒: u64,
    }

    impl 模拟对话器 {
        fn 新(序列: Vec<模型响应>) -> Self {
            模拟对话器 { 响应序列: Mutex::new(序列.into()), 延迟毫秒: 0 }
        }

        fn 新_带延迟(序列: Vec<模型响应>, 延迟毫秒: u64) -> Self {
            模拟对话器 { 响应序列: Mutex::new(序列.into()), 延迟毫秒 }
        }
    }

    impl Component for 模拟对话器 {
        fn name(&self) -> &'static str { "模拟对话器" }
    }

    impl 工具对话器 for 模拟对话器 {
        fn 对话(&self, _消息: Vec<对话消息>, _工具: Vec<serde_json::Value>) -> Result<模型响应> {
            if self.延迟毫秒 > 0 {
                std::thread::sleep(Duration::from_millis(self.延迟毫秒));
            }
            let mut 序列 = self.响应序列.lock().expect("模拟对话器 锁中毒");
            序列.pop_front().ok_or_else(|| Error::Other("对话序列已耗尽".into()))
        }
    }

    /// 模拟执行器：固定返回预设内容
    struct 模拟执行器;

    impl Component for 模拟执行器 {
        fn name(&self) -> &'static str { "模拟执行器" }
    }

    impl 执行器 for 模拟执行器 {
        fn 读文件(&self, _路径: &str) -> Result<String> { Ok("文件内容".to_string()) }
        fn 写文件(&self, _路径: &str, _内容: &str) -> Result<()> { Ok(()) }
        fn 运行命令(&self, _命令: &str) -> Result<String> { Ok("命令输出".to_string()) }
        fn 列目录(&self, _路径: &str) -> Result<String> { Ok("（空目录）".to_string()) }
        fn 按名找文件(&self, _模式: &str) -> Result<String> { Ok("（无匹配）".to_string()) }
        fn 搜索内容(&self, _关键词: &str) -> Result<String> { Ok("（无匹配）".to_string()) }
        fn 精确编辑(&self, _路径: &str, _旧: &str, _新: &str) -> Result<String> { Ok("替换成功（1 处）".to_string()) }
    }

    /// 构造 数据服务状态 + 共享任务看板引用（未装配驱动台）
    fn 驱动状态() -> (数据服务状态, Arc<Mutex<TaskBoard>>) {
        let 任务仓库: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>> = Arc::new(Mutex::new(TaskStore::new()));
        let 迭代日志: Arc<Mutex<dyn 迭代日志契约<Iteration, Version>>> = Arc::new(Mutex::new(IterationLog::new()));
        let 记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>> = Arc::new(Mutex::new(MemoryStore::new()));
        let 规则库: Arc<Mutex<dyn 规则库契约<Rule>>> = Arc::new(Mutex::new(RuleSet::new()));
        let 事件总线: Arc<Mutex<dyn 事件总线契约<Event>>> = Arc::new(Mutex::new(EventBus::new()));
        let 图谱 = Arc::new(Mutex::new(图谱::新()));
        let 心智地图 = Arc::new(Mutex::new(心智地图::新()));
        let 语境 = Arc::new(Mutex::new(过程上下文::新()));
        let 日志记录器 = Arc::new(Mutex::new(运行日志记录器::new()));
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 任务看板 = Arc::new(Mutex::new(TaskBoard::新建(
            std::env::temp_dir().join(format!("洪荒驱动测试看板_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 开发执行台 = Arc::new(hm_http::开发执行台::新());
        let 看板驱动台 = Arc::new(看板驱动台::新());
        let 状态 = 数据服务状态::新(任务仓库, 迭代日志, 记忆库, 规则库, 事件总线, 图谱, 心智地图, 语境, 任务看板.clone(), 日志记录器, 开发执行台, 看板驱动台, None);
        (状态, 任务看板)
    }

    /// 装配驱动台到状态（注入 mock 对话器/执行器 + 独立 ContextManager）
    fn 装配驱动器(状态: &数据服务状态, 看板: &Arc<Mutex<TaskBoard>>, 对话器: Arc<模拟对话器>) {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 上下文 = Arc::new(Mutex::new(ContextManager::新(
            std::env::temp_dir().join(format!("洪荒驱动测试上下文_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 驱动器 = Arc::new(五层协作驱动器::新(
            看板.clone(), 上下文, 对话器, Arc::new(模拟执行器), 10,
        ));
        状态.看板驱动台.装配(驱动器);
    }

    /// 带三态认知注入装配驱动器（返回 上下文库 句柄供断言临时态记录）
    fn 装配驱动器带认知(状态: &数据服务状态, 看板: &Arc<Mutex<TaskBoard>>, 对话器: Arc<模拟对话器>) -> Arc<Mutex<上下文库>> {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 上下文 = Arc::new(Mutex::new(ContextManager::新(
            std::env::temp_dir().join(format!("洪荒驱动测试上下文_认知_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 库 = Arc::new(Mutex::new(上下文库::新_带上限(1000)));
        let 注入 = 认知注入::新(状态.图谱.clone(), 状态.心智地图.clone(), 库.clone());
        let 驱动器 = Arc::new(
            五层协作驱动器::新(看板.clone(), 上下文, 对话器, Arc::new(模拟执行器), 10)
                .装配认知(注入),
        );
        状态.看板驱动台.装配(驱动器);
        库
    }

    /// 带三态认知注入 + 持久化存储装配驱动器（返回 存储 句柄供断言落盘）
    fn 装配驱动器带存储(状态: &数据服务状态, 看板: &Arc<Mutex<TaskBoard>>, 对话器: Arc<模拟对话器>) -> Arc<三态存储> {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 上下文 = Arc::new(Mutex::new(ContextManager::新(
            std::env::temp_dir().join(format!("洪荒驱动测试上下文_存储_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 存储 = Arc::new(三态存储::新(
            std::env::temp_dir().join(format!("洪荒驱动测试三态_{序号}")),
        ).expect("创建三态存储"));
        let 注入 = 认知注入::新(状态.图谱.clone(), 状态.心智地图.clone(), Arc::new(Mutex::new(上下文库::新_带上限(1000))))
            .装配存储(存储.clone());
        let 驱动器 = Arc::new(
            五层协作驱动器::新(看板.clone(), 上下文, 对话器, Arc::new(模拟执行器), 10)
                .装配认知(注入),
        );
        状态.看板驱动台.装配(驱动器);
        存储
    }

    /// 通过 HTTP handler 发布任务（与真实链路一致：状态=待圣人设计、发起人=道祖）
    async fn 发布任务(状态: &数据服务状态, 标题: &str) {
        let 请求 = Json(发布任务请求 {
            title: 标题.into(),
            description: "驱动测试描述".into(),
            scene: None,
            priority: None,
        });
        let Json(_) = 看板发布(State(状态.clone()), 请求).await.expect("发布应成功");
    }

    fn 设计样例() -> &'static str {
        r#"{"边界定义":{"模块":"a"},"安全区域":[],"契约":[{"契约名":"测试契约","方法":[{"名称":"方法一","签名":"fn 方法一()","描述":"做某事"}],"描述":"契约描述"}],"修改文件":[],"新建文件":[],"依赖":[]}"#
    }

    fn 实现样例() -> &'static str {
        r#"{"代码变更":[{"文件路径":"src/x.rs","变更类型":"修改","摘要":"实现功能"}],"自检":{"通过":true,"边界合规":true,"契约合规":true,"问题":[]}}"#
    }

    fn 验收样例() -> String {
        r#"{"轮次":[{"轮次":1,"通过":true,"边界检查":true,"契约检查":true,"安全检查":true,"事实检查":true,"完整性检查":true,"问题":[],"建议":"无"}],"最终结果":true}"#.into()
    }

    fn 终审样例() -> String {
        r#"{"通过":true,"需求满足度":9,"可维护性":8,"代码质量":8,"风险评估":"低","评语":"通过"}"#.into()
    }

    /// 完整五层链的对话器序列：设计→实现→验收→终审
    fn 完整链路响应() -> Vec<模型响应> {
        vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
            模型响应 { 内容: Some(实现样例().into()), 工具调用: vec![] },
            模型响应 { 内容: Some(验收样例()), 工具调用: vec![] },
            模型响应 { 内容: Some(终审样例()), 工具调用: vec![] },
        ]
    }

    /// 未装配驱动台时 POST /api/dev/pilot 应 503（fail-loud）
    #[tokio::test]
    async fn 驱动台_未装配驱动返回503() {
        let (状态, _) = 驱动状态();
        let 结果 = 看板驱动接口(State(状态)).await;
        match 结果 {
            Err((码, Json(受理错误响应 { 错误 }))) => {
                assert_eq!(码, StatusCode::SERVICE_UNAVAILABLE);
                assert!(错误.contains("未就绪"), "错误消息应说明未就绪: {错误}");
            }
            Ok(_) => panic!("未装配时应返回错误"),
        }
    }

    /// 装配后驱动一轮：待圣人设计 → 待大罗金仙实现，设计文档写入，状态接口可查
    #[tokio::test]
    async fn 驱动台_装配后驱动一轮任务流转() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ]));
        装配驱动器(&状态, &看板, 对话器);
        发布任务(&状态, "驱动流转任务").await; // 发布即驱动

        assert!(状态.看板驱动台.等待完成(5000), "发布后自动驱动应在超时内完成");

        // 任务已流转到 待大罗金仙实现 且设计文档写入
        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待大罗金仙实现);
        let 设计 = 任务.设计文档.as_ref().expect("设计文档应写入");
        assert_eq!(设计.契约[0].契约名, "测试契约");
        assert_eq!(设计.契约[0].方法[0].名称, "方法一");
        drop(看板守卫);

        // 状态接口返回 阶段完成 摘要
        let 摘要 = 状态.看板驱动台.当前状态();
        assert!(摘要.就绪);
        assert!(!摘要.运行中);
        let 阶段 = 摘要.最近阶段.expect("应有最近阶段");
        assert_eq!(阶段.类型, "阶段完成");
        assert_eq!(阶段.任务id, Some(1));
        assert_eq!(阶段.角色.as_deref(), Some("圣人"));
        assert_eq!(阶段.新状态.as_deref(), Some("待大罗金仙实现"));
        let 结果 = 摘要.最近结果.expect("应有最近结果");
        assert!(结果.contains("已推进"), "最近结果应说明推进: {结果}");
    }

    /// 带三态认知注入装配驱动一轮：流转正常 + 临时态上下文库记录 阶段提示 与 答复
    #[tokio::test]
    async fn 驱动台_带认知装配驱动一轮记录临时态() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ]));
        let 库 = 装配驱动器带认知(&状态, &看板, 对话器);
        发布任务(&状态, "认知装配流转任务").await;

        assert!(状态.看板驱动台.等待完成(5000), "发布后自动驱动应在超时内完成");

        // 任务正常流转（认知装配不改变流转语义）
        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待大罗金仙实现);
        let 设计 = 任务.设计文档.as_ref().expect("设计文档应写入");
        assert_eq!(设计.契约[0].契约名, "测试契约");
        drop(看板守卫);

        // 临时态上下文库记录本轮过程：阶段提示（用户）+ 助手答复
        let 库 = 库.lock().expect("上下文锁");
        assert!(库.长度() >= 2, "临时态应至少记录提示与答复，实际 {}", 库.长度());
        let 最近 = 库.最近(20);
        assert!(
            最近.iter().any(|m| m.角色 == 消息角色::用户 && m.内容.contains("认知装配流转任务")),
            "临时态应记录含任务标题的阶段提示"
        );
        assert!(
            最近.iter().any(|m| m.角色 == 消息角色::助手),
            "临时态应记录助手答复"
        );
    }

    /// 带三态持久化存储装配驱动一轮：落盘 + 上下文库可加载
    #[tokio::test]
    async fn 驱动台_带存储装配驱动后持久化() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ]));
        let 存储 = 装配驱动器带存储(&状态, &看板, 对话器);
        发布任务(&状态, "持久化流转任务").await;

        assert!(状态.看板驱动台.等待完成(5000), "发布后自动驱动应在超时内完成");

        // 任务正常流转
        let 看板守卫 = 看板.lock().expect("看板锁");
        assert_eq!(看板守卫.查询(1).expect("任务应存在").status, TaskStatus::待大罗金仙实现);
        drop(看板守卫);

        // 三态文件已写穿落盘，上下文库可加载且含阶段提示
        assert!(存储.目录().join("心智地图.json").exists(), "心智地图应落盘");
        assert!(存储.目录().join("图谱.json").exists(), "图谱应落盘");
        assert!(存储.目录().join("上下文库.jsonl").exists(), "上下文库应落盘");
        let 库 = 存储.加载上下文().expect("加载上下文库");
        assert!(
            库.全部().iter().any(|m| m.角色 == 消息角色::用户 && m.内容.contains("持久化流转任务")),
            "临时态应含阶段提示"
        );
    }

    /// 空看板驱动一轮返回 空闲
    #[tokio::test]
    async fn 驱动台_空看板驱动返回空闲() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![])));

        状态.看板驱动台.启动执行一轮();
        assert!(状态.看板驱动台.等待完成(5000));

        let 阶段 = 状态.看板驱动台.当前状态().最近阶段.expect("应有最近阶段");
        assert_eq!(阶段.类型, "空闲");
        assert!(!状态.看板驱动台.运行中());
    }

    /// 已有驱动运行中时再次驱动应 409
    #[tokio::test]
    async fn 驱动台_运行中重复驱动返回409() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新_带延迟(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ], 800));
        装配驱动器(&状态, &看板, 对话器);
        发布任务(&状态, "并发驱动任务").await;

        状态.看板驱动台.启动执行一轮();
        // 第一轮仍在后台执行（mock 延迟 800ms），此刻再次驱动应冲突
        let 结果 = 看板驱动接口(State(状态.clone())).await;
        match 结果 {
            Err((码, _)) => assert_eq!(码, StatusCode::CONFLICT),
            Ok(_) => panic!("运行中应返回冲突"),
        }
        assert!(状态.看板驱动台.等待完成(5000), "首轮驱动应完成");
    }

    /// 非法产出（非 JSON）：最近阶段=错误，任务状态不变（仍待承接）
    #[tokio::test]
    async fn 驱动台_非法产出记录错误且任务状态不变() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![] },
        ]));
        装配驱动器(&状态, &看板, 对话器);
        发布任务(&状态, "非法产出任务").await;

        状态.看板驱动台.启动执行一轮();
        assert!(状态.看板驱动台.等待完成(5000));

        let 摘要 = 状态.看板驱动台.当前状态();
        let 阶段 = 摘要.最近阶段.expect("应有最近阶段");
        assert_eq!(阶段.类型, "错误");
        assert!(阶段.消息.as_ref().is_some_and(|m| !m.is_empty()), "错误消息应非空");
        let 结果 = 摘要.最近结果.expect("应有最近结果");
        assert!(结果.contains("驱动失败"), "最近结果应说明失败: {结果}");

        // 任务状态未变，仍可被再次驱动
        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待圣人设计);
        assert!(任务.设计文档.is_none(), "失败不应写入设计文档");
    }

    /// 受理开发任务 = 发布看板任务（待圣人设计，发起人道祖）+ 自动驱动一轮
    #[tokio::test]
    async fn 受理_发布看板并自动驱动流转() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ]));
        装配驱动器(&状态, &看板, 对话器);

        let id = 受理开发任务(&状态, "修复登录超时".into()).expect("受理应成功");
        assert!(id > 0);
        assert!(状态.看板驱动台.等待完成(5000), "受理后自动驱动应在超时内完成");

        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(id).expect("看板任务应存在");
        assert_eq!(任务.title, "修复登录超时");
        assert_eq!(任务.status, TaskStatus::待大罗金仙实现);
        assert_eq!(任务.发起人, AgentRole::道祖);
        let 设计 = 任务.设计文档.as_ref().expect("设计文档应写入");
        assert_eq!(设计.契约[0].契约名, "测试契约");
    }

    /// 受理时已有驱动运行中 → 409，且不发布新任务
    #[tokio::test]
    async fn 受理_驱动运行中重复受理返回运行中() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新_带延迟(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ], 800));
        装配驱动器(&状态, &看板, 对话器);

        状态.看板驱动台.启动执行一轮();
        let 重复 = 受理开发任务(&状态, "第二个任务".into());
        assert!(matches!(重复.expect_err("应拒绝重复受理"), 受理失败::运行中));

        let 看板守卫 = 看板.lock().expect("看板锁");
        assert_eq!(看板守卫.全部().len(), 0, "受理失败不应发布看板任务");
        drop(看板守卫);
        assert!(状态.看板驱动台.等待完成(5000), "首轮驱动应完成");
    }

    /// 前端发布任务后自动驱动（无需点按钮）
    #[tokio::test]
    async fn 看板发布_成功后自动驱动一轮() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ]));
        装配驱动器(&状态, &看板, 对话器);

        发布任务(&状态, "发布即驱动任务").await;
        assert!(状态.看板驱动台.等待完成(5000), "发布后应自动驱动完成");

        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待大罗金仙实现);
        drop(看板守卫);
        let 阶段 = 状态.看板驱动台.当前状态().最近阶段.expect("应有最近阶段");
        assert_eq!(阶段.类型, "阶段完成");
    }

    /// 驱动台未装配时发布任务：发布成功但停在待承接，不报错
    #[tokio::test]
    async fn 看板发布_驱动台未装配不驱动() {
        let (状态, 看板) = 驱动状态();

        发布任务(&状态, "无驱动任务").await;
        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待圣人设计, "未装配驱动台应停在待承接");
    }

    /// 驱动到空闲：空看板立即空闲
    #[tokio::test]
    async fn 驱动到空闲_空看板空闲() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![])));

        状态.看板驱动台.启动执行到空闲(10);
        assert!(状态.看板驱动台.等待完成(5000), "空看板应立即完成");

        let 摘要 = 状态.看板驱动台.当前状态();
        assert_eq!(摘要.最近阶段.expect("应有最近阶段").类型, "空闲");
        assert!(摘要.最近结果.expect("应有最近结果").contains("无可驱动任务"));
    }

    /// 驱动到空闲：单任务完整五层链一次跑完到已完成
    #[tokio::test]
    async fn 驱动到空闲_完整五层链自动流转到已完成() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(完整链路响应())));
        发布任务(&状态, "到空闲任务").await;
        // 发布即自动驱动：第 1 轮（设计）已由发布联动完成
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(10);
        assert!(状态.看板驱动台.等待完成(10000), "完整链路应在超时内完成");

        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::已完成, "五层链应推进到已完成");
        drop(看板守卫);

        let 摘要 = 状态.看板驱动台.当前状态();
        assert_eq!(摘要.最近结果.expect("应有汇总"), "共推进 3 轮", "drain 应推进 实现/验收/终审 三轮");
        let 阶段 = 摘要.最近阶段.expect("应有最近阶段");
        assert_eq!(阶段.类型, "阶段完成");
        assert_eq!(阶段.角色.as_deref(), Some("道祖"));
        assert_eq!(阶段.新状态.as_deref(), Some("已完成"));
    }

    /// 驱动到空闲：中途错误保留已推进轮次
    #[tokio::test]
    async fn 驱动到空闲_中途错误保留已推进轮次() {
        let (状态, 看板) = 驱动状态();
        // 发布自动驱动消耗设计；drain 内 实现成功(第1轮) → 验收非法产出(非 JSON) → 驱动错误
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
            模型响应 { 内容: Some(实现样例().into()), 工具调用: vec![] },
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![] },
        ])));
        发布任务(&状态, "中途出错任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(10);
        assert!(状态.看板驱动台.等待完成(10000), "应完成");

        let 摘要 = 状态.看板驱动台.当前状态();
        assert_eq!(摘要.最近阶段.expect("应有最近阶段").类型, "错误");
        let 结果 = 摘要.最近结果.expect("应有最近结果");
        assert!(结果.contains("已推进 1 轮"), "应保留已推进轮次: {结果}");
        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待准圣验收, "出错轮次的任务不应被推进");
    }

    /// 驱动到空闲：上限耗尽时任务停留可承接
    #[tokio::test]
    async fn 驱动到空闲_上限耗尽任务停留() {
        let (状态, 看板) = 驱动状态();
        // 发布自动驱动(设计) 后 drain(2)：实现+验收 → 停在待道祖终审
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(完整链路响应())));
        发布任务(&状态, "上限耗尽任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(2);
        assert!(状态.看板驱动台.等待完成(10000), "应完成");

        let 摘要 = 状态.看板驱动台.当前状态();
        assert_eq!(摘要.最近结果.expect("应有汇总"), "共推进 2 轮");
        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::待道祖终审, "drain 2 轮应停在待道祖终审");
    }

    /// HTTP：未装配 → 503
    #[tokio::test]
    async fn 驱动到空闲接口_未装配返回未上线() {
        let (状态, _看板) = 驱动状态();
        let 结果 = 看板驱动到空闲接口(
            State(状态.clone()),
            Json(驱动到空闲请求 { 上限: None }),
        )
        .await;
        assert!(结果.is_err(), "未装配应 503");
    }

    /// HTTP：运行中 → 409
    #[tokio::test]
    async fn 驱动到空闲接口_运行中返回运行中() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新_带延迟(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ], 800)));

        状态.看板驱动台.启动执行到空闲(10);
        let 重复 = 看板驱动到空闲接口(State(状态.clone()), Json(驱动到空闲请求 { 上限: None })).await;
        assert!(重复.is_err(), "运行中应 409");
        assert!(状态.看板驱动台.等待完成(5000), "首轮驱动应完成");
    }

    /// HTTP：正常驱动到空闲
    #[tokio::test]
    async fn 驱动到空闲接口_正常驱动() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(完整链路响应())));
        发布任务(&状态, "HTTP到空闲任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        let Json(响应) = 看板驱动到空闲接口(State(状态.clone()), Json(驱动到空闲请求 { 上限: Some(10) })).await.expect("应受理");
        assert_eq!(响应.受理, true);
        assert!(状态.看板驱动台.等待完成(10000), "应完成");

        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::已完成);
    }

    /// 驱动事件流：预留清空且序号递增
    #[tokio::test]
    async fn 驱动事件流_预留清空且序号递增() {
        let 台 = 看板驱动台::新();
        // 预占即清空上一次会话的事件流与序号
        assert!(台.预留());
        assert!(台.驱动事件增量(0).is_empty(), "预留后应清空旧事件");
        // 新事件序号从 1 递增
        台.记录驱动事件(&驱动阶段事件_测试("阶段完成", Some(1), Some("圣人".into()), Some("待大罗金仙实现".into())));
        台.记录驱动事件(&驱动阶段事件_测试("空闲", None, None, None));
        let 增量 = 台.驱动事件增量(0);
        assert_eq!(增量.len(), 2);
        assert_eq!(增量[0].序号, 1);
        assert_eq!(增量[0].类型, "阶段完成");
        assert_eq!(增量[1].序号, 2);
        assert_eq!(增量[1].类型, "空闲");
        台.释放();
    }

    /// 驱动事件流：多轮驱动记录完整事件（阶段完成×4 + 空闲）
    #[tokio::test]
    async fn 驱动事件流_多轮驱动记录完整事件() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(完整链路响应())));
        发布任务(&状态, "事件流任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(10);
        assert!(状态.看板驱动台.等待完成(10000), "drain 应完成");

        let 事件 = 状态.看板驱动台.驱动事件增量(0);
        assert_eq!(事件.len(), 5, "应记录 设计/实现/验收/终审 阶段完成 + 空闲");
        assert_eq!(事件[0].序号, 1);
        assert_eq!(事件[0].类型, "阶段完成");
        assert_eq!(事件[0].新状态.as_deref(), Some("待大罗金仙实现"));
        assert_eq!(事件[3].新状态.as_deref(), Some("已完成"));
        assert_eq!(事件[4].类型, "空闲");
    }

    /// 驱动事件流：错误记录错误事件，前面推进轮次各有阶段完成事件
    #[tokio::test]
    async fn 驱动事件流_错误记录错误事件() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
            模型响应 { 内容: Some(实现样例().into()), 工具调用: vec![] },
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![] },
        ])));
        发布任务(&状态, "错误事件流任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(10);
        assert!(状态.看板驱动台.等待完成(10000), "应完成");

        let 事件 = 状态.看板驱动台.驱动事件增量(0);
        assert_eq!(事件.len(), 3, "设计/实现 阶段完成 + 验收错误");
        assert_eq!(事件[0].类型, "阶段完成");
        assert_eq!(事件[1].类型, "阶段完成");
        assert_eq!(事件[2].类型, "错误");
        assert!(事件[2].消息.as_deref().unwrap_or("").contains("JSON"), "错误事件应含非法产出消息");
    }

    /// 驱动事件流：增量按游标过滤
    #[tokio::test]
    async fn 驱动事件流_增量按游标过滤() {
        let 台 = 看板驱动台::新();
        assert!(台.预留());
        for i in 1..=5 {
            台.记录驱动事件(&驱动阶段事件_测试("阶段完成", Some(i), Some("圣人".into()), Some("待大罗金仙实现".into())));
        }
        let 部分 = 台.驱动事件增量(3);
        assert_eq!(部分.len(), 2, "since=3 应剩序号 4、5");
        assert_eq!(部分[0].序号, 4);
        assert_eq!(部分[1].序号, 5);
        assert!(台.驱动事件增量(999).is_empty(), "超界 since 应空");
        台.释放();
    }

    /// HTTP：驱动事件接口 事件增量与状态
    #[tokio::test]
    async fn 驱动事件接口_事件增量与状态() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(完整链路响应())));
        发布任务(&状态, "HTTP事件流任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");
        状态.看板驱动台.启动执行到空闲(10);
        assert!(状态.看板驱动台.等待完成(10000), "drain 应完成");

        let Json(响应) = 看板驱动事件接口(State(状态.clone()), Query(事件游标 { since: Some(0) })).await;
        assert_eq!(响应.就绪, true);
        assert_eq!(响应.运行中, false);
        assert!(响应.最近结果.is_some(), "应有最近结果");
        assert_eq!(响应.事件.len(), 5, "全量事件 5 条");

        let Json(部分) = 看板驱动事件接口(State(状态.clone()), Query(事件游标 { since: Some(3) })).await;
        assert_eq!(部分.事件.len(), 2, "since=3 应剩 2 条");
        assert_eq!(部分.事件[0].序号, 4);
    }

    /// 构造驱动阶段事件（测试 helper）
    fn 驱动阶段事件_测试(类型: &str, 任务id: Option<u64>, 角色: Option<String>, 新状态: Option<String>) -> hm_http::驱动阶段事件 {
        hm_http::驱动阶段事件 {
            序号: 0,
            类型: 类型.into(),
            任务id,
            角色,
            新状态,
            消息: None,
            时间: 0,
        }
    }
}
