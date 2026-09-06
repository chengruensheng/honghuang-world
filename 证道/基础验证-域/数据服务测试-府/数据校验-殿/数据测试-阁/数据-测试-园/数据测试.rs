#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use axum::{extract::{Path, State}, http::StatusCode, Json};
    use hm_http::{
        数据服务状态, 任务列表, 查询任务, 创建任务, 创建任务请求, 图谱查询, 日志列表, 记日志, 记日志请求,
        看板列表, 看板查询, 看板发布, 看板承接, 看板提交, 看板清理, 发布任务请求, 承接任务请求, 提交任务请求,
        开发执行台, 受理开发任务, 受理失败, 事件记录,
        模型状态接口, 模型列表接口, 模型选择接口, LLM选择请求,
        模型模板接口, 模型探测接口, 模型接入接口, LLM探测请求, LLM接入请求,
    };
    use hm_content::LLM池;
    use hm_config::{LlmConfig, LlmProvider};
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

    static 看板序号: AtomicU64 = AtomicU64::new(0);

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
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 任务看板 = Arc::new(Mutex::new(tc_task::TaskBoard::新建(
            std::env::temp_dir().join(format!("洪荒测试看板_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        let 开发执行台 = Arc::new(开发执行台::新());
        let 看板驱动台 = Arc::new(hm_http::看板驱动台::新());
        数据服务状态::新(任务仓库, 迭代日志, 记忆库, 规则库, 事件总线, 图谱, 心智地图, 语境, 任务看板, 日志记录器, 开发执行台, 看板驱动台, None, None)
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
            &std::env::temp_dir().to_string_lossy(),
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
        // 直接经开发执行台验证中断语义（受理编排已迁移到看板驱动，不经受理）
        let 台 = Arc::new(开发执行台::新());
        let 中断标志 = Arc::new(AtomicBool::new(false));
        台.装配(
            Arc::new(模拟开发执行器 { 执行毫秒: 2000, 中断标志: 中断标志.clone() }),
            &std::env::temp_dir().to_string_lossy(),
        );
        let 仓库: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>> = Arc::new(Mutex::new(TaskStore::new()));
        let id = 仓库.lock().expect("锁").创建("需要中断的长任务".into(), "中断测试".into()).expect("创建成功");
        let 结束仓库 = 仓库.clone();
        let 结束通知 = Arc::new(move |结果: &Result<String>| {
            let 目标 = if 结果.is_ok() { TaskStatus::已完成 } else { TaskStatus::已取消 };
            let _ = 结束仓库.lock().expect("锁").推进(id, 目标);
        });
        assert!(台.预留());
        台.启动执行("需要中断的长任务".into(), 结束通知);
        std::thread::sleep(Duration::from_millis(100));

        assert!(台.中断());
        等待执行结束(&台, 5000);

        let 守卫 = 仓库.lock().expect("锁");
        assert_eq!(守卫.查询(id).expect("任务应存在").status, TaskStatus::已取消);
        drop(守卫);
        let 最近 = 台.当前状态().最近结果.expect("应有失败摘要");
        assert!(最近.contains("执行失败"));
    }

    #[tokio::test]
    async fn 看板_发布任务后列表包含该任务() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "看板测试任务".into(),
            description: "验证发布".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");
        assert_eq!(id, 1);

        let Json(任务) = 看板列表(State(状态), Query(Default::default())).await;
        assert_eq!(任务.len(), 1);
        assert_eq!(任务[0].title, "看板测试任务");
        assert_eq!(任务[0].status, TaskStatus::待圣人设计);
    }

    #[tokio::test]
    async fn 看板_查询不存在id返回404() {
        let 状态 = 构造状态();
        let 结果 = 看板查询(State(状态), Path(999)).await;
        assert_eq!(结果.expect_err("应返回404"), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn 看板_查询存在id返回任务详情() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "详情测试".into(),
            description: "验证查询".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
            panic!("查询应成功");
        };
        assert_eq!(任务.title, "详情测试");
        assert_eq!(任务.status, TaskStatus::待圣人设计);
    }

    #[tokio::test]
    async fn 看板_圣人承接待圣人设计任务() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "承接测试".into(),
            description: "验证承接".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "圣人".into(),
        })).await.expect("承接应成功");

        let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
            panic!("查询应成功");
        };
        assert_eq!(任务.status, TaskStatus::圣人设计中);
        assert_eq!(任务.当前承接人, tc_task::AgentRole::圣人);
    }

    #[tokio::test]
    async fn 看板_角色不符承接返回错误() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "角色不符".into(),
            description: "验证拒绝".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        let 结果 = 看板承接(State(状态), Path(id), Json(承接任务请求 {
            role: "大罗金仙".into(),
        })).await;
        assert!(结果.is_err());
    }

    #[tokio::test]
    async fn 看板_圣人提交任务流转到待大罗金仙实现() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "提交测试".into(),
            description: "验证提交".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "圣人".into(),
        })).await.expect("承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "圣人".into(),
            next_status: "待大罗金仙实现".into(),
        })).await.expect("提交应成功");

        let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
            panic!("查询应成功");
        };
        assert_eq!(任务.status, TaskStatus::待大罗金仙实现);
    }

    #[tokio::test]
    async fn 看板_筛选按状态返回对应任务() {
        let 状态 = 构造状态();
        let _ = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "任务一".into(),
            description: "".into(),
            scene: None,
            priority: None,
        })).await.expect("发布1");
        let _ = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "任务二".into(),
            description: "".into(),
            scene: None,
            priority: None,
        })).await.expect("发布2");

        看板承接(State(状态.clone()), Path(1), Json(承接任务请求 {
            role: "圣人".into(),
        })).await.expect("圣人承接任务一");

        let Json(圣人设计中) = 看板列表(State(状态.clone()), Query(看板筛选参数 {
            status: Some("圣人设计中".into()),
            role: None,
        })).await;
        assert_eq!(圣人设计中.len(), 1);
        assert_eq!(圣人设计中[0].title, "任务一");

        let Json(待圣人设计) = 看板列表(State(状态), Query(看板筛选参数 {
            status: Some("待圣人设计".into()),
            role: None,
        })).await;
        assert_eq!(待圣人设计.len(), 1);
        assert_eq!(待圣人设计[0].title, "任务二");
    }

    use axum::extract::Query;
    use hm_http::看板筛选参数;

    /// 构造完整装配状态（五行相生桥接已串联），用于看板驱动五行闭环测试
    fn 构造装配状态() -> 数据服务状态 {
        let 装配 = hm_linkage::五行装配::装配();
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 任务看板 = Arc::new(Mutex::new(tc_task::TaskBoard::新建(
            std::env::temp_dir().join(format!("洪荒测试看板_装配_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        {
            let mut 看板 = 任务看板.lock().expect("看板锁中毒");
            看板.设置信号总线(装配.信号总线.clone());
        }
        数据服务状态::新(
            装配.任务仓库.clone(),
            装配.迭代日志.clone(),
            装配.记忆库.clone(),
            装配.规则库.clone(),
            装配.事件总线.clone(),
            装配.图谱.clone(),
            装配.心智地图.clone(),
            装配.语境.clone(),
            任务看板,
            装配.日志记录器.clone(),
            Arc::new(开发执行台::新()),
            Arc::new(hm_http::看板驱动台::新()),
            None,
            None,
        )
    }

    #[tokio::test]
    async fn 看板_发布任务触发任务推进信号() {
        let 状态 = 构造装配状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "信号测试".into(),
            description: "验证发布信号".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
            panic!("查询应成功");
        };
        assert_eq!(任务.title, "信号测试");
    }

    #[tokio::test]
    async fn 看板_任务完成触发木生火开启迭代() {
        let 状态 = 构造装配状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "闭环测试".into(),
            description: "验证木生火".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "圣人".into(),
        })).await.expect("承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "圣人".into(),
            next_status: "待大罗金仙实现".into(),
        })).await.expect("提交应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "大罗金仙".into(),
        })).await.expect("大罗金仙承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "大罗金仙".into(),
            next_status: "待准圣验收".into(),
        })).await.expect("提交应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "准圣".into(),
        })).await.expect("准圣承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "准圣".into(),
            next_status: "待道祖终审".into(),
        })).await.expect("提交应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "道祖".into(),
        })).await.expect("道祖承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "道祖".into(),
            next_status: "待清理".into(),
        })).await.expect("道祖提交应成功");

        // 六层流转：终审通过后由太乙金仙清理收尾，清理完成触发 木生火
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "太乙金仙".into(),
        })).await.expect("太乙金仙承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "太乙金仙".into(),
            next_status: "清理完成".into(),
        })).await.expect("太乙金仙提交应成功");

        let 迭代日志 = 状态.迭代日志.lock().expect("迭代日志锁中毒");
        let 迭代列表 = 迭代日志.全部();
        assert!(!迭代列表.is_empty(), "木生火应已开启迭代");
        assert!(迭代列表.iter().any(|it| it.变更说明.contains("闭环测试")),
            "迭代变更说明应包含任务标题");
    }

    #[tokio::test]
    async fn 看板_清理一键接口完成待清理任务() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "清理测试".into(),
            description: "验证一键清理".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        // 推进到待清理状态
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "圣人".into(),
        })).await.expect("圣人承接");
        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "圣人".into(),
            next_status: "待大罗金仙实现".into(),
        })).await.expect("圣人提交");
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "大罗金仙".into(),
        })).await.expect("大罗金仙承接");
        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "大罗金仙".into(),
            next_status: "待准圣验收".into(),
        })).await.expect("大罗金仙提交");
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "准圣".into(),
        })).await.expect("准圣承接");
        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "准圣".into(),
            next_status: "待道祖终审".into(),
        })).await.expect("准圣提交");
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "道祖".into(),
        })).await.expect("道祖承接");
        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "道祖".into(),
            next_status: "待清理".into(),
        })).await.expect("道祖提交到待清理");

        // 一键清理
        看板清理(State(状态.clone()), Path(id)).await.expect("清理应成功");

        let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
            panic!("查询应成功");
        };
        assert_eq!(任务.status, TaskStatus::清理完成);
        assert_eq!(任务.当前承接人, tc_task::AgentRole::太乙金仙);
    }

    #[tokio::test]
    async fn 看板_清理接口对非待清理状态返回错误() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "不可清理".into(),
            description: "验证错误清理".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        let 结果 = 看板清理(State(状态), Path(id)).await;
        assert!(结果.is_err(), "对待圣人设计状态的任务清理应失败");
    }

    /// 构造带 LLM 池的状态（复用 构造状态 的五引擎装配，仅覆写池字段）
    fn 构造池状态(池: Arc<LLM池>) -> 数据服务状态 {
        let mut 状态 = 构造状态();
        状态.llm池 = Some(池);
        状态
    }

    fn 池供应商(名: &str, 端点: &str, 列表: &str, 密钥: &str, 模型: &str) -> LlmProvider {
        LlmProvider {
            name: 名.into(),
            kind: "openai".into(),
            base_url: 端点.into(),
            models_url: 列表.into(),
            api_key: 密钥.into(),
            model: 模型.into(),
            timeout_secs: 2,
            retry: 0,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn 模型接口_未装配返回未配置() {
        let 状态 = 构造状态();
        let Json(响应) = 模型状态接口(State(状态)).await;
        assert!(!响应.配置);
        assert!(响应.供应商.is_empty());
        assert!(响应.当前选择.is_none());
    }

    #[tokio::test]
    async fn 模型接口_状态返回选择与供应商() {
        let 甲地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 池 = LLM池::从配置(&LlmConfig {
            providers: vec![池供应商("甲", &format!("http://{甲地址}/chat"), &format!("http://{甲地址}/models"), "secret-http", "甲模型")],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        });
        let 状态 = 构造池状态(Arc::new(池));
        let Json(响应) = 模型状态接口(State(状态)).await;
        // 状态接口序列化不得泄漏密钥（先整体借用，避免部分 move）
        let json = serde_json::to_string(&响应).unwrap();
        assert!(!json.contains("secret-http"));
        assert!(响应.配置);
        let 选择 = 响应.当前选择.clone().unwrap();
        assert_eq!(选择.供应商, "甲");
        assert_eq!(选择.模型, "甲模型");
        assert_eq!(响应.供应商.len(), 1);
        assert_eq!(响应.供应商[0].名称, "甲");
    }

    #[tokio::test]
    async fn 模型接口_模型列表聚合() {
        // 甲：本地可访问 list-models；乙：端口 1 拒绝连接 → 标注错误
        let 监听 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let 地址 = 监听.local_addr().unwrap();
        let 列表体 = r#"{"data":[{"id":"甲-1"},{"id":"甲-2"}]}"#.to_string();
        std::thread::spawn(move || {
            for 连接 in 监听.incoming() {
                if let Ok(mut 流) = 连接 {
                    use std::io::{Read, Write};
                    let mut 缓冲: Vec<u8> = Vec::new();
                    let mut 临时 = [0u8; 4096];
                    loop {
                        let n = 流.read(&mut 临时).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        缓冲.extend_from_slice(&临时[..n]);
                        if String::from_utf8_lossy(&缓冲).contains("\r\n\r\n") {
                            break;
                        }
                    }
                    let 响应 = format!(
                        "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        列表体.len(),
                        列表体,
                    );
                    let _ = 流.write_all(响应.as_bytes());
                    let _ = 流.flush();
                }
            }
        });
        let 乙地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 池 = LLM池::从配置(&LlmConfig {
            providers: vec![
                池供应商("甲", &format!("http://{地址}/chat"), &format!("http://{地址}/models"), "key-a", "甲模型"),
                池供应商("乙", &format!("http://{乙地址}/chat"), &format!("http://{乙地址}/models"), "key-b", "乙模型"),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        });
        let 状态 = 构造池状态(Arc::new(池));
        let Json(响应) = 模型列表接口(State(状态)).await;
        assert!(响应.配置);
        assert_eq!(响应.结果.len(), 2);
        let 甲条 = 响应.结果.iter().find(|r| r.供应商 == "甲").unwrap();
        let ids: Vec<&str> = 甲条.模型.as_ref().unwrap().iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["甲-1", "甲-2"]);
        let 乙条 = 响应.结果.iter().find(|r| r.供应商 == "乙").unwrap();
        assert!(乙条.错误.is_some(), "乙应标注列表错误，实际: {乙条:?}");
    }

    #[tokio::test]
    async fn 模型接口_选择生效() {
        let 甲地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 乙地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 池 = LLM池::从配置(&LlmConfig {
            providers: vec![
                池供应商("甲", &format!("http://{甲地址}/chat"), &format!("http://{甲地址}/models"), "key-a", "甲模型"),
                池供应商("乙", &format!("http://{乙地址}/chat"), &format!("http://{乙地址}/models"), "key-b", "乙模型"),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        });
        let 状态 = 构造池状态(Arc::new(池));
        let Json(响应) = 模型选择接口(
            State(状态.clone()),
            Json(LLM选择请求 { 供应商: "乙".into(), 模型: "乙模型".into() }),
        )
        .await
        .unwrap();
        assert!(响应.成功);
        let Json(状态响应) = 模型状态接口(State(状态)).await;
        let 选择 = 状态响应.当前选择.unwrap();
        assert_eq!(选择.供应商, "乙");
        assert_eq!(选择.模型, "乙模型");
    }

    #[tokio::test]
    async fn 模型接口_选择非法供应商报错() {
        let 甲地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 池 = LLM池::从配置(&LlmConfig {
            providers: vec![池供应商("甲", &format!("http://{甲地址}/chat"), &format!("http://{甲地址}/models"), "key-a", "甲模型")],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        });
        let 状态 = 构造池状态(Arc::new(池));
        let 结果 = 模型选择接口(
            State(状态),
            Json(LLM选择请求 { 供应商: "不存在".into(), 模型: "某模型".into() }),
        )
        .await;
        assert!(结果.is_err());
        assert_eq!(结果.unwrap_err(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn 模型接口_模板清单返回() {
        let Json(响应) = 模型模板接口().await;
        assert!(响应.模板.len() >= 9);
        assert!(响应.模板.iter().any(|t| t.名称 == "deepseek"));
        let json = serde_json::to_string(&响应).unwrap();
        assert!(!json.contains("api_key") && !json.contains("密钥"), "模板不得含密钥字段: {json}");
    }

    #[tokio::test]
    async fn 模型接口_探测_目录命中() {
        let 状态 = 构造状态();
        let Json(响应) = 模型探测接口(State(状态), Json(LLM探测请求 {
            供应商: Some("deepseek".into()),
            地址: None,
            密钥: None,
        })).await;
        assert_eq!(响应.来源, "目录");
        assert!(响应.模型.iter().any(|m| m.id == "deepseek-chat"));
        assert!(响应.错误.is_none());
    }

    #[tokio::test]
    async fn 模型接口_探测_自定义网络() {
        let 监听 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let 地址 = 监听.local_addr().unwrap();
        let 列表体 = r#"{"data":[{"id":"探测-1"},{"id":"探测-2"}]}"#.to_string();
        std::thread::spawn(move || {
            for 连接 in 监听.incoming() {
                if let Ok(mut 流) = 连接 {
                    use std::io::{Read, Write};
                    let mut 缓冲: Vec<u8> = Vec::new();
                    let mut 临时 = [0u8; 4096];
                    loop {
                        let n = 流.read(&mut 临时).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        缓冲.extend_from_slice(&临时[..n]);
                        if String::from_utf8_lossy(&缓冲).contains("\r\n\r\n") {
                            break;
                        }
                    }
                    let 响应 = format!(
                        "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        列表体.len(),
                        列表体,
                    );
                    let _ = 流.write_all(响应.as_bytes());
                    let _ = 流.flush();
                }
            }
        });
        let 状态 = 构造状态();
        let Json(响应) = 模型探测接口(State(状态), Json(LLM探测请求 {
            供应商: None,
            地址: Some(format!("http://{地址}")),
            密钥: None,
        })).await;
        assert_eq!(响应.来源, "网络");
        let ids: Vec<&str> = 响应.模型.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["探测-1", "探测-2"]);
    }

    #[tokio::test]
    async fn 模型接口_接入_缺字段400() {
        let 状态 = 构造池状态(Arc::new(LLM池::从配置(&LlmConfig {
            providers: vec![],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        })));
        let 结果 = 模型接入接口(State(状态), Json(LLM接入请求 {
            名称: String::new(),
            地址: "http://127.0.0.1:1/v1".into(),
            密钥: "key".into(),
            模型: "m".into(),
        })).await;
        assert!(结果.is_err());
        assert_eq!(结果.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn 模型接口_接入_成功含配置片段() {
        let 状态 = 构造池状态(Arc::new(LLM池::从配置(&LlmConfig {
            providers: vec![],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        })));
        let Json(响应) = 模型接入接口(State(状态), Json(LLM接入请求 {
            名称: "甲".into(),
            地址: "http://127.0.0.1:1/v1".into(),
            密钥: "secret-key".into(),
            模型: "甲模型".into(),
        })).await.unwrap();
        assert!(响应.成功);
        let 选择 = 响应.选择.clone().unwrap();
        assert_eq!(选择.供应商, "甲");
        assert_eq!(选择.模型, "甲模型");
        let 片段 = 响应.配置片段.clone().unwrap();
        assert!(片段.contains("base_url"), "配置片段应含 base_url: {片段}");
        assert!(片段.contains("secret-key"), "配置片段应含 env 提示: {片段}");
    }
}
