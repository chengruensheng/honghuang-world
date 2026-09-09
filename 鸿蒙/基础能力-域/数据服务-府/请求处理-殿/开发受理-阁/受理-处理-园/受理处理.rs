use axum::{Json, extract::{Query, State}, http::StatusCode, response::sse::{Event, KeepAlive, Sse}};
use futures_core::Stream;
use std::convert::Infallible;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{数据服务状态, 受理失败, 受理开发任务};

/// 开发受理请求体
#[derive(Debug, Deserialize)]
pub struct 开发受理请求 {
    /// 任务文本（下达给智能体的自然语言开发任务）
    pub 任务: String,
}

/// 开发受理响应体
#[derive(Debug, Serialize)]
pub struct 开发受理响应 {
    pub 受理: bool,
    pub 任务id: u64,
}

/// 停止响应体
#[derive(Debug, Serialize)]
pub struct 停止响应 {
    pub 中断: bool,
}

/// 事件查询响应体：执行台状态 + 增量事件
#[derive(Debug, Serialize)]
pub struct 开发事件响应 {
    pub 就绪: bool,
    pub 运行中: bool,
    pub 工作区: String,
    pub 最近结果: Option<String>,
    pub 事件: Vec<crate::事件记录>,
}

/// 事件游标查询参数
#[derive(Debug, Deserialize)]
pub struct 事件游标 {
    pub since: Option<u64>,
}

/// 错误响应体
#[derive(Debug, Serialize)]
pub struct 受理错误响应 {
    pub 错误: String,
}

/// POST /api/dev/agent：受理开发任务，后台线程执行智能体循环
pub async fn 受理开发任务接口(
    状态: State<数据服务状态>,
    Json(请求): Json<开发受理请求>,
) -> Result<Json<开发受理响应>, (StatusCode, Json<受理错误响应>)> {
    match 受理开发任务(&状态, 请求.任务) {
        Ok(id) => Ok(Json(开发受理响应 { 受理: true, 任务id: id })),
        Err(失败) => Err((失败状态码(&失败), Json(受理错误响应 { 错误: 失败消息(&失败) }))),
    }
}

/// GET /api/dev/events?since=N：执行台状态 + 序号大于 N 的增量事件
pub async fn 事件查询(
    状态: State<数据服务状态>,
    Query(游标): Query<事件游标>,
) -> Json<开发事件响应> {
    let 台 = &状态.开发执行台;
    let 汇总 = 台.当前状态();
    Json(开发事件响应 {
        就绪: 汇总.就绪,
        运行中: 汇总.运行中,
        工作区: 汇总.工作区,
        最近结果: 汇总.最近结果,
        事件: 台.事件增量(游标.since.unwrap_or(0)),
    })
}

/// POST /api/dev/agent/stop：置位中断句柄请求停止循环（幂等）
pub async fn 停止执行(状态: State<数据服务状态>) -> Json<停止响应> {
    let 中断 = 状态.开发执行台.中断();
    if !中断 {
        tracing::warn!("停止请求被忽略：智能体未装配上线");
    }
    Json(停止响应 { 中断 })
}

/// 工作区更新请求体
#[derive(Debug, Deserialize)]
pub struct 工作区请求 {
    pub 工作区: String,
}

/// POST /api/dev/workspace：更新智能体工作区（重新装配执行器）
pub async fn 更新工作区(
    状态: State<数据服务状态>,
    Json(请求): Json<工作区请求>,
) -> Result<StatusCode, (StatusCode, String)> {
    let 工作区 = 请求.工作区.trim();
    if 工作区.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "工作区路径不能为空".into()));
    }
    if 状态.开发执行台.运行中() {
        return Err((StatusCode::CONFLICT, "任务执行中，无法切换工作区".into()));
    }
    match &状态.重装配工作区 {
        Some(回调) => match 回调(工作区) {
            Ok(()) => {
                tracing::info!("工作区已切换: {工作区}");
                Ok(StatusCode::OK)
            }
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("重装配失败: {e}"))),
        },
        None => Err((StatusCode::SERVICE_UNAVAILABLE, "智能体未上线，无法切换工作区".into())),
    }
}

/// 受理失败 → HTTP 状态码
fn 失败状态码(失败: &受理失败) -> StatusCode {
    match 失败 {
        受理失败::未上线 => StatusCode::SERVICE_UNAVAILABLE,
        受理失败::运行中 => StatusCode::CONFLICT,
        受理失败::任务为空 | 受理失败::无待确认 | 受理失败::创建受阻(_) => StatusCode::BAD_REQUEST,
    }
}

/// 受理失败 → 面向用户的错误消息
fn 失败消息(失败: &受理失败) -> String {
    match 失败 {
        受理失败::未上线 => "智能体未上线：需配置 LLM_API_KEY 并开启 run_dev_agent 后重启".into(),
        受理失败::运行中 => "已有任务执行中，请等待完成后再下达".into(),
        受理失败::任务为空 => "任务内容不能为空".into(),
        受理失败::无待确认 => "当前无待确认需求：请先与道祖澄清并对齐".into(),
        受理失败::创建受阻(e) => format!("任务创建受阻: {e}"),
    }
}

/// 道祖对话请求体
#[derive(Debug, Deserialize)]
pub struct 道祖对话请求 {
    pub 消息: String,
}

/// 道祖对话响应体：阶段 + 回复 + 待确认需求 + 自动发布任务ID
#[derive(Debug, Serialize)]
pub struct 道祖对话响应 {
    pub 阶段: hm_agent::会话阶段,
    pub 回复: String,
    pub 需求: Option<hm_agent::需求摘要>,
    /// 道祖对齐后自动发布到看板的任务ID（None=未对齐/仍在澄清中）
    pub 任务id: Option<u64>,
}

/// 道祖确认响应体
#[derive(Debug, Serialize)]
pub struct 道祖确认响应 {
    pub 任务id: u64,
}

/// POST /api/dev/chat：道祖接待用户消息（闲聊/澄清/对齐）
pub async fn 道祖对话接口(
    状态: State<数据服务状态>,
    Json(请求): Json<道祖对话请求>,
) -> Result<Json<道祖对话响应>, (StatusCode, Json<受理错误响应>)> {
    // 1. 道祖接待用户消息（锁内调LLM）
    let 接待 = 状态.道祖接待.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(受理错误响应 { 错误: "道祖未上线：需配置 LLM 并开启 run_dev_agent".into() }),
    ))?;
    let (阶段, 回复, 需求) = {
        let 接待锁 = 接待.lock().expect("道祖接待锁中毒");
        let 响应 = 接待锁.接待(请求.消息).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(受理错误响应 { 错误: e.to_string() }),
            )
        })?;
        (响应.阶段, 响应.回复, 响应.需求)
    };

    // 2. 道祖对齐完需求 → 停留「待确认」，等待用户确认（/api/dev/chat/confirm）后才发布看板
    //    不再自动发布；若驱动台忙等异常也留待确认，由确认接口统一处理。
    let 任务id: Option<u64> = None;

    Ok(Json(道祖对话响应 {
        阶段,
        回复,
        需求,
        任务id,
    }))
}

/// POST /api/dev/chat/stream：道祖接待流式（SSE）。
/// 事件序列：RUN_STARTED → TEXT_MESSAGE_CONTENT×N → TEXT_MESSAGE_END → RUN_FINISHED。
/// 接待 LLM 调用在 blocking 线程执行（std 锁不适合 async），增量经 mpsc 通道转交 SSE 推送。
pub async fn 道祖对话流式接口(
    状态: State<数据服务状态>,
    Json(请求): Json<道祖对话请求>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let 状态克隆 = 状态.clone();
    let (发送端, mut 接收端) = tokio::sync::mpsc::channel::<serde_json::Value>(64);
    let _处理 = tokio::task::spawn_blocking(move || {
        let 发帧 = |帧: serde_json::Value| {
            let _ = 发送端.blocking_send(帧);
        };
        发帧(json!({"type": "RUN_STARTED", "会话阶段": "接待中"}));
        // 道祖未上线：直接收尾
        let Some(接待) = 状态克隆.道祖接待.clone() else {
            发帧(json!({"type": "TEXT_MESSAGE_END"}));
            发帧(json!({"type": "RUN_FINISHED", "阶段": "接待中", "任务id": null}));
            return;
        };
        // 流式接待：锁内仅做消息组装与会话落定，LLM 增量经回调外推（不阻塞 SSE）
        let 结果 = {
            let 接待锁 = 接待.lock().expect("道祖接待锁中毒");
            let mut 回调 = |块: String| -> Result<(), hm_error::Error> {
                发帧(json!({"type": "TEXT_MESSAGE_CONTENT", "delta": 块}));
                Ok(())
            };
            match 接待锁.接待流式(请求.消息.clone(), &mut 回调) {
                Ok(响应) => Some(响应),
                Err(e) => {
                    tracing::warn!("道祖流式接待失败: {e}");
                    发帧(json!({"type": "TEXT_MESSAGE_CONTENT", "delta": format!("（道祖未能答复：{e}）")}));
                    None
                }
            }
        };
        // 对齐 → 停留「待确认」，由用户确认（/api/dev/chat/confirm）后才发布看板；不再自动发布。
        let mut 阶段 = "接待中";
        let 任务id: Option<u64> = None;
        if let Some(响应) = 结果 {
            阶段 = match 响应.阶段 {
                hm_agent::会话阶段::待确认 => "待确认",
                hm_agent::会话阶段::接待中 => "接待中",
            };
        }
        发帧(json!({"type": "TEXT_MESSAGE_END"}));
        发帧(json!({"type": "RUN_FINISHED", "阶段": 阶段, "任务id": 任务id}));
    });
    let 流 = async_stream::stream! {
        while let Some(帧) = 接收端.recv().await {
            yield Ok(Event::default().data(帧.to_string()));
        }
    };
    Sse::new(流).keep_alive(KeepAlive::default())
}

/// POST /api/dev/chat/confirm：确认发布对齐需求（落看板 + 写记忆 + 自动驱动）
pub async fn 道祖确认接口(
    状态: State<数据服务状态>,
) -> Result<Json<道祖确认响应>, (StatusCode, Json<受理错误响应>)> {
    match crate::确认发布对齐需求(&状态) {
        Ok(id) => Ok(Json(道祖确认响应 { 任务id: id })),
        Err(失败) => Err((失败状态码(&失败), Json(受理错误响应 { 错误: 失败消息(&失败) }))),
    }
}
