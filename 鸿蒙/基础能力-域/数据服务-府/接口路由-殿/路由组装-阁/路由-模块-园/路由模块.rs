use axum::{Router, middleware::{self, Next}, extract::{Request, State}, response::Response, routing::{get, post}};
use axum::http::StatusCode;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use hm_config::对外配置;
use crate::{
    数据服务状态,
    图谱查询, 格位查询, 语境查询, 认知检索接口, 日志列表, 记日志,
    模型状态接口, 模型列表接口, 模型选择接口,
    模型模板接口, 模型探测接口, 模型接入接口,
    智能体清单接口, 智能体绑定接口, 智能体解绑接口,
};

/// 构建 axum 路由：内置接口（认知/日志/LLM）+ 各拥有方注册的路由片段 + 写接口鉴权中间件。
///
/// 依赖方向：各府（五引擎府、hm-agent）提供自己的 `Router` 片段，本函数仅作注册与合并，
/// 基础能力-域 不再反向依赖系统装配-域。对外呈现面（CORS 来源 / 静态托管）由「对外契约」
/// 文件声明，前端来去不改后端代码。
pub fn 构建路由(状态: 数据服务状态, 对外: 对外配置, 片段: Vec<Router>) -> Router {
    let 鉴权令牌 = 状态.鉴权令牌.clone();
    let 自身 = Router::new()
        .route("/api/cognition/graph", get(图谱查询))
        .route("/api/cognition/cells", get(格位查询))
        .route("/api/cognition/context", get(语境查询))
        .route("/api/cognition/search", get(认知检索接口))
        .route("/api/logs", get(日志列表).post(记日志))
        .route("/api/llm/status", get(模型状态接口))
        .route("/api/llm/models", get(模型列表接口))
        .route("/api/llm/select", post(模型选择接口))
        .route("/api/llm/templates", get(模型模板接口))
        .route("/api/llm/discover", post(模型探测接口))
        .route("/api/llm/connect", post(模型接入接口))
        .route("/api/llm/agents", get(智能体清单接口))
        .route("/api/llm/agent/bind", post(智能体绑定接口))
        .route("/api/llm/agent/unbind", post(智能体解绑接口))
        .with_state(状态);

    // 注册各拥有方路由片段（五引擎只读 API、看板/开发端接口）
    let mut 应用 = 片段.into_iter().fold(自身, |应用, 片段| 应用.merge(片段));

    // 静态托管：仅当对外契约声明目录时挂载（空 = 纯 API，零前端依赖）
    if !对外.static_dir.trim().is_empty() {
        应用 = 应用.fallback_service(ServeDir::new(对外.static_dir.clone()));
    }

    应用 = 应用.layer(axum::extract::DefaultBodyLimit::max(512 * 1024));

    // 跨源：仅当对外契约声明来源白名单时启用（空 = 不启用 CORS 中间件）
    if !对外.cors_origins.is_empty() {
        let 来源: Vec<axum::http::HeaderValue> = 对外
            .cors_origins
            .iter()
            .filter_map(|来源| 来源.parse().ok())
            .collect();
        if !来源.is_empty() {
            应用 = 应用.layer(
                CorsLayer::new()
                    .allow_origin(来源)
                    .allow_methods(tower_http::cors::Any)
                    .allow_headers(tower_http::cors::Any),
            );
        }
    }

    应用.layer(middleware::from_fn_with_state(鉴权令牌, 鉴权层))
}

/// 写接口鉴权中间件：GET 请求放行（SSE 流端点除外）；非 GET 与 SSE 需携带 Authorization: Bearer <令牌>。
/// SSE 端点额外接受 ?token=<令牌> 查询参数（EventSource 无法携带请求头）。
async fn 鉴权层(State(令牌): State<Option<String>>, req: Request, next: Next) -> Result<Response, StatusCode> {
    let 路径 = req.uri().path();
    let 是流端点 = 路径.starts_with("/api/dev/stream") || 路径 == "/api/dev/chat/stream";
    if req.method() == axum::http::Method::GET && !是流端点 {
        return Ok(next.run(req).await);
    }
    match 令牌 {
        Some(t) => {
            let 头令牌 = req.headers().get("Authorization").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
            let 查令牌 = req.uri().query().and_then(|q| {
                q.split('&').find_map(|对| 对.strip_prefix("token=").map(|v| v.to_string()))
            });
            let 匹配 = 头令牌.as_deref() == Some(format!("Bearer {t}").as_str())
                || 查令牌.as_deref() == Some(t.as_str());
            if 匹配 {
                Ok(next.run(req).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        None => Ok(next.run(req).await),
    }
}

/// 启动数据服务：独立线程运行 HTTP 服务，失败仅告警不影响主程序。
///
/// 安全约束：bind 非 127.0.0.1 时必须设置 auth_token，否则启动失败（fail-loud）。
pub fn 启动数据服务(状态: 数据服务状态, 片段: Vec<Router>, bind: String, 端口: u16, 对外: 对外配置) {
    if bind != "127.0.0.1" && 状态.鉴权令牌.is_none() {
        tracing::error!("安全约束：bind={bind} 非 127.0.0.1 但未设置 auth_token，拒绝启动数据服务");
        return;
    }
    std::thread::spawn(move || {
        let 路由 = 构建路由(状态, 对外, 片段);
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::warn!("数据服务运行时创建失败: {e}");
                return;
            }
        };
        runtime.block_on(async {
            let 监听 = format!("{bind}:{端口}");
            match tokio::net::TcpListener::bind(&监听).await {
                Ok(listener) => {
                    tracing::info!("数据服务已启动: http://{bind}:{端口}");
                    if let Err(e) = axum::serve(listener, 路由).await {
                        tracing::warn!("数据服务运行失败: {e}");
                    }
                }
                Err(e) => tracing::warn!("数据服务绑定 {bind}:{端口} 失败: {e}"),
            }
        });
    });
}
