use axum::{Router, middleware::{self, Next}, extract::{Request, State}, response::{Redirect, Response}, routing::{get, post}};
use axum::http::StatusCode;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use crate::{
    数据服务状态,
    任务列表, 查询任务, 创建任务, 迭代列表, 当前版本, 记忆列表, 规则列表, 事件列表,
    图谱查询, 格位查询, 语境查询, 认知检索接口, 日志列表, 记日志,
    看板列表, 看板查询, 看板发布, 看板承接, 看板提交, 看板清理, 看板定向回退, 看板扫尾检查, 看板澄清,
    受理开发任务接口, 事件查询, 停止执行, 更新工作区, 道祖对话接口, 道祖对话流式接口, 道祖确认接口,
    看板驱动接口, 看板驱动到空闲接口, 看板驱动状态接口, 看板驱动事件接口, 看板驱动过程接口,
    看板驱动过程流接口, 看板驱动阶段流接口,
    看板驱动过程流_agui接口, 看板驱动阶段流_agui接口,
    长河接待流接口, 长河过程流接口, 长河事件查询接口,
    会话清单接口, 会话回放接口, 会话恢复接口, 会话分叉接口,
    模型状态接口, 模型列表接口, 模型选择接口,
    模型模板接口, 模型探测接口, 模型接入接口,
    智能体清单接口, 智能体绑定接口, 智能体解绑接口,
    文件清单接口, 文件内容接口,
    工作区查询, 工作区设置,
};

/// 构建 axum 路由：只读 API + 同源托管前端静态文件 + 写接口鉴权中间件
pub fn 构建路由(状态: 数据服务状态, 静态目录: String) -> Router {
    let 鉴权令牌 = 状态.鉴权令牌.clone();
    Router::new()
        .route("/", get(重定向入口))
        .route("/api/tasks", get(任务列表).post(创建任务))
        .route("/api/tasks/{id}", get(查询任务))
        .route("/api/iterations", get(迭代列表))
        .route("/api/iterations/version", get(当前版本))
        .route("/api/memories", get(记忆列表))
        .route("/api/rules", get(规则列表))
        .route("/api/events", get(事件列表))
        .route("/api/cognition/graph", get(图谱查询))
        .route("/api/cognition/cells", get(格位查询))
        .route("/api/cognition/context", get(语境查询))
        .route("/api/cognition/search", get(认知检索接口))
        .route("/api/logs", get(日志列表).post(记日志))
        .route("/api/board", get(看板列表).post(看板发布))
        .route("/api/board/{id}", get(看板查询))
        .route("/api/board/{id}/accept", post(看板承接))
        .route("/api/board/{id}/submit", post(看板提交))
        .route("/api/board/{id}/clean", post(看板清理))
        .route("/api/board/{id}/rollback", post(看板定向回退))
        .route("/api/board/{id}/sweep", post(看板扫尾检查))
        .route("/api/board/{id}/clarify", post(看板澄清))
        .route("/api/dev/agent", post(受理开发任务接口))
        .route("/api/dev/agent/stop", post(停止执行))
        .route("/api/dev/chat", post(道祖对话接口))
        .route("/api/dev/chat/stream", post(道祖对话流式接口))
        .route("/api/dev/chat/confirm", post(道祖确认接口))
        .route("/api/dev/workspace", post(更新工作区))
        .route("/api/dev/events", get(事件查询))
        .route("/api/dev/pilot", post(看板驱动接口))
        .route("/api/dev/pilot/drain", post(看板驱动到空闲接口))
        .route("/api/dev/pilot/status", get(看板驱动状态接口))
        .route("/api/dev/pilot/events", get(看板驱动事件接口))
        .route("/api/dev/pilot/process", get(看板驱动过程接口))
        .route("/api/dev/stream", get(看板驱动过程流接口))
        .route("/api/dev/stream/state", get(看板驱动阶段流接口))
        .route("/api/dev/stream/agui", get(看板驱动过程流_agui接口))
        .route("/api/dev/stream/agui/state", get(看板驱动阶段流_agui接口))
        .route("/api/river/chat", post(长河接待流接口))
        .route("/api/river/stream", get(长河过程流接口))
        .route("/api/river/events", get(长河事件查询接口))
        .route("/api/dev/sessions", get(会话清单接口))
        .route("/api/dev/sessions/{id}", get(会话回放接口))
        .route("/api/dev/sessions/{id}/resume", post(会话恢复接口))
        .route("/api/dev/sessions/{id}/fork", post(会话分叉接口))
        .route("/api/llm/status", get(模型状态接口))
        .route("/api/llm/models", get(模型列表接口))
        .route("/api/llm/select", post(模型选择接口))
        .route("/api/llm/templates", get(模型模板接口))
        .route("/api/llm/discover", post(模型探测接口))
        .route("/api/llm/connect", post(模型接入接口))
        .route("/api/llm/agents", get(智能体清单接口))
        .route("/api/llm/agent/bind", post(智能体绑定接口))
        .route("/api/llm/agent/unbind", post(智能体解绑接口))
        .route("/api/files", get(文件清单接口))
        .route("/api/files/content", get(文件内容接口))
        .route("/api/workspace", get(工作区查询).post(工作区设置))
        .fallback_service(ServeDir::new(静态目录))
        .layer(axum::extract::DefaultBodyLimit::max(512 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://127.0.0.1:8321".parse::<axum::http::HeaderValue>().unwrap(),
                    "http://localhost:8321".parse::<axum::http::HeaderValue>().unwrap(),
                    "tauri://localhost".parse::<axum::http::HeaderValue>().unwrap(),
                ])
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .layer(middleware::from_fn_with_state(鉴权令牌, 鉴权层))
        .with_state(状态)
}

/// 根路径重定向到前端入口页
async fn 重定向入口() -> Redirect {
    Redirect::permanent("/门面.html")
}

/// 写接口鉴权中间件：GET 请求放行（SSE 流端点除外）；非 GET 与 SSE 需携带 Authorization: Bearer <令牌>。
/// SSE 端点额外接受 ?token=<令牌> 查询参数（EventSource 无法携带请求头）。
async fn 鉴权层(State(令牌): State<Option<String>>, req: Request, next: Next) -> Result<Response, StatusCode> {
    let 路径 = req.uri().path();
    let 是流端点 = 路径.starts_with("/api/dev/stream") || 路径 == "/api/dev/chat/stream" || 路径 == "/api/river/stream";
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
pub fn 启动数据服务(状态: 数据服务状态, bind: String, 端口: u16, 静态目录: String) {
    if bind != "127.0.0.1" && 状态.鉴权令牌.is_none() {
        tracing::error!("安全约束：bind={bind} 非 127.0.0.1 但未设置 auth_token，拒绝启动数据服务");
        return;
    }
    std::thread::spawn(move || {
        let 路由 = 构建路由(状态, 静态目录);
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
