//! 对外接口-殿/路由组装-阁/任务-接口-园：任务 HTTP 只读 + 创建接口。
//!
//! 路由片段由启动入口收集后注入数据服务（数据服务不再反向依赖本引擎），
//! JSON 结构与原数据服务实现逐字一致，前端零漂移。

use std::sync::{Arc, Mutex};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use hm_domain_contract::任务仓库契约;
use crate::{Task, TaskStatus};

/// 创建任务请求体
#[derive(Debug, Deserialize)]
pub struct 创建任务请求 {
    pub 标题: String,
    pub 描述: String,
}

/// 任务接口状态：任务仓库契约句柄（路由片段与测试共用）
pub type 任务接口状态 = Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>>;

/// 构建任务接口路由片段：GET /api/tasks、GET /api/tasks/{id}、POST /api/tasks
pub fn 路由片段(仓库: 任务接口状态) -> Router {
    Router::new()
        .route("/api/tasks", get(任务列表).post(创建任务))
        .route("/api/tasks/{id}", get(查询任务))
        .with_state(仓库)
}

/// GET /api/tasks：列出全部任务
pub async fn 任务列表(状态: State<任务接口状态>) -> Json<Vec<Task>> {
    let 守卫 = 状态.lock().expect("引擎锁中毒");
    let 数据: Vec<Task> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}

/// GET /api/tasks/{id}：按 id 查询任务，不存在返回 404
pub async fn 查询任务(状态: State<任务接口状态>, Path(id): Path<u64>) -> Result<Json<Task>, StatusCode> {
    let 守卫 = 状态.lock().expect("引擎锁中毒");
    match 守卫.查询(id) {
        Some(任务) => Ok(Json(任务.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// POST /api/tasks：创建任务，返回新任务 id
pub async fn 创建任务(
    状态: State<任务接口状态>,
    Json(请求): Json<创建任务请求>,
) -> Result<Json<u64>, StatusCode> {
    let mut 守卫 = 状态.lock().expect("引擎锁中毒");
    守卫.创建(请求.标题, 请求.描述).map(Json).map_err(|e| {
        tracing::warn!("创建任务失败: {e}");
        StatusCode::BAD_REQUEST
    })
}
