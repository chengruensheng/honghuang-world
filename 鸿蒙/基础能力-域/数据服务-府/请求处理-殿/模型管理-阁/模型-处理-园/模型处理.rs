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


/// LLM 模板响应：内置热门供应商模板清单（无密钥）
#[derive(Debug, Serialize)]
pub struct LLM模板响应 {
    pub 模板: Vec<hm_content::模板>,
}

/// 模型探测请求体（dsh「获取可用模型」语义：目录命中零网络 / 自定义走端点）
#[derive(Deserialize)]
pub struct LLM探测请求 {
    pub 供应商: Option<String>,
    pub 地址: Option<String>,
    pub 密钥: Option<String>,
}

/// 模型探测响应：来源（目录/网络）+ 模型 + 错误（客户端展示，不 panic）
#[derive(Debug, Serialize)]
pub struct LLM探测响应 {
    pub 来源: String,
    pub 模型: Vec<hm_content::模型条目>,
    pub 错误: Option<String>,
}

/// 供应商接入请求体（名称/地址/模型必填；密钥支持 env: 引用或明文——明文仅会话内）
#[derive(Deserialize)]
pub struct LLM接入请求 {
    pub 名称: String,
    pub 地址: String,
    pub 密钥: String,
    pub 模型: String,
}

#[derive(Debug, Serialize)]
pub struct LLM接入响应 {
    pub 成功: bool,
    pub 选择: Option<池选择响应>,
    pub 配置片段: Option<String>,
}

/// GET /api/llm/templates：内置热门供应商模板（预填地址 + 默认模型，无密钥）
pub async fn 模型模板接口() -> Json<LLM模板响应> {
    Json(LLM模板响应 { 模板: hm_content::热门模板() })
}

/// POST /api/llm/discover：目录命中 → 模板默认模型（零网络）；
/// 否则网络探测（密钥三级：请求内 > 池内已存 > 无鉴权），失败返回错误字段供客户端展示。
pub async fn 模型探测接口(
    State(状态): State<数据服务状态>,
    Json(请求): Json<LLM探测请求>,
) -> Json<LLM探测响应> {
    let 供应商 = 请求.供应商.as_deref();
    let 地址 = 请求.地址.as_deref();
    let 密钥 = 请求.密钥.as_deref();
    match &状态.llm池 {
        Some(池) => match 池.探测模型(供应商, 地址, 密钥) {
            Ok((来源, 模型)) => Json(LLM探测响应 { 来源, 模型, 错误: None }),
            Err(e) => Json(LLM探测响应 { 来源: String::new(), 模型: vec![], 错误: Some(e.to_string()) }),
        },
        None => {
            // 无池：目录命中仍可回答（零网络）；网络探测仅用请求内密钥
            if let Some(名) = 供应商 {
                if let Some(模型) = hm_content::模板模型(名) {
                    return Json(LLM探测响应 { 来源: "目录".into(), 模型, 错误: None });
                }
            }
            match 地址.filter(|a| !a.is_empty()) {
                Some(地址值) => {
                    // 端点归一化：地址可能是完整 chat 端点（与池内规则一致）
                    let 根 = 地址值.trim_end_matches('/').trim_end_matches("/chat/completions");
                    match hm_content::网络探测模型(&format!("{根}/models"), 密钥) {
                        Ok((来源, 模型)) => Json(LLM探测响应 { 来源, 模型, 错误: None }),
                        Err(e) => Json(LLM探测响应 { 来源: String::new(), 模型: vec![], 错误: Some(e.to_string()) }),
                    }
                }
                None => Json(LLM探测响应 {
                    来源: String::new(),
                    模型: vec![],
                    错误: Some("模型探测需提供 base_url 或选择内置模板".into()),
                }),
            }
        }
    }
}

/// POST /api/llm/connect：接入供应商（注册进池 + 选中 + env 引用落盘 + 返回 TOML 片段）
pub async fn 模型接入接口(
    State(状态): State<数据服务状态>,
    Json(请求): Json<LLM接入请求>,
) -> Result<Json<LLM接入响应>, StatusCode> {
    let Some(池) = &状态.llm池 else {
        return Err(StatusCode::NOT_FOUND);
    };
    match 池.接入(&请求.名称, &请求.地址, &请求.密钥, &请求.模型) {
        Ok((选择, 片段)) => Ok(Json(LLM接入响应 {
            成功: true,
            选择: Some(池选择响应 { 供应商: 选择.供应商, 模型: 选择.模型 }),
            配置片段: Some(片段),
        })),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// 智能体绑定响应：身份 + 当前独立模型绑定（None = 跟随全局选择）
#[derive(Debug, Serialize)]
pub struct 智能体绑定响应 {
    pub 名: String,
    pub 绑定: Option<池选择响应>,
}

/// LLM 智能体清单响应：可绑定身份 × 当前绑定
#[derive(Debug, Serialize)]
pub struct LLM智能体响应 {
    pub 智能体: Vec<智能体绑定响应>,
}

/// 绑定请求体（智能体身份须在清单内；供应商须在池内）
#[derive(Deserialize)]
pub struct LLM绑定请求 {
    pub 智能体: String,
    pub 供应商: String,
    pub 模型: String,
}

/// 解绑请求体
#[derive(Deserialize)]
pub struct LLM解绑请求 {
    pub 智能体: String,
}

#[derive(Debug, Serialize)]
pub struct LLM绑定操作响应 {
    pub 成功: bool,
}

/// GET /api/llm/agents：智能体身份清单 + 当前独立模型绑定
pub async fn 智能体清单接口(State(状态): State<数据服务状态>) -> Json<LLM智能体响应> {
    match &状态.llm池 {
        Some(池) => Json(LLM智能体响应 {
            智能体: 池
                .绑定清单()
                .into_iter()
                .map(|b| 智能体绑定响应 {
                    名: b.名,
                    绑定: b.绑定.map(|s| 池选择响应 { 供应商: s.供应商, 模型: s.模型 }),
                })
                .collect(),
        }),
        None => Json(LLM智能体响应 { 智能体: vec![] }),
    }
}

/// POST /api/llm/agent/bind：为智能体绑定独立模型（校验身份与供应商；成功落盘绑定文件）
pub async fn 智能体绑定接口(
    State(状态): State<数据服务状态>,
    Json(请求): Json<LLM绑定请求>,
) -> Result<Json<LLM绑定操作响应>, StatusCode> {
    let Some(池) = &状态.llm池 else {
        return Err(StatusCode::NOT_FOUND);
    };
    if !池.可用() {
        return Err(StatusCode::NOT_FOUND);
    }
    池.绑定智能体(&请求.智能体, &请求.供应商, &请求.模型)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(LLM绑定操作响应 { 成功: true }))
}

/// POST /api/llm/agent/unbind：解除智能体绑定（回退全局选择，幂等）
pub async fn 智能体解绑接口(
    State(状态): State<数据服务状态>,
    Json(请求): Json<LLM解绑请求>,
) -> Result<Json<LLM绑定操作响应>, StatusCode> {
    let Some(池) = &状态.llm池 else {
        return Err(StatusCode::NOT_FOUND);
    };
    if !池.可用() {
        return Err(StatusCode::NOT_FOUND);
    }
    池.解绑智能体(&请求.智能体).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(LLM绑定操作响应 { 成功: true }))
}
