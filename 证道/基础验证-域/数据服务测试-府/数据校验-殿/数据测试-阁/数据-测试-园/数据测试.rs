#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use axum::{extract::{Path, State}, http::StatusCode, Json};
    use hm_http::{
        数据服务状态, 任务列表, 查询任务, 创建任务, 创建任务请求, 图谱查询, 日志列表, 记日志, 记日志请求,
    };
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
        数据服务状态::新(任务仓库, 迭代日志, 记忆库, 规则库, 事件总线, 图谱, 心智地图, 语境, 日志记录器)
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
}