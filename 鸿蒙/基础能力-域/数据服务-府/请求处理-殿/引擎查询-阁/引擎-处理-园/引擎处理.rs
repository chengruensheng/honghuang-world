use axum::{Json, extract::{State, Path}, http::StatusCode};
use serde::Deserialize;
use crate::数据服务状态;
use tc_task::Task;
use lj_iteration::{Iteration, Version};
use qk_memory::Memory;
use dy_rule::Rule;
use hd_event::Event;

/// 创建任务请求体
#[derive(Debug, Deserialize)]
pub struct 创建任务请求 {
    pub 标题: String,
    pub 描述: String,
}

/// GET /api/tasks：列出全部任务
pub async fn 任务列表(状态: State<数据服务状态>) -> Json<Vec<Task>> {
    let 守卫 = 状态.任务仓库.lock().expect("引擎锁中毒");
    let 数据: Vec<Task> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}

/// GET /api/tasks/{id}：按 id 查询任务，不存在返回 404
pub async fn 查询任务(状态: State<数据服务状态>, Path(id): Path<u64>) -> Result<Json<Task>, StatusCode> {
    let 守卫 = 状态.任务仓库.lock().expect("引擎锁中毒");
    match 守卫.查询(id) {
        Some(任务) => Ok(Json(任务.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// POST /api/tasks：创建任务，返回新任务 id
pub async fn 创建任务(状态: State<数据服务状态>, Json(请求): Json<创建任务请求>) -> Result<Json<u64>, StatusCode> {
    let mut 守卫 = 状态.任务仓库.lock().expect("引擎锁中毒");
    守卫.创建(请求.标题, 请求.描述).map(Json).map_err(|e| {
        tracing::warn!("创建任务失败: {e}");
        StatusCode::BAD_REQUEST
    })
}

/// GET /api/iterations：列出全部迭代
pub async fn 迭代列表(状态: State<数据服务状态>) -> Json<Vec<Iteration>> {
    let 守卫 = 状态.迭代日志.lock().expect("引擎锁中毒");
    let 数据: Vec<Iteration> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}

/// GET /api/iterations/version：当前版本
pub async fn 当前版本(状态: State<数据服务状态>) -> Json<Version> {
    let 守卫 = 状态.迭代日志.lock().expect("引擎锁中毒");
    let 版本 = 守卫.当前版本();
    drop(守卫);
    Json(版本)
}

/// GET /api/memories：列出全部记忆
pub async fn 记忆列表(状态: State<数据服务状态>) -> Json<Vec<Memory>> {
    let 守卫 = 状态.记忆库.lock().expect("引擎锁中毒");
    let 数据: Vec<Memory> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}

/// GET /api/rules：列出全部规则
pub async fn 规则列表(状态: State<数据服务状态>) -> Json<Vec<Rule>> {
    let 守卫 = 状态.规则库.lock().expect("引擎锁中毒");
    let 数据: Vec<Rule> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}

/// GET /api/events：列出全部事件
pub async fn 事件列表(状态: State<数据服务状态>) -> Json<Vec<Event>> {
    let 守卫 = 状态.事件总线.lock().expect("引擎锁中毒");
    let 数据: Vec<Event> = 守卫.全部().into_iter().cloned().collect();
    drop(守卫);
    Json(数据)
}