use axum::{Json, extract::State};
use hm_cognition::{图谱, 心智地图, 过程上下文};
use crate::数据服务状态;

/// GET /api/cognition/graph：项目图谱（模块/符号/依赖，当前为空结构）
pub async fn 图谱查询(状态: State<数据服务状态>) -> Json<图谱> {
    let 守卫 = 状态.图谱.lock().expect("引擎锁中毒");
    let 图谱 = 守卫.clone();
    drop(守卫);
    Json(图谱)
}

/// GET /api/cognition/cells：心智地图格位（当前为空结构）
pub async fn 格位查询(状态: State<数据服务状态>) -> Json<心智地图> {
    let 守卫 = 状态.心智地图.lock().expect("引擎锁中毒");
    let 地图 = 守卫.clone();
    drop(守卫);
    Json(地图)
}

/// GET /api/cognition/context：过程上下文语境（当前为空结构）
pub async fn 语境查询(状态: State<数据服务状态>) -> Json<过程上下文> {
    let 守卫 = 状态.语境.lock().expect("引擎锁中毒");
    let 语境 = 守卫.clone();
    drop(守卫);
    Json(语境)
}