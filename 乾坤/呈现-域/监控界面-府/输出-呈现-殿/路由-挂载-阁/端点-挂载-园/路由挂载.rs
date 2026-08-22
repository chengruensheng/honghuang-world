//! 路由挂载 —— axum 路由定义 + 启动 HTTP 服务函数。
//!
//! 依据：融合蓝图-设计稿.md §4.3 路由设计、§11.4.2 Rust HTTP 服务。
//! 8 个端点：主页 + 静态资源 + 快照 + 最近事件 + SSE 直播 + 回放 + 任务索引 + 健康检查。
//! 启动函数 `启动监控(端口)` 透出到 lib 根，供命令操作府或独立 bin 调用。

use std::time::SystemTime;

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;

use crate::{
    三源就绪, 三源就绪 as 三源, 任务树视图, 健康状态, 历史回放, 取世界快照, 建拓扑, 建步骤流,
    建直播流,
};

/// 共享状态——启动时刻（毫秒），经 axum State 注入各 handler。
#[derive(Clone, Copy)]
struct 共享状态 {
    启动时刻: u64,
}

/// 当前毫秒（UNIX 纪元）。
fn 当前毫秒() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 构造监控界面路由——10 个端点（含分裂流拓扑/步骤流）。
pub fn 建路由() -> Router {
    let 状态 = 共享状态 {
        启动时刻: 当前毫秒(),
    };
    Router::new()
        .route("/", get(主页))
        .route("/static/style.css", get(样式))
        .route("/static/app.js", get(脚本))
        .route("/api/snapshot", get(快照))
        .route("/api/events/recent", get(最近事件))
        .route("/api/events/stream", get(直播))
        .route("/api/replay", get(回放))
        .route("/api/tasks", get(任务))
        .route("/api/topology", get(拓扑视图))
        .route("/api/lines/:id/steps", get(任务线步骤))
        .route("/api/health", get(健康))
        .with_state(状态)
}

/// 启动监控 HTTP 服务——透出到 lib 根，供外部调用。
///
/// 默认端口 8080；绑定失败 panic（启动期错误不应静默）。
pub async fn 启动监控(端口: u16) {
    let 监听地址 = format!("0.0.0.0:{端口}");
    rizhi_fu::info!("监控界面启动于 {监听地址}");
    let 监听器 = match tokio::net::TcpListener::bind(&监听地址).await {
        Ok(l) => l,
        Err(e) => {
            rizhi_fu::error!("监控界面绑定 {监听地址} 失败: {e}");
            return;
        }
    };
    let 路由 = 建路由();
    if let Err(e) = axum::serve(监听器, 路由).await {
        rizhi_fu::error!("监控界面服务异常: {e}");
    }
}

// ===== handlers =====

/// GET / —— 主页 HTML。
async fn 主页() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::HTML_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::主页HTML,
    )
}

/// GET /static/style.css —— CSS 样式。
async fn 样式() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::CSS_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::样式CSS,
    )
}

/// GET /static/app.js —— 前端逻辑。
async fn 脚本() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::JS_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::脚本JS,
    )
}

/// GET /api/snapshot —— 当前世界状态快照。
async fn 快照(State(状态): State<共享状态>) -> impl IntoResponse {
    let 快照 = 取世界快照(状态.启动时刻);
    Json(快照)
}

/// GET /api/events/recent?limit=200 —— 最近 N 条事件（三源合并，倒序）。
async fn 最近事件(Query(参数): Query<最近事件参数>) -> impl IntoResponse {
    let 条数 = 参数.limit.unwrap_or(200).min(1000);
    let 事件 = crate::读最近(条数);
    Json(事件)
}

#[derive(Deserialize)]
struct 最近事件参数 {
    limit: Option<usize>,
}

/// GET /api/events/stream —— SSE 直播长连接。
async fn 直播() -> impl IntoResponse {
    rizhi_fu::info!("SSE 直播连接建立");
    建直播流()
}

/// GET /api/replay?since=&until= —— 历史回放（按时间窗）。
async fn 回放(Query(参数): Query<回放参数>) -> impl IntoResponse {
    let 事件 = 历史回放(参数.since.unwrap_or(0), 参数.until.unwrap_or(0));
    Json(事件)
}

#[derive(Deserialize)]
struct 回放参数 {
    since: Option<u64>,
    until: Option<u64>,
}

/// GET /api/tasks —— 任务索引（按 _task_id 聚合）。
async fn 任务() -> impl IntoResponse {
    let 全部 = crate::读全部();
    let 索引 = 任务树视图(&全部);
    Json(索引)
}

/// GET /api/topology —— 分裂流拓扑段列表（串行/并行/汇流）。
///
/// 依据：融合蓝图-设计稿.md §13.d.6。返回 `建拓扑(&读全部())`。
async fn 拓扑视图() -> impl IntoResponse {
    let 全部 = crate::读全部();
    let 拓 = 建拓扑(&全部);
    Json(拓)
}

/// GET /api/lines/:id/steps —— 单任务线的步骤流（§13.c 形态）。
///
/// 该任务线事件 = 读全部().filter(|e| e.任务线id == id)。
async fn 任务线步骤(Path(id): Path<String>) -> impl IntoResponse {
    let 全部 = crate::读全部();
    let 该线: Vec<_> = 全部.into_iter().filter(|e| e.任务线id == id).collect();
    let 步 = 建步骤流(&该线);
    Json(步)
}

/// GET /api/health —— 健康检查。
async fn 健康(State(状态): State<共享状态>) -> impl IntoResponse {
    let 运行秒 = 当前毫秒().saturating_sub(状态.启动时刻) / 1000;
    let 就绪 = 三源就绪();
    let 状态字 = if 就绪.事件流 || 就绪.观测记录 || 就绪.识海记录 {
        "ok"
    } else {
        "degraded"
    };
    Json(健康状态 {
        状态: 状态字.to_string(),
        运行秒,
        三源就绪: 就绪,
    })
}

// 抑制未使用警告——三源类型仅在健康检查中通过 三源就绪() 间接使用
#[allow(dead_code)]
fn _三源类型引用() -> 三源 {
    三源 {
        事件流: false,
        观测记录: false,
        识海记录: false,
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 路由构造不panic() {
        let _路由 = 建路由();
    }

    #[test]
    fn 当前毫秒非零() {
        let ms = 当前毫秒();
        assert!(ms > 0);
    }
}
