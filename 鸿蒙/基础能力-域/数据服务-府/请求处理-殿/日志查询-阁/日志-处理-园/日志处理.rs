use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use hm_log::日志记录;
use crate::数据服务状态;

/// 记日志请求体
#[derive(Debug, Deserialize)]
pub struct 记日志请求 {
    pub 标签: String,
    pub 样式: String,
    pub 内容: String,
}

/// GET /api/logs：列出全部运行日志
pub async fn 日志列表(状态: State<数据服务状态>) -> Json<Vec<日志记录>> {
    let 守卫 = 状态.日志记录器.lock().expect("日志锁中毒");
    let 数据 = 守卫.全部();
    drop(守卫);
    Json(数据)
}

/// POST /api/logs：追加一条运行日志，成功返回 204
pub async fn 记日志(状态: State<数据服务状态>, Json(请求): Json<记日志请求>) -> StatusCode {
    let mut 守卫 = 状态.日志记录器.lock().expect("日志锁中毒");
    守卫.记日志(请求.标签, 请求.样式, 请求.内容);
    StatusCode::NO_CONTENT
}