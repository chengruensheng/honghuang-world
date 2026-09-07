#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use axum::{Json, extract::{Path, State}, http::StatusCode};
    use hm_http::{
        数据服务状态, 看板驱动台, 会话恢复接口, 会话分叉接口, 会话清单接口,
        会话定向请求, 会话分叉请求, 分叉响应, 会话清单响应, 受理错误响应,
        发布任务请求, 看板发布,
    };
    use hm_agent::五层协作驱动器;
    use hm_cognition::{ContextManager, 图谱, 心智地图, 过程上下文};
    use hm_contract::Component;
    use hm_content_contract::{工具对话器, 对话消息, 工具调用, 模型响应};
    use hm_domain_contract::{任务仓库契约, 迭代日志契约, 记忆库契约, 规则库契约, 事件总线契约};
    use hm_error::{Error, Result};
    use hm_execute_contract::执行器;
    use hm_log::运行日志记录器;
    use tc_task::{Task, TaskStatus, TaskStore, TaskBoard};
    use lj_iteration::{Iteration, Version, IterationLog};
    use qk_memory::{Memory, MemoryStore};
    use dy_rule::{Rule, RuleSet};
    use hd_event::{Event, EventBus};

    static 会话序号: AtomicU64 = AtomicU64::new(0);

    /// 模拟对话器：按预设序列返回模型响应
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
        let 序号 = 会话序号.fetch_add(1, Ordering::SeqCst);
        let 任务看板 = Arc::new(Mutex::new(TaskBoard::新建(
            std::env::temp_dir().join(format!("洪荒会话恢复测试看板_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 开发执行台 = Arc::new(hm_http::开发执行台::新());
        let 看板驱动台 = Arc::new(看板驱动台::新());
        let 状态 = 数据服务状态::新(任务仓库, 迭代日志, 记忆库, 规则库, 事件总线, 图谱, 心智地图, 语境, 任务看板.clone(), 日志记录器, 开发执行台, 看板驱动台, None, None);
        (状态, 任务看板)
    }

    /// 装配驱动台到状态（注入 mock 对话器/执行器 + 独立 ContextManager）
    fn 装配驱动器(状态: &数据服务状态, 看板: &Arc<Mutex<TaskBoard>>, 对话器: Arc<模拟对话器>) {
        let 序号 = 会话序号.fetch_add(1, Ordering::SeqCst);
        let 上下文 = Arc::new(Mutex::new(ContextManager::新(
            std::env::temp_dir().join(format!("洪荒会话恢复测试上下文_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 驱动器 = Arc::new(五层协作驱动器::新(
            看板.clone(), 上下文, 对话器, Arc::new(模拟执行器), 10,
        ));
        状态.看板驱动台.装配(驱动器);
    }

    /// 装配驱动台 + 检查点回调（每轮末落盘运行断点到会话存储，供 Resume/Fork 恢复）
    fn 装配驱动器带检查点(状态: &数据服务状态, 看板: &Arc<Mutex<TaskBoard>>, 对话器: Arc<模拟对话器>) {
        let 序号 = 会话序号.fetch_add(1, Ordering::SeqCst);
        let 上下文 = Arc::new(Mutex::new(ContextManager::新(
            std::env::temp_dir().join(format!("洪荒会话恢复测试上下文_检查点_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 驱动台 = 状态.看板驱动台.clone();
        let 驱动器 = 五层协作驱动器::新(看板.clone(), 上下文, 对话器, Arc::new(模拟执行器), 10);
        let 驱动器 = 驱动器.设置检查点回调(Arc::new(move |任务id, 角色名, 轮次, 阶段提示, 消息, 清单| {
            驱动台.写入检查点(任务id, 角色名, 轮次, 阶段提示, 消息, 清单);
        }));
        状态.看板驱动台.装配(Arc::new(驱动器));
    }

    /// 通过 HTTP handler 发布任务（状态=待圣人设计、发起人=道祖）
    async fn 发布任务(状态: &数据服务状态, 标题: &str) {
        let 请求 = Json(发布任务请求 {
            title: 标题.into(),
            description: "会话恢复测试描述".into(),
            scene: None,
            priority: None,
        });
        let Json(_) = 看板发布(State(状态.clone()), 请求).await.expect("发布应成功");
    }

    fn 设计样例() -> &'static str {
        r#"{"边界定义":{"模块":"a"},"安全区域":[],"契约":[{"契约名":"测试契约","方法":[{"名称":"方法一","签名":"fn 方法一()","描述":"做某事"}],"描述":"契约描述"}],"修改文件":[],"新建文件":[],"依赖":[]}"#
    }

    /// 恢复接口：源会话无该任务检查点时返回 404
    #[tokio::test]
    async fn 会话接口_恢复无检查点返回错误() {
        let (状态, _看板) = 驱动状态();
        let 结果 = 会话恢复接口(
            State(状态.clone()),
            Path(1),
            Json(会话定向请求 { 任务id: 1 }),
        ).await;
        let Err((码, Json(体))) = 结果 else {
            panic!("无检查点的恢复应返回错误");
        };
        assert_eq!(码, StatusCode::NOT_FOUND, "无检查点应 404");
        assert!(体.错误.contains("检查点"), "错误应提示检查点：{}", 体.错误);
    }

    /// 分叉接口：装配后从源会话分叉，返回新会话 id
    #[tokio::test]
    async fn 会话接口_分叉返回新会话id() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![])));
        let Json(体) = 会话分叉接口(
            State(状态.clone()),
            Path(1),
            Json(会话分叉请求 { 任务id: 1, 继承消息: true }),
        ).await.expect("分叉应成功");
        assert!(体.会话id > 0, "分叉应返回正的新会话 id");
    }

    /// 分叉接口：不继承消息模式同样派生新会话
    #[tokio::test]
    async fn 会话接口_分叉不继承消息返回新会话id() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![])));
        let Json(体) = 会话分叉接口(
            State(状态.clone()),
            Path(2),
            Json(会话分叉请求 { 任务id: 1, 继承消息: false }),
        ).await.expect("分叉应成功");
        assert!(体.会话id > 0, "不继承消息的分叉也应返回正的新会话 id");
    }

    /// 会话清单：分叉派生后清单可见历史会话
    #[tokio::test]
    async fn 会话接口_清单返回历史会话() {
        let (状态, 看板) = 驱动状态();
        装配驱动器(&状态, &看板, Arc::new(模拟对话器::新(vec![])));
        let Json(分叉体) = 会话分叉接口(
            State(状态.clone()),
            Path(1),
            Json(会话分叉请求 { 任务id: 1, 继承消息: true }),
        ).await.expect("分叉应成功");

        let Json(清单体) = 会话清单接口(State(状态.clone())).await;
        assert!(!清单体.会话.is_empty(), "清单应包含刚派生的会话");
        assert!(清单体.会话.iter().any(|s| s.会话id == 分叉体.会话id), "清单应包含分叉会话 id");
    }

    /// 恢复接口：检查点角色阶段已推进时闸门拦截返回 400
    #[tokio::test]
    async fn 会话接口_恢复已推进阶段闸门拦截() {
        let (状态, 看板) = 驱动状态();
        let 对话器 = Arc::new(模拟对话器::新(vec![
            模型响应 {
                内容: None,
                工具调用: vec![工具调用 {
                    id: "call-1".into(),
                    名称: "列目录".into(),
                    参数: r#"{"路径": "."}"#.into(),
                }],
            },
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ]));
        装配驱动器带检查点(&状态, &看板, 对话器);
        发布任务(&状态, "断点恢复任务").await;
        assert!(状态.看板驱动台.等待完成(5000), "发布后应自动驱动完成");

        let 会话id = 状态.看板驱动台.会话清单()
            .first()
            .map(|s| s.会话id)
            .expect("应存在驱动会话");

        // 任务已推进到 待大罗金仙实现，检查点角色「圣人」的阶段已完成，恢复应被闸门拦截
        let 结果 = 会话恢复接口(
            State(状态.clone()),
            Path(会话id),
            Json(会话定向请求 { 任务id: 1 }),
        ).await;
        let Err((码, Json(体))) = 结果 else {
            panic!("已推进阶段的恢复应被闸门拦截");
        };
        assert_eq!(码, StatusCode::BAD_REQUEST, "闸门拦截应返回 400");
        assert!(
            体.错误.contains("已推进") || 体.错误.contains("无需恢复"),
            "错误应说明无需恢复：{}", 体.错误
        );
    }
}
