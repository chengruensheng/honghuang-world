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

/// 过程事件端点：注入「思考」→ 先开棒（RUN_STARTED）再推理三段式
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
    let 四 = 取帧(&mut body).await;
    assert!(一.contains("RUN_STARTED"), "实际 {}", 一);
    assert!(一.contains("角色"), "RUN_STARTED 也应带角色字段，实际 {}", 一);
    assert!(二.contains("REASONING_MESSAGE_START"), "实际 {}", 二);
    assert!(三.contains("REASONING_MESSAGE_CONTENT"), "实际 {}", 三);
    assert!(三.contains("分析需求"), "实际 {}", 三);
    assert!(四.contains("REASONING_MESSAGE_END"), "实际 {}", 四);
}

/// 过程事件端点：思考动画标记（「__思考开始__」等）不应作为推理内容下发
#[tokio::test]
async fn agui端点_思考标记不产出推理内容() {
    let 状态 = 构造状态();
    状态.看板驱动台.记录过程事件(&驱动过程事件 {
        序号: 1,
        任务id: Some(5),
        角色: Some("圣人".into()),
        轮次: 0,
        类型: "思考".into(),
        工具名: String::new(),
        内容: "__思考开始__".into(),
        时间: 123,
    });

    let sse = 看板驱动过程流_agui接口(State(状态.clone()), Query(事件游标 { since: Some(0) })).await;
    let mut body = sse.into_response().into_body();
    let 一 = 取帧(&mut body).await;
    assert!(一.contains("RUN_STARTED"), "实际 {}", 一);
    assert!(!一.contains("REASONING_MESSAGE"), "动画标记不得下发为推理，实际 {}", 一);
    状态.看板驱动台.记录过程事件(&驱动过程事件 {
        序号: 2,
        任务id: Some(5),
        角色: Some("圣人".into()),
        轮次: 0,
        类型: "工具调用".into(),
        工具名: "读文件".into(),
        内容: "{}".into(),
        时间: 123,
    });
    let 二 = 取帧(&mut body).await;
    assert!(二.contains("TOOL_CALL_START"), "后续事件应照常下发，实际 {}", 二);
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
    let 一 = 取帧(&mut body).await; // RUN_STARTED（本棒开头）
    let 二 = 取帧(&mut body).await; // TOOL_CALL_START
    let 三 = 取帧(&mut body).await; // TOOL_CALL_ARGS
    let 四 = 取帧(&mut body).await; // TOOL_CALL_END
    let 五 = 取帧(&mut body).await; // TOOL_CALL_RESULT
    assert!(一.contains("RUN_STARTED"), "实际 {}", 一);
    assert!(二.contains("TOOL_CALL_START"), "实际 {}", 二);
    assert!(二.contains("写文件"), "实际 {}", 二);
    assert!(三.contains("TOOL_CALL_ARGS"), "实际 {}", 三);
    assert!(四.contains("TOOL_CALL_END"), "实际 {}", 四);
    assert!(五.contains("TOOL_CALL_RESULT"), "实际 {}", 五);
    assert!(五.contains("写入成功"), "实际 {}", 五);
    // 工具结果复用最近一次工具调用的 toolCallId（序号由台内部分配：工具调用=1，会话 id 未设置兜底 0）
    assert!(五.contains("\"toolCallId\":\"tc-0-1\""), "实际 {}", 五);
}

/// 阶段事件端点：阶段帧也须署名。
///
/// 前端靠「角色」把收尾帧归位到对应那根棒；不署名时它只能挂到「当前最后一根棒」，
/// 而过程流与阶段流是两条独立通道、回放到达顺序不保证——末棒就会配上别人的终态。
#[tokio::test]
async fn agui端点_阶段事件带角色署名() {
    let 状态 = 构造状态();
    状态.看板驱动台.记录驱动事件(&驱动阶段事件 {
        序号: 1,
        类型: "阶段完成".into(),
        任务id: Some(2),
        角色: Some("太乙金仙".into()),
        新状态: Some("清理完成".into()),
        层级: None,
        消息: None,
        时间: 123,
    });

    let sse = 看板驱动阶段流_agui接口(State(状态.clone()), Query(事件游标 { since: Some(0) })).await;
    let mut body = sse.into_response().into_body();
    let 一 = 取帧(&mut body).await; // STATE_DELTA（状态机流转出的新状态）
    let 二 = 取帧(&mut body).await; // STEP_FINISHED
    assert!(一.contains("STATE_DELTA"), "实际 {}", 一);
    assert!(一.contains("/任务/2/status"), "补丁须指向真实任务的状态，实际 {}", 一);
    assert!(一.contains("清理完成"), "实际 {}", 一);
    assert!(二.contains("STEP_FINISHED"), "实际 {}", 二);
    assert!(二.contains("太乙金仙 · 清理与归档"), "步骤名须为「角色 · 职责」，实际 {}", 二);
    assert!(二.contains("\"角色\":\"太乙金仙\""), "阶段帧须带角色署名，实际 {}", 二);
}
