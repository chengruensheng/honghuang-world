use std::convert::Infallible;
use std::time::Duration;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use crate::{数据服务状态, 看板驱动状态, 事件游标, 受理错误响应, 会话清单响应, 驱动会话详情};

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
    台.启动执行一轮("手动驱动");
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
    台.启动执行到空闲(上限, "手动到空闲");
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

/// 驱动过程事件响应体
#[derive(Debug, Serialize)]
pub struct 驱动过程响应 {
    pub 就绪: bool,
    pub 运行中: bool,
    pub 事件: Vec<crate::驱动过程事件>,
}

/// GET /api/dev/pilot/process?since=N：驱动过程事件增量（智能体循环每步：思考/工具调用/工具结果/答复）
pub async fn 看板驱动过程接口(
    状态: State<数据服务状态>,
    Query(游标): Query<事件游标>,
) -> Json<驱动过程响应> {
    let 台 = &状态.看板驱动台;
    Json(驱动过程响应 {
        就绪: 台.就绪(),
        运行中: 台.运行中(),
        事件: 台.过程事件增量(游标.since.unwrap_or(0)),
    })
}

/// GET /api/dev/sessions：历史会话清单（按创建时间倒序）
pub async fn 会话清单接口(状态: State<数据服务状态>) -> Json<会话清单响应> {
    Json(会话清单响应 {
        会话: 状态.看板驱动台.会话清单(),
    })
}

/// GET /api/dev/sessions/{id}：会话回放（元数据 + 全程过程事件）
pub async fn 会话回放接口(
    状态: State<数据服务状态>,
    Path(会话id): Path<u64>,
) -> Result<Json<驱动会话详情>, (StatusCode, Json<受理错误响应>)> {
    match 状态.看板驱动台.会话回放(会话id) {
        Some(详情) => Ok(Json(详情)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(受理错误响应 { 错误: "会话不存在".into() }),
        )),
    }
}

/// 定向驱动请求体（Resume/Fork 共用：目标任务 id）
#[derive(Debug, Deserialize)]
pub struct 会话定向请求 {
    pub 任务id: u64,
}

/// 分叉请求体：可指定是否继承源会话检查点消息（缺省继承）
#[derive(Debug, Deserialize)]
pub struct 会话分叉请求 {
    pub 任务id: u64,
    #[serde(default = "默认继承")]
    pub 继承消息: bool,
}

/// 分叉请求缺省继承源会话检查点消息
fn 默认继承() -> bool {
    true
}

/// 分叉响应体：返回新会话 id
#[derive(Debug, Serialize)]
pub struct 分叉响应 {
    pub 会话id: u64,
}

/// 把 启动定向驱动 的字符串错误映射为 HTTP 状态码/响应体
fn 映射驱动错误(错误: String) -> (StatusCode, Json<受理错误响应>) {
    if 错误.contains("未就绪") {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(受理错误响应 { 错误: 错误.into() }),
        );
    }
    if 错误.contains("执行中") {
        return (
            StatusCode::CONFLICT,
            Json(受理错误响应 { 错误: 错误.into() }),
        );
    }
    (
        StatusCode::BAD_REQUEST,
        Json(受理错误响应 { 错误: 错误.into() }),
    )
}

/// POST /api/dev/sessions/{id}/resume：定向恢复驱动（携带源会话检查点消息续跑）
pub async fn 会话恢复接口(
    状态: State<数据服务状态>,
    Path(会话id): Path<u64>,
    Json(请求): Json<会话定向请求>,
) -> Result<Json<驱动受理响应>, (StatusCode, Json<受理错误响应>)> {
    let 台 = &状态.看板驱动台;
    if 台.读取检查点(会话id, 请求.任务id).is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(受理错误响应 { 错误: "无可恢复检查点（该会话未落盘该任务的运行断点）".into() }),
        ));
    }
    match 台.启动定向驱动(会话id, 请求.任务id, true, "恢复") {
        Ok(_会话id) => Ok(Json(驱动受理响应 { 受理: true })),
        Err(e) => Err(映射驱动错误(e)),
    }
}

/// POST /api/dev/sessions/{id}/fork：从源会话分叉一条新驱动线（默认继承源断点消息），返回新会话 id
pub async fn 会话分叉接口(
    状态: State<数据服务状态>,
    Path(会话id): Path<u64>,
    Json(请求): Json<会话分叉请求>,
) -> Result<Json<分叉响应>, (StatusCode, Json<受理错误响应>)> {
    let 台 = &状态.看板驱动台;
    match 台.启动定向驱动(会话id, 请求.任务id, 请求.继承消息, "分叉") {
        Ok(新会话id) => Ok(Json(分叉响应 { 会话id: 新会话id })),
        Err(e) => Err(映射驱动错误(e)),
    }
}

/// GET /api/dev/stream?since=N：驱动过程事件 SSE 流（增量推送，15s 心跳保活；并发超限 429）
pub async fn 看板驱动过程流接口(
    状态: State<数据服务状态>,
    Query(游标): Query<事件游标>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let Ok(许可) = 状态.sse信号量.clone().try_acquire_owned() else {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    };
    let 台 = 状态.看板驱动台.clone();
    let mut 游标 = 游标.since.unwrap_or(0);
    let 流 = async_stream::stream! {
        let _持有 = 许可;
        loop {
            for 事件 in 台.过程事件增量(游标) {
                游标 = 事件.序号;
                let 载荷 = serde_json::to_string(&事件).unwrap_or_else(|_| "{}".into());
                yield Ok(Event::default().data(载荷));
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    };
    Ok(Sse::new(流).keep_alive(KeepAlive::default()))
}

/// GET /api/dev/stream/state?since=N：驱动阶段事件 SSE 流（增量推送，15s 心跳保活；并发超限 429）
pub async fn 看板驱动阶段流接口(
    状态: State<数据服务状态>,
    Query(游标): Query<事件游标>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let Ok(许可) = 状态.sse信号量.clone().try_acquire_owned() else {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    };
    let 台 = 状态.看板驱动台.clone();
    let mut 游标 = 游标.since.unwrap_or(0);
    let 流 = async_stream::stream! {
        let _持有 = 许可;
        loop {
            for 事件 in 台.驱动事件增量(游标) {
                游标 = 事件.序号;
                let 载荷 = serde_json::to_string(&事件).unwrap_or_else(|_| "{}".into());
                yield Ok(Event::default().data(载荷));
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    };
    Ok(Sse::new(流).keep_alive(KeepAlive::default()))
}

/// GET /api/dev/stream/agui?since=N：驱动过程事件 → AG-UI 标准事件 SSE 流（适配器映射，15s 心跳保活；并发超限 429）
pub async fn 看板驱动过程流_agui接口(
    状态: State<数据服务状态>,
    Query(游标): Query<事件游标>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let Ok(许可) = 状态.sse信号量.clone().try_acquire_owned() else {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    };
    let 台 = 状态.看板驱动台.clone();
    let mut 游标 = 游标.since.unwrap_or(0);
    let mut 适配器 = crate::协议适配器::新();
    let 流 = async_stream::stream! {
        let _持有 = 许可;
        loop {
            let 会话id = 台.当前会话id().unwrap_or(0);
            for 事件 in 台.过程事件增量(游标) {
                游标 = 事件.序号;
                for 协议事件 in 适配器.过程事件(&事件, 会话id) {
                    let mut 载荷json: serde_json::Value = serde_json::to_value(&协议事件).unwrap_or_else(|_| serde_json::json!({}));
                    // 往JSON里加角色字段，前端按角色分发到对应组件
                    if let Some(角色) = &事件.角色 {
                        载荷json["角色"] = serde_json::Value::String(角色.clone());
                    }
                    let 载荷 = serde_json::to_string(&载荷json).unwrap_or_else(|_| "{}".into());
                    yield Ok(Event::default().data(载荷));
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    };
    Ok(Sse::new(流).keep_alive(KeepAlive::default()))
}

/// GET /api/dev/stream/agui/state?since=N：驱动阶段事件 → AG-UI 标准事件 SSE 流（适配器映射；并发超限 429）
pub async fn 看板驱动阶段流_agui接口(
    状态: State<数据服务状态>,
    Query(游标): Query<事件游标>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let Ok(许可) = 状态.sse信号量.clone().try_acquire_owned() else {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    };
    let 台 = 状态.看板驱动台.clone();
    let mut 游标 = 游标.since.unwrap_or(0);
    let 适配器 = crate::协议适配器::新();
    let 流 = async_stream::stream! {
        let _持有 = 许可;
        loop {
            let 会话id = 台.当前会话id().unwrap_or(0);
            for 事件 in 台.驱动事件增量(游标) {
                游标 = 事件.序号;
                for 协议事件 in 适配器.阶段事件(&事件, 会话id) {
                    let 载荷 = serde_json::to_string(&协议事件).unwrap_or_else(|_| "{}".into());
                    yield Ok(Event::default().data(载荷));
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    };
    Ok(Sse::new(流).keep_alive(KeepAlive::default()))
}
