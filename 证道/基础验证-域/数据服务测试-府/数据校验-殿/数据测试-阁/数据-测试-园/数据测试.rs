#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use axum::{extract::{Path, State}, http::StatusCode, Json};
    use hm_http::{
        数据服务状态, 任务列表, 查询任务, 创建任务, 创建任务请求, 图谱查询, 日志列表, 记日志, 记日志请求,
        开发执行台, 受理开发任务, 受理失败, 事件记录,
    };
    use hm_contract::Component;
    use hm_execute_contract::{开发执行契约, 开发事件, 开发事件类型};
    use hm_error::{Error, Result};
    use hm_log::运行日志记录器;
    use hm_cognition::{图谱, 心智地图, 过程上下文};
    use hm_domain_contract::{任务仓库契约, 迭代日志契约, 记忆库契约, 规则库契约, 事件总线契约};
    use tc_task::{Task, TaskStatus, TaskStore};
    use lj_iteration::{Iteration, Version, IterationLog};
    use qk_memory::{Memory, MemoryStore};
    use dy_rule::{Rule, RuleSet};
    use hd_event::{Event, EventBus};

    fn 构造状态() -> 数据服务状态 {
        let 任务仓库: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>> = Arc::new(Mutex::new(TaskStore::new()));
        let 迭代日志: Arc<Mutex<dyn 迭代日志契约<Iteration, Version>>> = Arc::new(Mutex::new(IterationLog::new()));
        let 记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>> = Arc::new(Mutex::new(MemoryStore::new()));
        let 规则库: Arc<Mutex<dyn 规则库契约<Rule>>> = Arc::new(Mutex::new(RuleSet::new()));
        let 事件总线: Arc<Mutex<dyn 事件总线契约<Event>>> = Arc::new(Mutex::new(EventBus::new()));
        let 图谱 = Arc::new(Mutex::new(图谱::新()));
        let 心智地图 = Arc::new(Mutex::new(心智地图::新()));
        let 语境 = Arc::new(Mutex::new(过程上下文::新()));
        let 日志记录器 = Arc::new(Mutex::new(运行日志记录器::new()));
        let 开发执行台 = Arc::new(开发执行台::新());
        数据服务状态::新(任务仓库, 迭代日志, 记忆库, 规则库, 事件总线, 图谱, 心智地图, 语境, 日志记录器, 开发执行台)
    }

    /// 模拟开发执行器：按步进轮询中断标志，模拟智能体的同步阻塞执行
    struct 模拟开发执行器 {
        执行毫秒: u64,
        中断标志: Arc<AtomicBool>,
    }

    impl Component for 模拟开发执行器 {
        fn name(&self) -> &'static str { "模拟开发执行器" }
    }

    impl 开发执行契约 for 模拟开发执行器 {
        fn 执行开发任务(&self, 任务: String) -> Result<String> {
            let 步数 = (self.执行毫秒 / 25).max(1);
            for _ in 0..步数 {
                if self.中断标志.load(Ordering::SeqCst) {
                    return Err(Error::中断("模拟执行被中断".into()));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(format!("已处理: {任务}"))
        }

        fn 中断句柄(&self) -> Arc<AtomicBool> {
            self.中断标志.clone()
        }
    }

    /// 轮询等待执行台空闲（超时即 panic，避免测试悬挂）
    fn 等待执行结束(台: &开发执行台, 超时毫秒: u64) {
        let 起始 = std::time::Instant::now();
        while 台.运行中() {
            if 起始.elapsed().as_millis() as u64 > 超时毫秒 {
                panic!("执行超时未结束");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    #[test]
    fn 日志记录器_记日志后全部返回该记录() {
        let mut 记录器 = 运行日志记录器::new();
        记录器.记日志("【完成】".into(), "ok".into(), "任务完成".into());

        let 全部 = 记录器.全部();
        assert_eq!(全部.len(), 1);
        assert_eq!(全部[0].标签, "【完成】");
        assert_eq!(全部[0].内容, "任务完成");
        assert!(全部[0].时间 > 0);
    }

    #[test]
    fn 日志记录器_环形缓冲超上限丢弃最旧() {
        let mut 记录器 = 运行日志记录器::new_with_容量(2);
        记录器.记日志("一".into(), "act".into(), "第一条".into());
        记录器.记日志("二".into(), "act".into(), "第二条".into());
        记录器.记日志("三".into(), "act".into(), "第三条".into());

        let 全部 = 记录器.全部();
        assert_eq!(全部.len(), 2);
        assert_eq!(全部[0].内容, "第二条");
        assert_eq!(全部[1].内容, "第三条");
    }

    #[tokio::test]
    async fn 数据服务_任务列表返回任务实体() {
        let 状态 = 构造状态();
        状态.任务仓库.lock().expect("锁").创建("标题甲".into(), "描述甲".into()).expect("创建成功");
        状态.任务仓库.lock().expect("锁").创建("标题乙".into(), "描述乙".into()).expect("创建成功");

        let Json(任务) = 任务列表(State(状态)).await;
        assert_eq!(任务.len(), 2);
        assert_eq!(任务[0].title, "标题甲");
        assert_eq!(任务[1].title, "标题乙");
    }

    #[tokio::test]
    async fn 数据服务_任务查询存在id返回标题() {
        let 状态 = 构造状态();
        let id = 状态.任务仓库.lock().expect("锁").创建("查询我".into(), "描述".into()).expect("创建成功");

        let 结果 = 查询任务(State(状态), Path(id)).await;
        assert!(结果.is_ok());
        assert_eq!(结果.expect("应查询成功").0.title, "查询我");
    }

    #[tokio::test]
    async fn 数据服务_任务查询不存在id返回404() {
        let 状态 = 构造状态();

        let 结果 = 查询任务(State(状态), Path(999)).await;
        assert_eq!(结果.expect_err("应返回错误"), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn 数据服务_创建任务返回新id并入库() {
        let 状态 = 构造状态();
        let 结果 = 创建任务(State(状态.clone()), Json(创建任务请求 {
            标题: "新任务".into(),
            描述: "详情".into(),
        }))
        .await;
        let Json(id) = 结果.expect("创建应成功");
        assert!(id > 0);

        let 守卫 = 状态.任务仓库.lock().expect("锁");
        assert_eq!(守卫.全部().len(), 1);
        assert_eq!(守卫.查询(id).expect("应存在").title, "新任务");
    }

    #[tokio::test]
    async fn 数据服务_认知图谱返回空结构() {
        let 状态 = 构造状态();

        let Json(图) = 图谱查询(State(状态)).await;
        assert!(图.模块集.is_empty());
        assert!(图.符号集.is_empty());
        assert!(图.依赖集.is_empty());
    }

    #[tokio::test]
    async fn 数据服务_日志列表返回记录() {
        let 状态 = 构造状态();
        let _ = 记日志(State(状态.clone()), Json(记日志请求 {
            标签: "【就绪】".into(),
            样式: "ok".into(),
            内容: "启动".into(),
        }))
        .await;

        let Json(记录) = 日志列表(State(状态)).await;
        assert!(!记录.is_empty());
        assert_eq!(记录[0].标签, "【就绪】");
        assert_eq!(记录[0].内容, "启动");
        assert!(记录[0].时间 > 0);
    }

    #[test]
    fn 持久化_任务保存后加载数据一致() {
        let mut store = TaskStore::new();
        store.创建("持久化任务".into(), "持久化描述".into()).expect("创建成功");
        let 临时 = std::env::temp_dir().join("zd_http_test_任务.toml");
        let 路径 = 临时.to_string_lossy().to_string();
        store.保存(&路径).expect("保存成功");

        let 加载 = TaskStore::加载(&路径).expect("加载成功");
        let 全部 = 加载.全部();
        assert_eq!(全部.len(), 1);
        assert_eq!(全部[0].title, "持久化任务");
        assert_eq!(全部[0].description, "持久化描述");
        let _ = std::fs::remove_file(&临时);
    }

    #[test]
    fn 受理_返回任务id且完成后任务已完成() {
        let 状态 = 构造状态();
        状态.开发执行台.装配(
            Arc::new(模拟开发执行器 { 执行毫秒: 60, 中断标志: Arc::new(AtomicBool::new(false)) }),
            "F:/临时工作区",
        );

        let id = 受理开发任务(&状态, "修复登录超时".into()).expect("受理应成功");
        assert!(id > 0);

        等待执行结束(&状态.开发执行台, 5000);
        let 守卫 = 状态.任务仓库.lock().expect("锁");
        let 任务 = 守卫.查询(id).expect("任务应存在");
        assert_eq!(任务.title.contains("修复登录超时"), true);
        assert_eq!(任务.status, TaskStatus::已完成);
        drop(守卫);
        assert_eq!(状态.开发执行台.当前状态().最近结果.expect("应有结果"), "已处理: 修复登录超时");
        assert!(!状态.开发执行台.运行中());
    }

    #[test]
    fn 受理_执行中重复受理返回运行中() {
        let 状态 = 构造状态();
        状态.开发执行台.装配(
            Arc::new(模拟开发执行器 { 执行毫秒: 500, 中断标志: Arc::new(AtomicBool::new(false)) }),
            "F:/临时工作区",
        );

        let 首次 = 受理开发任务(&状态, "第一个任务".into());
        assert!(首次.is_ok());
        let 重复 = 受理开发任务(&状态, "第二个任务".into());
        assert!(matches!(重复.expect_err("应拒绝重复受理"), 受理失败::运行中));

        状态.开发执行台.中断();
        等待执行结束(&状态.开发执行台, 5000);
    }

    #[test]
    fn 受理_未装配返回未上线() {
        let 状态 = 构造状态();

        let 结果 = 受理开发任务(&状态, "任何任务".into());
        assert!(matches!(结果.expect_err("应拒绝受理"), 受理失败::未上线));
    }

    #[test]
    fn 受理_空任务返回任务为空() {
        let 状态 = 构造状态();
        状态.开发执行台.装配(
            Arc::new(模拟开发执行器 { 执行毫秒: 0, 中断标志: Arc::new(AtomicBool::new(false)) }),
            "F:/临时工作区",
        );

        let 结果 = 受理开发任务(&状态, "   ".into());
        assert!(matches!(结果.expect_err("应拒绝空任务"), 受理失败::任务为空));
    }

    #[test]
    fn 事件流_受理时清空且序号从1递增() {
        let 台 = 开发执行台::新();
        台.记录事件(&开发事件 {
            轮次: 0,
            类型: 开发事件类型::思考,
            工具名: String::new(),
            内容: "旧会话事件".into(),
        });
        台.记录事件(&开发事件 {
            轮次: 1,
            类型: 开发事件类型::任务答复,
            工具名: String::new(),
            内容: "旧会话答复".into(),
        });
        assert_eq!(台.事件增量(0).len(), 2);
        assert_eq!(台.事件增量(0)[1].序号, 2);

        // 预留成功即清空上一次会话
        assert!(台.预留());
        assert!(台.事件增量(0).is_empty());

        // 新记录序号从 1 重新递增
        台.记录事件(&开发事件 {
            轮次: 0,
            类型: 开发事件类型::思考,
            工具名: String::new(),
            内容: "新会话事件".into(),
        });
        let 增量: Vec<事件记录> = 台.事件增量(0);
        assert_eq!(增量.len(), 1);
        assert_eq!(增量[0].序号, 1);
        assert_eq!(增量[0].内容, "新会话事件");
        台.释放();
    }

    #[test]
    fn 事件流_since增量与超界() {
        let 台 = 开发执行台::新();
        for 轮次 in 0..3 {
            台.记录事件(&开发事件 {
                轮次,
                类型: 开发事件类型::思考,
                工具名: String::new(),
                内容: format!("事件{轮次}"),
            });
        }

        let 增量 = 台.事件增量(1);
        assert_eq!(增量.len(), 2);
        assert_eq!(增量[0].序号, 2);
        assert_eq!(增量[1].序号, 3);
        assert!(台.事件增量(999).is_empty());
    }

    #[test]
    fn 停止_置位中断并任务已取消() {
        let 状态 = 构造状态();
        状态.开发执行台.装配(
            Arc::new(模拟开发执行器 { 执行毫秒: 2000, 中断标志: Arc::new(AtomicBool::new(false)) }),
            "F:/临时工作区",
        );

        let id = 受理开发任务(&状态, "需要中断的长任务".into()).expect("受理应成功");
        std::thread::sleep(Duration::from_millis(100));

        assert!(状态.开发执行台.中断());
        等待执行结束(&状态.开发执行台, 5000);

        let 守卫 = 状态.任务仓库.lock().expect("锁");
        assert_eq!(守卫.查询(id).expect("任务应存在").status, TaskStatus::已取消);
        drop(守卫);
        let 最近 = 状态.开发执行台.当前状态().最近结果.expect("应有失败摘要");
        assert!(最近.contains("执行失败"));
    }
}
