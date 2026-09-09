use super::*;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use http_body_util::BodyExt;
use hm_http::{看板驱动阶段流_agui接口, 看板驱动过程流_agui接口, 事件游标, 驱动阶段事件, 驱动过程事件};
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

/// 阶段事件端点：注入「空闲」→ 输出 RUN_FINISHED + threadId/runId
#[tokio::test]
async fn agui端点_阶段事件输出run_finished() {
    let 状态 = 构造状态();
    状态.看板驱动台.记录驱动事件(&驱动阶段事件 {
        序号: 1,
        类型: "空闲".into(),
        任务id: None,
        角色: None,
        新状态: None,
        层级: None,
        消息: None,
        时间: 123,
    });

    let sse = 看板驱动阶段流_agui接口(State(状态.clone()), Query(事件游标 { since: Some(0) })).await;
    let mut body = sse.into_response().into_body();
    let 文本 = 取帧(&mut body).await;
    assert!(文本.contains("RUN_FINISHED"), "实际 {}", 文本);
    assert!(文本.contains("threadId"), "实际 {}", 文本);
    assert!(文本.contains("runId"), "实际 {}", 文本);
}

/// 过程事件端点：注入「思考」→ 输出 REASONING 三段式
#[tokio::test]
async fn agui端点_过程事件输出推理三段式() {
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

    let sse = 看板驱动过程流_agui接口(State(状态.clone()), Query(事件游标 { since: Some(0) })).await;
    let mut body = sse.into_response().into_body();
    let 一 = 取帧(&mut body).await;
    let 二 = 取帧(&mut body).await;
    let 三 = 取帧(&mut body).await;
    assert!(一.contains("REASONING_MESSAGE_START"), "实际 {}", 一);
    assert!(二.contains("REASONING_MESSAGE_CONTENT"), "实际 {}", 二);
    assert!(二.contains("分析需求"), "实际 {}", 二);
    assert!(三.contains("REASONING_MESSAGE_END"), "实际 {}", 三);
}

/// 过程事件端点：注入「工具调用 + 工具结果」→ 输出 TOOL_CALL_* + TOOL_CALL_RESULT（复用工具调用 id）
#[tokio::test]
async fn agui端点_工具调用与结果复用toolcallid() {
    let 状态 = 构造状态();
    状态.看板驱动台.记录过程事件(&驱动过程事件 {
        序号: 3,
        任务id: Some(5),
        角色: Some("圣人".into()),
        轮次: 1,
        类型: "工具调用".into(),
        工具名: "写文件".into(),
        内容: "{\"path\":\"a.rs\"}".into(),
        时间: 123,
    });
    状态.看板驱动台.记录过程事件(&驱动过程事件 {
        序号: 4,
        任务id: Some(5),
        角色: Some("圣人".into()),
        轮次: 1,
        类型: "工具结果".into(),
        工具名: String::new(),
        内容: "写入成功".into(),
        时间: 123,
    });

    let sse = 看板驱动过程流_agui接口(State(状态.clone()), Query(事件游标 { since: Some(0) })).await;
    let mut body = sse.into_response().into_body();
    let 一 = 取帧(&mut body).await; // TOOL_CALL_START
    let 二 = 取帧(&mut body).await; // TOOL_CALL_ARGS
    let 三 = 取帧(&mut body).await; // TOOL_CALL_END
    let 四 = 取帧(&mut body).await; // TOOL_CALL_RESULT
    assert!(一.contains("TOOL_CALL_START"), "实际 {}", 一);
    assert!(一.contains("写文件"), "实际 {}", 一);
    assert!(二.contains("TOOL_CALL_ARGS"), "实际 {}", 二);
    assert!(三.contains("TOOL_CALL_END"), "实际 {}", 三);
    assert!(四.contains("TOOL_CALL_RESULT"), "实际 {}", 四);
    assert!(四.contains("写入成功"), "实际 {}", 四);
    // 工具结果复用最近一次工具调用的 toolCallId（序号由台内部分配：工具调用=1，会话 id 未设置兜底 0）
    assert!(四.contains("\"toolCallId\":\"tc-0-1\""), "实际 {}", 四);
}
