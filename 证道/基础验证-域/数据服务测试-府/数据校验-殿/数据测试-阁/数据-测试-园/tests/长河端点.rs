use super::*;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use http_body_util::BodyExt;
use hm_http::{
    长河接待流接口, 长河过程流接口, 长河事件查询接口,
    事件游标, 驱动阶段事件, 驱动过程事件, 道祖对话请求,
};
use qk_mirror::长河事件;
use std::time::Duration;

/// 从 SSE 响应 body 读下一数据帧的文本（超时即 panic，避免无限流悬挂）
async fn 取帧(body: &mut axum::body::Body) -> String {
    let 帧 = tokio::time::timeout(Duration::from_secs(2), body.frame())
        .await
        .expect("取帧超时")
        .expect("流结束")
        .expect("帧错误");
    let 数据 = 帧.into_data().expect("应为数据帧");
    String::from_utf8_lossy(&数据).to_string()
}

/// 过程流：注入「思考」→ 输出长河事件 思考开始/增量/结束 三段式
#[tokio::test]
async fn 长河端点_过程事件输出思考三段式() {
    let 状态 = 构造状态();
    状态.看板驱动台.记录过程事件(&驱动过程事件 {
        序号: 1,
        任务id: Some(5),
        角色: Some("圣人".into()),
        轮次: 0,
        类型: "思考".into(),
        工具名: String::new(),
        内容: "分析需求".into(),
        时间: 123,
    });

    let sse = 长河过程流接口(State(状态.clone())).await;
    let mut body = sse.into_response().into_body();
    let 一 = 取帧(&mut body).await;
    let 二 = 取帧(&mut body).await;
    let 三 = 取帧(&mut body).await;
    assert!(一.contains("\"类型\":\"思考开始\""), "实际 {}", 一);
    assert!(二.contains("\"类型\":\"思考增量\""), "实际 {}", 二);
    assert!(二.contains("分析需求"), "实际 {}", 二);
    assert!(三.contains("\"类型\":\"思考结束\""), "实际 {}", 三);
}

/// 过程流：注入「阶段完成」→ 输出长河事件 任务状态（状态词由后端唯一词表直出）
#[tokio::test]
async fn 长河端点_阶段完成输出任务状态() {
    let 状态 = 构造状态();
    状态.看板驱动台.记录驱动事件(&驱动阶段事件 {
        序号: 1,
        类型: "阶段完成".into(),
        任务id: Some(9),
        角色: Some("圣人".into()),
        新状态: Some("已完成".into()),
        层级: None,
        消息: None,
        时间: 123,
    });

    let sse = 长河过程流接口(State(状态.clone())).await;
    let mut body = sse.into_response().into_body();
    let 一 = 取帧(&mut body).await;
    assert!(一.contains("\"类型\":\"任务状态\""), "实际 {}", 一);
    assert!(一.contains("\"任务id\":9"), "实际 {}", 一);
    assert!(一.contains("已完成"), "实际 {}", 一);
}

/// 事件查询：经过程流写入总线后，按游标增量回放长河事件
#[tokio::test]
async fn 长河端点_事件查询按游标增量回放() {
    let 状态 = 构造状态();
    状态.看板驱动台.记录过程事件(&驱动过程事件 {
        序号: 1,
        任务id: Some(5),
        角色: Some("圣人".into()),
        轮次: 0,
        类型: "思考".into(),
        工具名: String::new(),
        内容: "分析".into(),
        时间: 123,
    });

    // 经过程流把三段思考事件写入长河总线
    let sse = 长河过程流接口(State(状态.clone())).await;
    let mut body = sse.into_response().into_body();
    let _ = 取帧(&mut body).await;
    let _ = 取帧(&mut body).await;
    let _ = 取帧(&mut body).await;

    let Json(响应) = 长河事件查询接口(State(状态.clone()), Query(事件游标 { since: Some(0) })).await;
    assert!(响应.游标 >= 3, "游标 {}", 响应.游标);
    assert!(
        响应.事件.iter().any(|e| matches!(e, 长河事件::思考开始 { .. })),
        "缺思考开始：{:?}",
        响应.事件
    );
    // 续传：since=游标后应无增量
    let Json(续) = 长河事件查询接口(State(状态.clone()), Query(事件游标 { since: Some(响应.游标) })).await;
    assert!(续.事件.is_empty(), "续传应无增量：{:?}", 续.事件);
}

/// 接待流：道祖未上线（构造状态无接待句柄）→ 输出 己言/回复开始/回复结束/运行结束
#[tokio::test]
async fn 长河端点_接待流道祖未上线输出己言与收尾() {
    let 状态 = 构造状态();
    let sse = 长河接待流接口(State(状态.clone()), axum::Json(道祖对话请求 { 消息: "你好".into() })).await;
    let mut body = sse.into_response().into_body();
    let 一 = 取帧(&mut body).await; // 己言
    let 二 = 取帧(&mut body).await; // 回复开始
    let 三 = 取帧(&mut body).await; // 回复结束
    let 四 = 取帧(&mut body).await; // 运行结束
    assert!(一.contains("\"类型\":\"己言\""), "实际 {}", 一);
    assert!(一.contains("你好"), "实际 {}", 一);
    assert!(二.contains("\"类型\":\"回复开始\""), "实际 {}", 二);
    assert!(三.contains("\"类型\":\"回复结束\""), "实际 {}", 三);
    assert!(四.contains("\"类型\":\"运行结束\""), "实际 {}", 四);
}
