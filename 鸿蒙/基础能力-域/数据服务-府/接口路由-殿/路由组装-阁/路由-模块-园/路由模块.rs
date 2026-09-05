use axum::{Router, response::Redirect, routing::{get, post}};
use tower_http::services::ServeDir;
use crate::{
    数据服务状态,
    任务列表, 查询任务, 创建任务, 迭代列表, 当前版本, 记忆列表, 规则列表, 事件列表,
    图谱查询, 格位查询, 语境查询, 日志列表, 记日志,
    受理开发任务接口, 事件查询, 停止执行,
};

/// 构建 axum 路由：只读 API + 同源托管前端静态文件
pub fn 构建路由(状态: 数据服务状态, 静态目录: String) -> Router {
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
        .route("/api/logs", get(日志列表).post(记日志))
        .route("/api/dev/agent", post(受理开发任务接口))
        .route("/api/dev/agent/stop", post(停止执行))
        .route("/api/dev/events", get(事件查询))
        .fallback_service(ServeDir::new(静态目录))
        .with_state(状态)
}

/// 根路径重定向到前端入口页
async fn 重定向入口() -> Redirect {
    Redirect::permanent("/入口.html")
}

/// 启动数据服务：独立线程运行 HTTP 服务，失败仅告警不影响主程序
pub fn 启动数据服务(状态: 数据服务状态, 端口: u16, 静态目录: String) {
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
            let 监听 = format!("0.0.0.0:{端口}");
            match tokio::net::TcpListener::bind(&监听).await {
                Ok(listener) => {
                    tracing::info!("数据服务已启动: http://localhost:{端口}");
                    if let Err(e) = axum::serve(listener, 路由).await {
                        tracing::warn!("数据服务运行失败: {e}");
                    }
                }
                Err(e) => tracing::warn!("数据服务绑定端口 {端口} 失败: {e}"),
            }
        });
    });
}