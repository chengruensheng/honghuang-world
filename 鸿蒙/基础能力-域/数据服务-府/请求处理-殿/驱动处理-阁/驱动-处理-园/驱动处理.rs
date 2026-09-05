use axum::{Json, extract::{Query, State}, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::{数据服务状态, 看板驱动状态, 事件游标, 受理错误响应};

/// 驱动受理响应体
#[derive(Debug, Serialize)]
pub struct 驱动受理响应 {
    pub 受理: bool,
}

/// 驱动到空闲请求体（上限缺省 10 轮）
#[derive(Debug, Deserialize)]
pub struct 驱动到空闲请求 {
    #[serde(default)]
    pub 上限: Option<usize>,
}

/// 驱动通道检查与预占（未装配 503 / 运行中 409）
fn 检查并预占(台: &crate::看板驱动台) -> Result<(), (StatusCode, Json<受理错误响应>)> {
    if !台.就绪() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(受理错误响应 { 错误: "看板驱动未就绪：需配置 LLM_API_KEY 并开启 run_dev_agent 后重启".into() }),
        ));
    }
    if !台.预留() {
        return Err((
            StatusCode::CONFLICT,
            Json(受理错误响应 { 错误: "已有驱动执行中，请等待完成后再驱动".into() }),
        ));
    }
    Ok(())
}

/// POST /api/dev/pilot：驱动看板一轮。
///
/// 后台线程执行（LLM 调用耗时秒级），HTTP 立即返回受理；
/// 未装配 → 503 fail-loud；已有驱动运行中 → 409。
pub async fn 看板驱动接口(
    状态: State<数据服务状态>,
) -> Result<Json<驱动受理响应>, (StatusCode, Json<受理错误响应>)> {
    let 台 = &状态.看板驱动台;
    检查并预占(台)?;
    台.启动执行一轮();
    Ok(Json(驱动受理响应 { 受理: true }))
}

/// POST /api/dev/pilot/drain：驱动看板到空闲。
///
/// 循环执行一轮直到 无可驱动任务 / 达到上限 / 出错；已推进轮次结果保留；
/// 未装配 → 503；运行中 → 409。
pub async fn 看板驱动到空闲接口(
    状态: State<数据服务状态>,
    Json(请求): Json<驱动到空闲请求>,
) -> Result<Json<驱动受理响应>, (StatusCode, Json<受理错误响应>)> {
    let 台 = &状态.看板驱动台;
    检查并预占(台)?;
    let 上限 = 请求.上限.unwrap_or(10);
    台.启动执行到空闲(上限);
    Ok(Json(驱动受理响应 { 受理: true }))
}

/// 驱动事件响应体（对齐开发事件响应结构）
#[derive(Debug, Serialize)]
pub struct 驱动事件响应 {
    pub 就绪: bool,
    pub 运行中: bool,
    pub 最近结果: Option<String>,
    pub 事件: Vec<crate::驱动阶段事件>,
}

/// GET /api/dev/pilot/events?since=N：驱动阶段事件增量（游标=上次末条序号）
pub async fn 看板驱动事件接口(
    状态: State<数据服务状态>,
    Query(游标): Query<事件游标>,
) -> Json<驱动事件响应> {
    let 台 = &状态.看板驱动台;
    Json(驱动事件响应 {
        就绪: 台.就绪(),
        运行中: 台.运行中(),
        最近结果: 台.当前状态().最近结果,
        事件: 台.驱动事件增量(游标.since.unwrap_or(0)),
    })
}

/// GET /api/dev/pilot/status：看板驱动台状态（就绪/运行中/最近阶段/最近结果）
pub async fn 看板驱动状态接口(状态: State<数据服务状态>) -> Json<看板驱动状态> {
    Json(状态.看板驱动台.当前状态())
}
