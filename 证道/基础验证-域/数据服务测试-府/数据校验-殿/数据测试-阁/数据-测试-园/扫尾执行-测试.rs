#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use axum::extract::{Path, State};
    use axum::http::StatusCode;
    use axum::Json;
    use hm_contract::Component;
    use hm_error::Result;
    use hm_execute::本地执行器;
    use hm_execute_contract::执行器;
    use hm_agent::{
        开发服务状态, 看板驱动台, 开发执行台,
        看板扫尾检查, 看板清理, 看板澄清, 澄清请求,
    };
    use tc_task::{AgentRole, CodeChange, ImplementationDoc, SelfCheckResult, Task, TaskBoard, TaskStatus, TaskStore, 扫尾记录};
    use hm_domain_contract::记忆库契约;
    use qk_memory::{Memory, MemoryStore};

    static 看板序号: AtomicU64 = AtomicU64::new(0);

    /// 模拟执行器：按名找文件返回预设快照（可配）
    struct 模拟执行器 {
        快照: String,
    }

    impl Component for 模拟执行器 {
        fn name(&self) -> &'static str { "模拟执行器" }
    }

    impl 执行器 for 模拟执行器 {
        fn 读文件(&self, _路径: &str) -> Result<String> { Ok("文件内容".to_string()) }
        fn 写文件(&self, _路径: &str, _内容: &str) -> Result<()> { Ok(()) }
        fn 运行命令(&self, _命令: &str) -> Result<String> { Ok("命令输出".to_string()) }
        fn 列目录(&self, _路径: &str) -> Result<String> { Ok("（空目录）".to_string()) }
        fn 按名找文件(&self, _模式: &str) -> Result<String> { Ok(self.快照.clone()) }
        fn 搜索内容(&self, _关键词: &str) -> Result<String> { Ok("（无匹配）".to_string()) }
        fn 精确编辑(&self, _路径: &str, _旧: &str, _新: &str) -> Result<String> { Ok("替换成功（1 处）".to_string()) }
    }

    /// 构造开发服务状态（hm-agent）：任务看板 + 开发执行台 + 看板驱动台 + 记忆库
    fn 基础状态(看板: &Arc<Mutex<TaskBoard>>) -> 开发服务状态<Memory> {
        let 记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>> = Arc::new(Mutex::new(MemoryStore::new()));
        开发服务状态::新(
            看板.clone(),
            Arc::new(开发执行台::新()),
            Arc::new(看板驱动台::新()),
            记忆库,
        )
    }

    /// 构造带 mock 执行器（可配快照）扫尾执行者的状态
    fn 带扫尾(状态: 开发服务状态<Memory>, 看板: &Arc<Mutex<TaskBoard>>, 快照: &str) -> 开发服务状态<Memory> {
        let 执行者 = hm_agent::扫尾执行者::新(Arc::new(模拟执行器 { 快照: 快照.into() }), 看板.clone());
        状态.设置扫尾执行者(Arc::new(执行者))
    }

    /// 发布任务并按标识改写注入实现文档（不经流转，聚焦扫尾）
    fn 发布带实现文档任务(看板: &Arc<Mutex<TaskBoard>>, 声明: &[&str], 自检通过: bool) -> u64 {
        let mut 板 = 看板.lock().expect("看板锁中毒");
        let id = 板.发布任务(Task::新建(0, "扫尾测试任务".into(), "描述".into(), 100)).unwrap();
        let 标识 = { 板.查询(id).unwrap().任务标识.任务id };
        let doc = ImplementationDoc {
            代码变更: 声明.iter().map(|p| CodeChange { 文件路径: p.to_string(), 变更类型: "修改".into(), 摘要: "实现".into() }).collect(),
            工具调用: vec![],
            自检: SelfCheckResult { 通过: 自检通过, 边界合规: true, 契约合规: true, 问题: vec![] },
            created_at: 100,
        };
        板.按标识改写(&标识, |t| {
            t.实现文档 = Some(doc);
            t.status = TaskStatus::待清理;
        }).expect("改写应成功");
        id
    }

    /// 测试1：mock 快照含全部声明 → 通过
    #[test]
    fn 扫尾执行_mock快照全兑现通过() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒扫尾_mock_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 带扫尾(基础状态(&看板), &看板, "src/main.rs\nsrc/lib.rs");
        let id = 发布带实现文档任务(&看板, &["src/main.rs", "src/lib.rs"], true);

        let 记录 = 状态.扫尾执行者.as_ref().expect("已装配").执行(id).unwrap();
        assert!(记录.通过, "声明全落地应通过: {:?}", 记录);
        assert_eq!(记录.变更总数, 2);
        assert_eq!(记录.兑现数, 2);
        assert!(记录.漂移项.is_empty());
        // 写回验证
        let 板 = 看板.lock().expect("看板锁中毒");
        let 任务 = 板.查询(id).unwrap();
        assert!(任务.扫尾记录.as_ref().map(|r| r.通过).unwrap_or(false));
    }

    /// 测试2：声明含缺失文件 → 不通过、未兑现数准确
    #[test]
    fn 扫尾执行_声明缺失检出未兑现() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒扫尾_缺失_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 带扫尾(基础状态(&看板), &看板, "src/main.rs");
        let id = 发布带实现文档任务(&看板, &["src/main.rs", "src/缺失.rs"], true);

        let 记录 = 状态.扫尾执行者.as_ref().expect("已装配").执行(id).unwrap();
        assert!(!记录.通过);
        assert_eq!(记录.未兑现数, 1);
        assert!(记录.漂移项.iter().any(|d| d.路径 == "src/缺失.rs"));
    }

    /// 测试3：声明缺一项但自检通过 → 漂移项含 声明未兑现 类型
    #[test]
    fn 扫尾执行_漂移项类型为声明未兑现() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒扫尾_漂移_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 带扫尾(基础状态(&看板), &看板, "src/main.rs");
        let id = 发布带实现文档任务(&看板, &["src/main.rs", "src/未产出.rs"], true);
        let 记录 = 状态.扫尾执行者.as_ref().expect("已装配").执行(id).unwrap();
        let 项 = 记录.漂移项.iter().find(|d| d.路径 == "src/未产出.rs").expect("应检出");
        assert_eq!(项.类型, tc_task::漂移类型::声明未兑现);
    }

    /// 测试4：工作区多余文件 → 不通过且多余数准确
    #[test]
    fn 扫尾执行_工作区未声明文件检出多余() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒扫尾_多余_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 带扫尾(基础状态(&看板), &看板, "src/main.rs\n可疑文件.tmp");
        let id = 发布带实现文档任务(&看板, &["src/main.rs"], true);
        let 记录 = 状态.扫尾执行者.as_ref().expect("已装配").执行(id).unwrap();
        assert!(!记录.通过, "有未声明文件应不通过");
        assert_eq!(记录.多余数, 1);
        assert!(记录.漂移项.iter().any(|d| d.路径 == "可疑文件.tmp"));
    }

    /// 测试5：实体执行器（临时目录）真实文件 → 通过且写回任务
    #[test]
    fn 扫尾执行_实体执行器真实工作区通过() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 临时 = std::env::temp_dir().join(format!("洪荒扫尾真实区_{序号}"));
        std::fs::create_dir_all(临时.join("src")).unwrap();
        std::fs::write(临时.join("src/main.rs"), "fn main() {}").unwrap();

        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒扫尾_真实_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 基础状态(&看板).设置扫尾执行者(Arc::new(hm_agent::扫尾执行者::新(
            Arc::new(本地执行器::new_with_limits(临时.to_string_lossy().as_ref(), 5, 1024)),
            看板.clone(),
        )));
        let id = 发布带实现文档任务(&看板, &["src/main.rs"], true);
        let 记录 = 状态.扫尾执行者.as_ref().expect("已装配").执行(id).unwrap();
        assert!(记录.通过, "真实文件应通过: {:?}", 记录);
        assert_eq!(记录.兑现数, 1);
        std::fs::remove_dir_all(&临时).ok();
    }

    /// 测试6：API——任务无实现文档 → 400
    #[tokio::test]
    async fn 扫尾接口_无实现文档返回400() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒扫尾_400_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 带扫尾(基础状态(&看板), &看板, "");
        let id = { 看板.lock().expect("看板锁中毒").发布任务(Task::新建(0, "无实现".into(), "d".into(), 100)).unwrap() };
        let 响应 = 看板扫尾检查(State(状态), Path(id)).await;
        match 响应 {
            Err((码, _)) => assert_eq!(码, StatusCode::BAD_REQUEST),
            Ok(_) => panic!("无实现文档应返回 400"),
        }
    }

    /// 测试7：API——任务不存在 → 404
    #[tokio::test]
    async fn 扫尾接口_任务不存在返回404() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒扫尾_404_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 带扫尾(基础状态(&看板), &看板, "");
        let 响应 = 看板扫尾检查(State(状态), Path(99999)).await;
        match 响应 {
            Err((码, _)) => assert_eq!(码, StatusCode::NOT_FOUND),
            Ok(_) => panic!("不存在任务应返回 404"),
        }
    }

    /// 测试8：API——未装配执行者 → 503
    #[tokio::test]
    async fn 扫尾接口_未装配返回503() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒扫尾_503_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 基础状态(&看板); // 未装配扫尾执行者
        let 响应 = 看板扫尾检查(State(状态), Path(1)).await;
        match 响应 {
            Err((码, _)) => assert_eq!(码, StatusCode::SERVICE_UNAVAILABLE),
            Ok(_) => panic!("未装配应返回 503"),
        }
    }

        /// 测试9：旧数据兼容——加载后扫尾记录为 None
    #[test]
    fn 扫尾记录_旧数据加载默认无() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 路径 = format!("{}/洪荒扫尾_旧数据_{序号}.jsonl", std::env::temp_dir().to_string_lossy());
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(路径.clone())));
        let id = 发布带实现文档任务(&看板, &["src/main.rs"], true);
        // 模拟旧数据：去掉扫尾记录字段再写盘
        let 板 = 看板.lock().expect("看板锁中毒");
        let 任务 = 板.查询(id).unwrap();
        let mut 值 = serde_json::to_value(任务).unwrap();
        值.as_object_mut().unwrap().remove("扫尾记录");
        let 旧行 = serde_json::to_string(&值).unwrap();
        drop(板);
        std::fs::write(&路径, 旧行).unwrap();
        let 新板 = TaskBoard::加载(&路径).unwrap();
        let 任务 = 新板.查询(id).unwrap();
        assert_eq!(任务.扫尾记录, None, "旧数据缺扫尾记录应默认 None");
        std::fs::remove_file(&路径).ok();
    }

    // ===== 任务1：清理前置强制扫尾（门禁） =====

    /// 测试10：清理 API——扫尾通过才承接+提交到 清理完成
    #[tokio::test]
    async fn 清理接口_扫尾通过则清理完成() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒清理_通过_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 带扫尾(基础状态(&看板), &看板, "src/main.rs");
        let id = 发布带实现文档任务(&看板, &["src/main.rs"], true);
        let 响应 = 看板清理(State(状态), Path(id)).await;
        let Ok(Json(清理结果)) = 响应 else {
            panic!("扫尾通过应清理成功");
        };
        assert_eq!(清理结果.任务id, id);
        assert!(清理结果.记录.as_ref().map(|r| r.通过).unwrap_or(false), "响应应带回通过的扫尾记录");
        let 板 = 看板.lock().expect("看板锁中毒");
        assert_eq!(板.查询(id).map(|t| t.status), Some(TaskStatus::清理完成));
    }

    /// 测试11：清理 API——扫尾未通过（声明缺失）→ 400 且保持待清理
    #[tokio::test]
    async fn 清理接口_扫尾未通过拒绝清理() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒清理_拒_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 带扫尾(基础状态(&看板), &看板, "src/main.rs");
        let id = 发布带实现文档任务(&看板, &["src/main.rs", "src/缺失.rs"], false);
        let 响应 = 看板清理(State(状态), Path(id)).await;
        match 响应 {
            Err((码, 消息)) => {
                assert_eq!(码, StatusCode::BAD_REQUEST);
                assert!(消息.contains("拒绝清理"), "错误信息应说明拒绝理由: {消息}");
            }
            Ok(_) => panic!("未通过应拒绝清理"),
        }
        // 未通过仍保持待清理且扫尾记录已写入
        let 板 = 看板.lock().expect("看板锁中毒");
        let 任务 = 板.查询(id).unwrap();
        assert_eq!(任务.status, TaskStatus::待清理, "未通过不得推进状态");
        assert!(!任务.扫尾记录.as_ref().map(|r| r.通过).unwrap_or(true), "应已写入不通过的扫尾记录");
    }

    /// 测试12：清理 API——未装配执行者兼容直接清理（记录 None）
    #[tokio::test]
    async fn 清理接口_未装配执行者兼容清理() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒清理_兼容_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 基础状态(&看板); // 未装配扫尾执行者
        let id = { 看板.lock().expect("看板锁中毒").发布任务(Task::新建(0, "兼容".into(), "d".into(), 100)).unwrap() };
        // 直接设状态待清理（无实现文档也可清理）
        {
            let (mut 板, uuid) = {
                let 板 = 看板.lock().expect("看板锁中毒");
                let uuid = 板.查询(id).unwrap().任务标识.任务id;
                (板, uuid)
            };
            板.按标识改写(&uuid, |t| t.status = TaskStatus::待清理).unwrap();
        }
        let 响应 = 看板清理(State(状态), Path(id)).await;
        let Ok(Json(清理结果)) = 响应 else {
            panic!("未装配执行者应兼容清理成功");
        };
        assert_eq!(清理结果.记录, None, "未装配执行者记录应为 None");
        let 板 = 看板.lock().expect("看板锁中毒");
        assert_eq!(板.查询(id).map(|t| t.status), Some(TaskStatus::清理完成));
    }

    // ===== 任务3：道祖澄清并推进（木层回退任务） =====

    /// 辅助：构造处于待道祖澄清的任务
    fn 发布待澄清任务(看板: &Arc<Mutex<TaskBoard>>) -> u64 {
        let id = { 看板.lock().expect("看板锁中毒").发布任务(Task::新建(0, "澄清测试".into(), "描述".into(), 100)).unwrap() };
        let uuid = { 看板.lock().expect("看板锁中毒").查询(id).unwrap().任务标识.任务id };
        let mut 板 = 看板.lock().expect("看板锁中毒");
        板.按标识改写(&uuid, |t| t.status = TaskStatus::待道祖澄清).unwrap();
        id
    }

    /// 测试13：澄清 API——继续=true → 待圣人设计 + 澄清记录落库
    #[tokio::test]
    async fn 澄清接口_继续进入待圣人设计() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒澄清_继续_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 基础状态(&看板);
        let id = 发布待澄清任务(&看板);
        let 响应 = 看板澄清(State(状态), Path(id), Json(澄清请求 { 结论: "需求改为X".into(), 继续: true })).await;
        let Ok(Json(结果)) = 响应 else {
            panic!("澄清应成功");
        };
        assert_eq!(结果.任务id, id);
        assert_eq!(结果.新状态, "待圣人设计");
        assert!(结果.澄清记录.继续, "记录应标记继续");
        assert_eq!(结果.澄清记录.结论, "需求改为X");
        let 板 = 看板.lock().expect("看板锁中毒");
        let 任务 = 板.查询(id).unwrap();
        assert_eq!(任务.status, TaskStatus::待圣人设计);
        assert_eq!(任务.澄清记录.as_ref().map(|c| c.结论.as_str()), Some("需求改为X"));
    }

    /// 测试14：澄清 API——继续=false → 已取消
    #[tokio::test]
    async fn 澄清接口_终止进入已取消() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒澄清_终止_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 基础状态(&看板);
        let id = 发布待澄清任务(&看板);
        let 响应 = 看板澄清(State(状态), Path(id), Json(澄清请求 { 结论: "需求不可行，终止".into(), 继续: false })).await;
        let Ok(Json(结果)) = 响应 else {
            panic!("澄清应成功");
        };
        assert_eq!(结果.新状态, "已取消");
        let 板 = 看板.lock().expect("看板锁中毒");
        assert_eq!(板.查询(id).map(|t| t.status), Some(TaskStatus::已取消));
    }

    /// 测试15：澄清 API——非待道祖澄清状态 → 400 状态不变
    #[tokio::test]
    async fn 澄清接口_非澄清状态返回400() {
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(format!(
            "{}/洪荒澄清_400_{序号}.jsonl", std::env::temp_dir().to_string_lossy()
        ))));
        let 状态 = 基础状态(&看板);
        let id = { 看板.lock().expect("看板锁中毒").发布任务(Task::新建(0, "普通".into(), "d".into(), 100)).unwrap() };
        // 任务保持待受理（默认），非待道祖澄清
        let 响应 = 看板澄清(State(状态), Path(id), Json(澄清请求 { 结论: "x".into(), 继续: true })).await;
        match 响应 {
            Err((码, 消息)) => {
                assert_eq!(码, StatusCode::BAD_REQUEST);
                assert!(消息.contains("待道祖澄清"), "应说明仅待道祖澄清可澄清: {消息}");
            }
            Ok(_) => panic!("非澄清状态应 400"),
        }
        // 状态与澄清记录均不变
        let 板 = 看板.lock().expect("看板锁中毒");
        let 任务 = 板.查询(id).unwrap();
        assert_eq!(任务.status, TaskStatus::待受理);
        assert_eq!(任务.澄清记录, None);
    }

    }