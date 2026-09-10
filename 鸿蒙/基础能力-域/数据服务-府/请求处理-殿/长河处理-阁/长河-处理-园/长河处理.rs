//! 长河处理-阁：水镜统一通道 /api/river 三端点。
//!
//! 设计稿 v1「一条 SSE 多路复用长河事件」的后端落点：
//! - POST /api/river/chat  —— 接待流式（己言→回复开始/增量/结束→运行结束，长河事件直出）；
//! - GET  /api/river/stream —— 驱动过程/阶段事件多路复用（译制为长河事件，长连接推送）；
//! - GET  /api/river/events —— 游标增量回放（断线续传/历史快照，按河序归约）。
//!
//! 所有产出事件统一写入 长河总线，供 events 端点按游标重放。

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_core::Stream;
use serde::Serialize;

use qk_mirror::长河事件;

use crate::{数据服务状态, 事件游标, 道祖对话请求};

/// 长河事件增量响应体（events 端点）
#[derive(Debug, Serialize)]
pub struct 长河事件响应 {
    /// 当前最高河序（下次续传的 since 值）
    pub 游标: u64,
    /// 河序大于 since 的长河事件（有序）
    pub 事件: Vec<长河事件>,
}

/// 把长河事件序列化为 wire 载荷（{"类型":..., ...中文字段}）
fn 串(事: &长河事件) -> String {
    serde_json::to_string(事).unwrap_or_else(|_| "{}".into())
}

/// 己方消息 id 的唯一后缀（毫秒时间戳，避免同秒冲突）
fn 己言后缀() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// POST /api/river/chat：接待流式（长河事件 SSE）。
///
/// 事件序列：己言 → 回复开始 → 回复增量×N → 回复结束 → 运行结束。
/// 接待 LLM 调用在 blocking 线程执行（std 锁不适合 async），增量经 mpsc 转交 SSE 推送；
/// 每帧同时写入长河总线供回放。并发超限返回 429。
pub async fn 长河接待流接口(
    状态: State<数据服务状态>,
    Json(请求): Json<道祖对话请求>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let Ok(许可) = 状态.sse信号量.clone().try_acquire_owned() else {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    };
    let 状态克隆 = 状态.clone();
    let (发送端, mut 接收端) = tokio::sync::mpsc::channel::<String>(64);
    let _处理 = tokio::task::spawn_blocking(move || {
        let 总线 = 状态克隆.长河总线.clone();
        let 发帧 = |帧: String| {
            let _ = 发送端.blocking_send(帧);
        };
        let 录派 = |事: 长河事件| {
            总线.记录(事.clone());
            发帧(串(&事));
        };

        // 己方发言入河（刷新/恢复后历史可见）
        let 己id = format!("己-{}", 己言后缀());
        录派(长河事件::己言 { 消息id: 己id, 文: 请求.消息.clone() });
        录派(长河事件::回复开始 { 消息id: "接待".into(), 角色: "道祖".into() });

        // 道祖未上线：直接收尾
        let Some(接待) = 状态克隆.道祖接待.clone() else {
            录派(长河事件::回复结束 { 消息id: "接待".into() });
            录派(长河事件::运行结束 {
                阶段: qk_mirror::阶段::接待中.to_string(),
                任务id: None,
            });
            return;
        };

        // 流式接待：锁内做消息组装，LLM 增量经回调外推（不阻塞 SSE）
        let 结果 = {
            let 接待锁 = 接待.lock().expect("道祖接待锁中毒");
            let mut 回调 = |块: String| -> Result<(), hm_error::Error> {
                录派(长河事件::回复增量 { 消息id: "接待".into(), 文: 块 });
                Ok(())
            };
            match 接待锁.接待流式(请求.消息.clone(), &mut 回调) {
                Ok(响应) => Some(响应),
                Err(e) => {
                    tracing::warn!("长河接待流失败: {e}");
                    录派(长河事件::回复增量 {
                        消息id: "接待".into(),
                        文: format!("（道祖未能答复：{e}）"),
                    });
                    None
                }
            }
        };

        // 对齐 → 停留「待确认」，由确认接口发布看板；此处不自动发布。
        let mut 阶段 = qk_mirror::阶段::接待中.to_string();
        if let Some(响应) = 结果 {
            阶段 = match 响应.阶段 {
                hm_agent::会话阶段::待确认 => qk_mirror::阶段::待确认.to_string(),
                hm_agent::会话阶段::接待中 => qk_mirror::阶段::接待中.to_string(),
            };
        }
        录派(长河事件::回复结束 { 消息id: "接待".into() });
        录派(长河事件::运行结束 { 阶段, 任务id: None });
    });

    let 流 = async_stream::stream! {
        let _持有 = 许可;
        while let Some(帧) = 接收端.recv().await {
            yield Ok(Event::default().data(帧));
        }
    };
    Ok(Sse::new(流).keep_alive(KeepAlive::default()))
}

/// GET /api/river/stream：驱动过程/阶段事件多路复用（长河事件 SSE）。
///
/// 过程事件（思考/工具/答复）与阶段事件（任务状态/运行结束/河错）合并为单一长河流，
/// 各自维护游标（序号独立、预留时重置）。15s 心跳保活；并发超限 429。
pub async fn 长河过程流接口(
    状态: State<数据服务状态>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let Ok(许可) = 状态.sse信号量.clone().try_acquire_owned() else {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    };
    let 台 = 状态.看板驱动台.clone();
    let 总线 = 状态.长河总线.clone();
    let mut 过程游标: u64 = 0;
    let mut 阶段游标: u64 = 0;
    let mut 译 = crate::译制器::新();
    let 流 = async_stream::stream! {
        let _持有 = 许可;
        loop {
            let 会话id = 台.当前会话id().unwrap_or(0);
            for 事件 in 台.过程事件增量(过程游标) {
                过程游标 = 事件.序号;
                for 长河 in 译.过程事件(&事件, 会话id) {
                    总线.记录(长河.clone());
                    yield Ok(Event::default().data(串(&长河)));
                }
            }
            for 事件 in 台.驱动事件增量(阶段游标) {
                阶段游标 = 事件.序号;
                for 长河 in 译.阶段事件(&事件) {
                    总线.记录(长河.clone());
                    yield Ok(Event::default().data(串(&长河)));
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    };
    Ok(Sse::new(流).keep_alive(KeepAlive::default()))
}

/// GET /api/river/events?since=N：长河事件游标增量回放（JSON）。
///
/// 断线续传 / 历史快照入口：返回河序大于 since 的事件与当前最高河序。
pub async fn 长河事件查询接口(
    状态: State<数据服务状态>,
    Query(游标): Query<事件游标>,
) -> Json<长河事件响应> {
    let 总线 = &状态.长河总线;
    let since = 游标.since.unwrap_or(0);
    let 事件 = 总线.增量(since).into_iter().map(|帧| 帧.事).collect();
    Json(长河事件响应 {
        游标: 总线.当前河序(),
        事件,
    })
}
