#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use axum::{Json, extract::{Query, State}, http::StatusCode};
    use hm_agent::{
        开发服务状态,
        看板驱动接口, 看板驱动到空闲接口, 驱动到空闲请求,
        看板驱动状态接口, 看板驱动事件接口, 驱动事件响应, 事件游标,
        受理错误响应, 发布任务请求, 看板发布,
        五层协作驱动器, 任务项, 任务状态,
        看板驱动台, 受理依赖, 受理开发任务, 受理失败,
        驱动阶段事件, 驱动会话存储, 运行检查点, 检查点消息上限,
    };
    use hm_cognition::{ContextManager, 上下文库, 三态存储, 图谱, 心智地图, 过程上下文, 消息角色, 认知注入};
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
        fn 运行命令(&self, _命令: &str) -> Result<String> {
            // 机器核验门要求 cargo test 输出含 `test result: ok.` 才判通过（防模型自报验收通过），
            // 故模拟命令输出须携带该证据行。
            Ok("test result: ok. 3 passed; 0 failed; 0 ignored".to_string())
        }
        fn 列目录(&self, _路径: &str) -> Result<String> { Ok("（空目录）".to_string()) }
        fn 按名找文件(&self, _模式: &str) -> Result<String> { Ok("（无匹配）".to_string()) }
        fn 搜索内容(&self, _关键词: &str) -> Result<String> { Ok("（无匹配）".to_string()) }
        fn 精确编辑(&self, _路径: &str, _旧: &str, _新: &str) -> Result<String> { Ok("替换成功（1 处）".to_string()) }
    }

    /// 构造开发服务状态（hm-agent）：任务看板 + 开发执行台 + 看板驱动台 + 记忆库
    fn 驱动状态() -> (开发服务状态<Memory>, Arc<Mutex<TaskBoard>>) {
        let 记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>> = Arc::new(Mutex::new(MemoryStore::new()));
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 任务看板 = Arc::new(Mutex::new(TaskBoard::新建(
            std::env::temp_dir().join(format!("洪荒驱动流转看板_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 状态 = 开发服务状态::新(
            任务看板.clone(),
            Arc::new(hm_agent::开发执行台::新()),
            Arc::new(看板驱动台::新()),
            记忆库,
        );
        (状态, 任务看板)
    }

    /// 装配驱动台到状态（注入 mock 对话器/执行器 + 独立 ContextManager）
    fn 装配驱动器(状态: &开发服务状态<Memory>, 看板: &Arc<Mutex<TaskBoard>>, 对话器: Arc<模拟对话器>) {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 上下文 = Arc::new(Mutex::new(ContextManager::新(
            std::env::temp_dir().join(format!("洪荒驱动流转上下文_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 驱动器 = Arc::new(五层协作驱动器::新(
            看板.clone(), 上下文, 对话器, Arc::new(模拟执行器), 10,
        ));
        状态.看板驱动台.装配(驱动器);
    }

    /// 通过 HTTP handler 发布任务（与真实链路一致：状态=待圣人设计、发起人=道祖）
    async fn 发布任务(状态: &开发服务状态<Memory>, 标题: &str) {
        let 请求 = Json(发布任务请求 {
            title: 标题.into(),
            description: "驱动测试描述".into(),
            scene: None,
            priority: None,
            临时规则: None,
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

    fn 审核样例() -> String {
        r#"{"通过":true,"驳回原因":null,"评语":"审核通过"}"#.into()
    }

    fn 清理样例() -> &'static str {
        r#"{"清理项":[{"项":"临时产物","结果":"已清理"}],"归档完成":true}"#
    }

    /// 完整七阶段链的对话器序列：设计→实现→验收→终审→审核→清理
    fn 完整链路响应() -> Vec<模型响应> {
        vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some(实现样例().into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some(验收样例()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some(终审样例()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some(审核样例()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some(清理样例().into()), 工具调用: vec![], 思考: None },
        ]
    }

    /// 构造驱动阶段事件（测试 helper）
    fn 驱动阶段事件_测试(类型: &str, 任务id: Option<u64>, 角色: Option<String>, 新状态: Option<String>) -> hm_agent::驱动阶段事件 {
        hm_agent::驱动阶段事件 {
            序号: 0,
            类型: 类型.into(),
            任务id,
            角色,
            新状态,
            层级: None,
            消息: None,
            时间: 0,
        }
    }

    /// 驱动到空闲：空看板立即空闲
    #[tokio::test]
    async fn 驱动到空闲_空看板空闲() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![])));

        状态.看板驱动台.启动执行到空闲(10, "手动到空闲");
        assert!(状态.看板驱动台.等待完成(5000), "空看板应立即完成");

        let 摘要 = 状态.看板驱动台.当前状态();
        assert_eq!(摘要.最近阶段.expect("应有最近阶段").类型, "空闲");
        assert!(摘要.最近结果.expect("应有最近结果").contains("无可驱动任务"));
    }

    /// 驱动到空闲：单任务完整六层链一次跑完到清理完成
    #[tokio::test]
    async fn 驱动到空闲_完整六层链自动流转到清理完成() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(完整链路响应())));
        发布任务(&状态, "到空闲任务").await;
        // 发布即自动驱动：第 1 轮（设计）已由发布联动完成
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(10, "手动到空闲");
        assert!(状态.看板驱动台.等待完成(10000), "完整链路应在超时内完成");

        let 看板守卫 = 看板.lock().expect("看板锁");
        let 任务 = 看板守卫.查询(1).expect("任务应存在");
        assert_eq!(任务.status, TaskStatus::清理完成, "六层链应推进到清理完成");
        drop(看板守卫);

        let 摘要 = 状态.看板驱动台.当前状态();
        assert_eq!(摘要.最近结果.expect("应有汇总"), "共推进 5 轮", "drain 应推进 实现/验收/终审/审核/清理 五轮");
        let 阶段 = 摘要.最近阶段.expect("应有最近阶段");
        assert_eq!(阶段.类型, "阶段完成");
        assert_eq!(阶段.角色.as_deref(), Some("太乙金仙"));
        assert_eq!(阶段.新状态.as_deref(), Some("清理完成"));
    }

    /// 驱动到空闲：中途错误保留已推进轮次
    #[tokio::test]
    async fn 驱动到空闲_中途错误保留已推进轮次() {
        let (状态, 看板) = 驱动状态();
        // 发布自动驱动消耗设计；drain 内 实现成功(第1轮) → 验收非法产出(非 JSON) → 重试 2 次仍失败 → 驱动错误
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some(实现样例().into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![], 思考: None },
        ])));
        发布任务(&状态, "中途出错任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(10, "手动到空闲");
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

        状态.看板驱动台.启动执行到空闲(2, "手动到空闲");
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
    ///
    /// 确定性：持住看板锁，使后台驱动阻塞在「选候选」短锁处，运行中标志必保持置位。
    /// （空看板时驱动会瞬间返回空闲，仅靠 mock 延迟无法保证运行中窗口 —— 原偶发失败根因）
    #[tokio::test]
    async fn 驱动到空闲接口_运行中返回运行中() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新_带延迟(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![], 思考: None },
        ], 800)));

        let 看板守卫 = 看板.lock().expect("看板锁");
        状态.看板驱动台.启动执行到空闲(10, "手动到空闲");
        let 重复 = 看板驱动到空闲接口(State(状态.clone()), Json(驱动到空闲请求 { 上限: None })).await;
        assert!(重复.is_err(), "运行中应 409");
        drop(看板守卫);
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
        assert_eq!(任务.status, TaskStatus::清理完成);
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

    /// 驱动事件流：跨轮预留后序号仍单调递增
    ///
    /// 缓冲每轮清空（新连接只看当前轮）是对的，但序号是客户端 since 游标的刻度，
    /// 一旦随缓冲一起倒拨，已连接的老客户端（游标停在上轮末号）就再也拉不到新事件。
    #[tokio::test]
    async fn 驱动事件流_跨轮预留序号单调不断裂() {
        let 台 = 看板驱动台::新();
        assert!(台.预留());
        台.记录驱动事件(&驱动阶段事件_测试("阶段完成", Some(1), Some("圣人".into()), Some("待大罗金仙实现".into())));
        let 首轮末号 = 台.驱动事件增量(0).last().map(|e| e.序号).expect("首轮应有事件");
        台.释放();

        // 第二轮：缓冲清空，但序号须接着上轮末号走
        assert!(台.预留());
        assert!(台.驱动事件增量(0).is_empty(), "预留后应清空旧事件缓冲");
        台.记录驱动事件(&驱动阶段事件_测试("阶段完成", Some(1), Some("大罗金仙".into()), Some("待准圣验收".into())));
        let 跨轮增量 = 台.驱动事件增量(首轮末号);
        assert_eq!(跨轮增量.len(), 1, "带首轮游标拉增量，不得因缓冲清空而断裂");
        assert!(跨轮增量[0].序号 > 首轮末号, "新事件序号须大于上轮游标");
        台.释放();
    }

    /// 驱动事件流：多轮驱动记录完整事件（阶段完成×5 + 空闲）
    #[tokio::test]
    async fn 驱动事件流_多轮驱动记录完整事件() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(完整链路响应())));
        发布任务(&状态, "事件流任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(10, "手动到空闲");
        assert!(状态.看板驱动台.等待完成(10000), "drain 应完成");

        let 事件 = 状态.看板驱动台.驱动事件增量(0);
        assert_eq!(事件.len(), 7, "应记录 设计/实现/验收/终审/审核/清理 阶段完成 + 空闲");
        assert_eq!(事件[0].序号, 1);
        assert_eq!(事件[0].类型, "阶段完成");
        assert_eq!(事件[0].新状态.as_deref(), Some("待大罗金仙实现"));
        assert_eq!(事件[3].新状态.as_deref(), Some("待人工验收"));
        assert_eq!(事件[4].新状态.as_deref(), Some("待清理"));
        assert_eq!(事件[5].新状态.as_deref(), Some("清理完成"));
        assert_eq!(事件[6].类型, "空闲");
    }

    /// 驱动事件流：错误记录错误事件，前面推进轮次各有阶段完成事件
    #[tokio::test]
    async fn 驱动事件流_错误记录错误事件() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some(实现样例().into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![], 思考: None },
            模型响应 { 内容: Some("这不是JSON".into()), 工具调用: vec![], 思考: None },
        ])));
        发布任务(&状态, "错误事件流任务").await;
        assert!(状态.看板驱动台.等待完成(10000), "发布自动驱动应完成");

        状态.看板驱动台.启动执行到空闲(10, "手动到空闲");
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
        状态.看板驱动台.启动执行到空闲(10, "手动到空闲");
        assert!(状态.看板驱动台.等待完成(10000), "drain 应完成");

        let Json(响应) = 看板驱动事件接口(State(状态.clone()), Query(事件游标 { since: Some(0) })).await;
        assert_eq!(响应.就绪, true);
        assert_eq!(响应.运行中, false);
        assert!(响应.最近结果.is_some(), "应有最近结果");
        assert_eq!(响应.事件.len(), 7, "全量事件 7 条");

        let Json(部分) = 看板驱动事件接口(State(状态.clone()), Query(事件游标 { since: Some(3) })).await;
        assert_eq!(部分.事件.len(), 4, "since=3 应剩 4 条");
        assert_eq!(部分.事件[0].序号, 4);
    }
}
