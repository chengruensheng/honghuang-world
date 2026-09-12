//! 对外接口-殿/路由组装-阁/记忆-接口-园：记忆 HTTP 只读接口。
//!
//! 路由片段由启动入口收集后注入数据服务（数据服务不再反向依赖本引擎），
//! JSON 结构与原数据服务实现逐字一致，前端零漂移。

use std::sync::{Arc, Mutex};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use hm_domain_contract::记忆库契约;
use crate::Memory;

/// 记忆接口状态：记忆库契约句柄（路由片段与测试共用）
pub type 记忆接口状态 = Arc<Mutex<dyn 记忆库契约<Memory>>>;

/// 构建记忆接口路由片段：GET /api/memories
pub fn 路由片段(记忆库: 记忆接口状态) -> Router {
    Router::new()
        .route("/api/memories", get(记忆列表))
        .with_state(记忆库)
}

/// GET /api/memories：列出全部记忆
pub async fn 记忆列表(状态: State<记忆接口状态>) -> Json<Vec<Memory>> {
    let 守卫 = 状态.lock().expect("引擎锁中毒");
    let 数据: Vec<Memory> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}
