use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use crate::数据服务状态;

/// LLM 池状态响应：配置与否 + 当前选择 + 供应商清单（脱敏：绝不含 api_key）
#[derive(Debug, Serialize)]
pub struct LLM状态响应 {
    pub 配置: bool,
    pub 当前选择: Option<池选择响应>,
    pub 供应商: Vec<hm_content::供应商信息>,
}

#[derive(Debug, Clone, Serialize)]
pub struct 池选择响应 {
    pub 供应商: String,
    pub 模型: String,
}

/// LLM 模型发现响应：聚合各供应商 list-models 结果（失败供应商标注错误）
#[derive(Debug, Serialize)]
pub struct LLM模型响应 {
    pub 配置: bool,
    pub 结果: Vec<hm_content::模型发现>,
}

/// 模型选择请求体
#[derive(Deserialize)]
pub struct LLM选择请求 {
    pub 供应商: String,
    pub 模型: String,
}

#[derive(Debug, Serialize)]
pub struct LLM选择响应 {
    pub 成功: bool,
}

/// GET /api/llm/status：当前选择 + 供应商清单
pub async fn 模型状态接口(State(状态): State<数据服务状态>) -> Json<LLM状态响应> {
    match &状态.llm池 {
        Some(池) => Json(LLM状态响应 {
            配置: 池.可用(),
            当前选择: 池.当前选择().map(|s| 池选择响应 { 供应商: s.供应商, 模型: s.模型 }),
            供应商: 池.供应商清单(),
        }),
        None => Json(LLM状态响应 { 配置: false, 当前选择: None, 供应商: vec![] }),
    }
}

/// GET /api/llm/models：拉取各供应商可用模型（聚合，失败供应商标注错误）
pub async fn 模型列表接口(State(状态): State<数据服务状态>) -> Json<LLM模型响应> {
    match &状态.llm池 {
        Some(池) => Json(LLM模型响应 { 配置: 池.可用(), 结果: 池.列表模型() }),
        None => Json(LLM模型响应 { 配置: false, 结果: vec![] }),
    }
}

/// POST /api/llm/select：切换供应商 + 模型（校验存在；成功落盘选择文件）
pub async fn 模型选择接口(
    State(状态): State<数据服务状态>,
    Json(请求): Json<LLM选择请求>,
) -> Result<Json<LLM选择响应>, StatusCode> {
    let Some(池) = &状态.llm池 else {
        return Err(StatusCode::NOT_FOUND);
    };
    if !池.可用() {
        return Err(StatusCode::NOT_FOUND);
    }
    池.选择(&请求.供应商, &请求.模型)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(LLM选择响应 { 成功: true }))
}
