//! 对外接口-殿/路由组装-阁/事件-接口-园：事件 HTTP 只读接口。
//!
//! 路由片段由启动入口收集后注入数据服务（数据服务不再反向依赖本引擎），
//! JSON 结构与原数据服务实现逐字一致，前端零漂移。

use std::sync::{Arc, Mutex};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use hm_domain_contract::事件总线契约;
use crate::Event;

/// 事件接口状态：事件总线契约句柄（路由片段与测试共用）
pub type 事件接口状态 = Arc<Mutex<dyn 事件总线契约<Event>>>;

/// 构建事件接口路由片段：GET /api/events
pub fn 路由片段(事件总线: 事件接口状态) -> Router {
    Router::new()
        .route("/api/events", get(事件列表))
        .with_state(事件总线)
}

/// GET /api/events：列出全部事件
pub async fn 事件列表(状态: State<事件接口状态>) -> Json<Vec<Event>> {
    let 守卫 = 状态.lock().expect("引擎锁中毒");
    let 数据: Vec<Event> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}
