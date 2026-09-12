#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use axum::body::Body;
    use axum::http::{header, Method, Request, StatusCode};
    use axum::Router;
    use hm_agent::{开发服务状态, 开发执行台, 看板驱动台};
    use hm_cognition::{图谱, 心智地图, 过程上下文};
    use hm_config::对外配置;
    use hm_domain_contract::{任务仓库契约, 记忆库契约};
    use hm_http::{构建路由, 数据服务状态};
    use hm_log::运行日志记录器;
    use http_body_util::BodyExt;
    use qk_memory::{Memory, MemoryStore};
    use tc_task::{Task, TaskStatus, TaskStore};
    use tower::ServiceExt;

    static 用例序号: AtomicU64 = AtomicU64::new(0);

    /// 构造最小可用的 数据服务状态（hm-http）：认知三态 + 日志记录器 + LLM 池 + 鉴权令牌
    fn 构造数据状态() -> 数据服务状态 {
        let 图谱 = Arc::new(Mutex::new(图谱::新()));
        let 心智地图 = Arc::new(Mutex::new(心智地图::新()));
        let 语境 = Arc::new(Mutex::new(过程上下文::新()));
        let 日志记录器 = Arc::new(Mutex::new(运行日志记录器::new()));
        数据服务状态::新(图谱, 心智地图, 语境, 日志记录器, None, None)
    }

    /// 构造开发服务状态（hm-agent）：任务看板 + 开发执行台 + 看板驱动台 + 记忆库
    fn 构造开发状态() -> 开发服务状态<Memory> {
        let 记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>> = Arc::new(Mutex::new(MemoryStore::new()));
        let 序号 = 用例序号.fetch_add(1, Ordering::SeqCst);
        let 任务看板 = Arc::new(Mutex::new(tc_task::TaskBoard::新建(
            std::env::temp_dir()
                .join(format!("洪荒对外契约测试看板_{序号}.jsonl"))
                .to_string_lossy()
                .to_string(),
        )));
        开发服务状态::新(
            任务看板,
            Arc::new(开发执行台::新()),
            Arc::new(看板驱动台::新()),
            记忆库,
        )
    }

    /// 构造被测路由：任务片段由任务核心府提供（数据服务仅作合并与中间件）
    fn 构造路由(对外: 对外配置) -> Router {
        let 仓库: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>> = Arc::new(Mutex::new(TaskStore::new()));
        构建路由(构造数据状态(), 对外, vec![tc_task::路由片段(仓库)])
    }

    /// 构造一个 GET 请求（可带请求头）
    fn 构造请求(路径: &str, 来源: Option<&str>) -> Request<Body> {
        let mut 构造器 = Request::builder().method(Method::GET).uri(路径);
        if let Some(来源) = 来源 {
            构造器 = 构造器.header(header::ORIGIN, 来源);
        }
        构造器.body(Body::empty()).expect("构造请求应成功")
    }

    /// 读取响应体为字符串
    async fn 读体(响应: axum::response::Response) -> String {
        let 字节 = 响应.into_body().collect().await.expect("读响应体应成功").to_bytes();
        String::from_utf8_lossy(&字节).into_owned()
    }

    /// 1. 静态目录为空 ⇒ 纯 API：未知路径 404（未托管）
    #[tokio::test]
    async fn 对外契约_静态目录为空纯api未知路径404() {
        let 路由 = 构造路由(对外配置::default());
        let 响应 = 路由.oneshot(构造请求("/nope", None)).await.expect("请求应送达");
        assert_eq!(响应.status(), StatusCode::NOT_FOUND, "未声明静态目录时未知路径应 404");
    }

    /// 2. 声明静态目录 ⇒ 未知路径由静态服务托管返回文件（200 且含文件内容）
    #[tokio::test]
    async fn 对外契约_声明静态目录则托管未知路径() {
        let 序号 = 用例序号.fetch_add(1, Ordering::SeqCst);
        let 目录 = std::env::temp_dir().join(format!("洪荒对外契约静态_{序号}"));
        std::fs::create_dir_all(&目录).expect("创建静态目录");
        std::fs::write(目录.join("index.html"), "对外契约静态页标记").expect("写首页");

        let 对外 = 对外配置 {
            static_dir: 目录.to_string_lossy().into_owned(),
            ..对外配置::default()
        };
        // “/” 不是任何 API 路由，属未知路径，应回落到静态托管并返回 index.html
        let 路由 = 构造路由(对外);
        let 响应 = 路由.oneshot(构造请求("/", None)).await.expect("请求应送达");
        assert_eq!(响应.status(), StatusCode::OK, "声明静态目录后未知路径应由静态服务返回");
        let 体 = 读体(响应).await;
        assert!(体.contains("对外契约静态页标记"), "响应体应含静态文件内容，实际: {体}");

        let _ = std::fs::remove_dir_all(&目录);
    }

    /// 3. 来源白名单为空 ⇒ 不启用 CORS，响应无 access-control-allow-origin
    #[tokio::test]
    async fn 对外契约_来源白名单为空无跨源头() {
        let 路由 = 构造路由(对外配置::default());
        let 响应 = 路由
            .oneshot(构造请求("/api/tasks", Some("http://localhost:1234")))
            .await
            .expect("请求应送达");
        assert!(
            响应.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none(),
            "空来源不应启用 CORS 中间件"
        );
    }

    /// 4. 来源白名单命中 ⇒ 响应回带对应 access-control-allow-origin
    #[tokio::test]
    async fn 对外契约_来源白名单命中回带跨源头() {
        let 对外 = 对外配置 {
            cors_origins: vec!["http://localhost:1234".to_string()],
            ..对外配置::default()
        };
        let 路由 = 构造路由(对外);
        let 响应 = 路由
            .oneshot(构造请求("/api/tasks", Some("http://localhost:1234")))
            .await
            .expect("请求应送达");
        let 回带 = 响应
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|值| 值.to_str().ok());
        assert_eq!(回带, Some("http://localhost:1234"), "白名单命中应回带该来源");
    }

    /// 5. 并发上限：设 10 ⇒ 10 许可；设 0 ⇒ 不限制（取信号量最大许可数）
    ///
    /// 注：`Semaphore::new(usize::MAX)` 会 panic（tokio 上限为 `usize::MAX >> 3`），
    /// 故「不限制」以 `Semaphore::MAX_PERMITS` 表达，可用许可数即该上限。
    #[test]
    fn 对外契约_并发上限零表示不限制() {
        assert_eq!(构造开发状态().设置并发上限(10).sse信号量.available_permits(), 10);

        let 不限 = 构造开发状态().设置并发上限(0);
        assert_eq!(
            不限.sse信号量.available_permits(),
            tokio::sync::Semaphore::MAX_PERMITS,
            "上限 0 应转为不限制（信号量最大许可数）"
        );
    }
}
