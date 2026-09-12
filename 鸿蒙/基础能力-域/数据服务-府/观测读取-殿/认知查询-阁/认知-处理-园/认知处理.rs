use std::sync::Arc;
use axum::{Json, extract::{Query, State}, http::StatusCode};
use serde::{Deserialize, Serialize};
use hm_cognition::{图谱, 心智地图, 过程上下文, 检索决策记录};
use hm_content_contract::工具对话器;
use crate::数据服务状态;

/// 接口错误响应体：认知等接口的统一错误载荷（不含敏感细节）
#[derive(Debug, Serialize)]
pub struct 受理错误响应 {
    pub 错误: String,
}

/// GET /api/cognition/graph：项目图谱（模块/符号/依赖，运行时真实数据）
pub async fn 图谱查询(状态: State<数据服务状态>) -> Json<图谱> {
    let 守卫 = 状态.图谱.lock().expect("引擎锁中毒");
    let 图谱 = 守卫.clone();
    drop(守卫);
    Json(图谱)
}

/// GET /api/cognition/cells：心智地图格位（运行时真实数据）
pub async fn 格位查询(状态: State<数据服务状态>) -> Json<心智地图> {
    let 守卫 = 状态.心智地图.lock().expect("引擎锁中毒");
    let 地图 = 守卫.clone();
    drop(守卫);
    Json(地图)
}

/// GET /api/cognition/context：过程上下文语境（运行时真实数据）
pub async fn 语境查询(状态: State<数据服务状态>) -> Json<过程上下文> {
    let 守卫 = 状态.语境.lock().expect("引擎锁中毒");
    let 语境 = 守卫.clone();
    drop(守卫);
    Json(语境)
}

/// 认知检索请求：q = 问题（三态认知问答）
#[derive(Debug, Deserialize)]
pub struct 认知检索请求 {
    pub q: String,
}

/// 认知检索响应：完整决策轨迹（检索源/下沉路径/候选格位）+ 答复（可选经 LLM 组装为自然语言）
#[derive(Debug, Serialize)]
pub struct 认知检索响应 {
    pub 决策: 检索决策记录,
    pub 答复: String,
}

/// GET /api/cognition/search?q=问题：三态认知问答。
/// 决策轨迹对客户端可见；答复可选经 LLM 组装（未装配 LLM 池时回退结构化答复）。
/// 认知注入未装配 → 503 fail-loud（不 500）。
pub async fn 认知检索接口(
    状态: State<数据服务状态>,
    查询: Query<认知检索请求>,
) -> Result<Json<认知检索响应>, (StatusCode, Json<受理错误响应>)> {
    let 认知 = 状态.认知注入.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(受理错误响应 { 错误: "项目认知未装配：需开启 run_dev_agent 后重启".into() }),
    ))?;
    let 对话器: Option<Arc<dyn 工具对话器>> = 状态
        .llm池
        .clone()
        .map(|池| 池 as Arc<dyn 工具对话器>);
    let (决策, 答复) = 认知.检索决策(&查询.q, 对话器);
    Ok(Json(认知检索响应 { 决策, 答复 }))
}