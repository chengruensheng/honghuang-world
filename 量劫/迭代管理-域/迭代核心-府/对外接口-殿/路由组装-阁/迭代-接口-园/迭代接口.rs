//! 对外接口-殿/路由组装-阁/迭代-接口-园：迭代 HTTP 只读接口。
//!
//! 路由片段由启动入口收集后注入数据服务（数据服务不再反向依赖本引擎），
//! JSON 结构与原数据服务实现逐字一致，前端零漂移。

use std::sync::{Arc, Mutex};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use hm_domain_contract::迭代日志契约;
use crate::{Iteration, Version};

/// 迭代接口状态：迭代日志契约句柄（路由片段与测试共用）
pub type 迭代接口状态 = Arc<Mutex<dyn 迭代日志契约<Iteration, Version>>>;

/// 构建迭代接口路由片段：GET /api/iterations、GET /api/iterations/version
pub fn 路由片段(日志: 迭代接口状态) -> Router {
    Router::new()
        .route("/api/iterations", get(迭代列表))
        .route("/api/iterations/version", get(当前版本))
        .with_state(日志)
}

/// GET /api/iterations：列出全部迭代
pub async fn 迭代列表(状态: State<迭代接口状态>) -> Json<Vec<Iteration>> {
    let 守卫 = 状态.lock().expect("引擎锁中毒");
    let 数据: Vec<Iteration> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}

/// GET /api/iterations/version：当前版本
pub async fn 当前版本(状态: State<迭代接口状态>) -> Json<Version> {
    let 守卫 = 状态.lock().expect("引擎锁中毒");
    let 版本 = 守卫.当前版本();
    drop(守卫);
    Json(版本)
}
