use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use crate::{数据服务状态, 看板驱动状态, 受理错误响应};

/// 驱动受理响应体
#[derive(Debug, Serialize)]
pub struct 驱动受理响应 {
    pub 受理: bool,
}

/// POST /api/dev/pilot：驱动看板一轮。
///
/// 后台线程执行（LLM 调用耗时秒级），HTTP 立即返回受理；
/// 未装配 → 503 fail-loud；已有驱动运行中 → 409。
pub async fn 看板驱动接口(
    状态: State<数据服务状态>,
) -> Result<Json<驱动受理响应>, (StatusCode, Json<受理错误响应>)> {
    let 台 = &状态.看板驱动台;
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
    台.启动执行一轮();
    Ok(Json(驱动受理响应 { 受理: true }))
}

/// GET /api/dev/pilot/status：看板驱动台状态（就绪/运行中/最近阶段/最近结果）
pub async fn 看板驱动状态接口(状态: State<数据服务状态>) -> Json<看板驱动状态> {
    Json(状态.看板驱动台.当前状态())
}
