use axum::{Json, extract::{Path, State}};
use hm_http::{数据服务状态, 看板发布, 看板查询, 看板定向回退, 定向回退请求, 发布任务请求};
use tc_task::{DesignDoc, ImplementationDoc, TaskStatus};

use super::*;

async fn 发布(状态: &数据服务状态) -> u64 {
    let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
        title: "回退接口任务".into(),
        description: "验证定向回退接口".into(),
        scene: None,
        priority: None,
        临时规则: None,
    }))
    .await
    .expect("发布应成功");
    id
}

/// 给任务注入设计/实现文档（追溯器自动判断用）
fn 注入文档(状态: &数据服务状态, id: u64) {
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    let 标识 = board.查询(id).expect("任务").任务标识.任务id;
    board
        .按标识改写(&标识, |t| {
            t.设计文档 = Some(
                serde_json::from_str::<DesignDoc>(
                    r#"{"边界定义":{},"契约":[],"修改文件":[],"新建文件":["src/加法.rs"],"依赖":[],"created_at":1}"#,
                )
                .expect("设计文档解析"),
            );
            t.实现文档 = Some(
                serde_json::from_str::<ImplementationDoc>(
                    r#"{"代码变更":[{"文件路径":"src/减法.rs","变更类型":"新增","摘要":"越界"}],"自检":{"通过":true,"边界合规":true,"契约合规":true},"created_at":1}"#,
                )
                .expect("实现文档解析"),
            );
        })
        .expect("注入文档应成功");
}

/// 测试1：手动回退 API——提供建议根源层级=土 → 状态=待修复，回退记录完整
#[tokio::test]
async fn rollback接口_建议土层级直接回退() {
    let 状态 = 构造状态();
    let id = 发布(&状态).await;

    let Json(响应) = 看板定向回退(
        State(状态.clone()),
        Path(id),
        Json(定向回退请求 {
            错误描述: "函数返回值不对".into(),
            建议根源层级: Some("土".into()),
        }),
    )
    .await
    .expect("回退应成功");

    assert!(响应.成功);
    assert_eq!(响应.回退到, "待修复", "土层级回退到待修复");
    assert_eq!(响应.回退次数, 1);

    let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
        panic!("查询应成功");
    };
    assert_eq!(任务.status, TaskStatus::待修复);
    let 回退 = 任务.回退来源.as_ref().expect("应有回退记录");
    assert_eq!(回退.目标层级, tc_task::五行层级::土);
    assert_eq!(回退.回退次数, 1);
}

/// 测试2：手动回退 API——建议根源层级=火 → 状态=待圣人设计
#[tokio::test]
async fn rollback接口_建议火层级回退到设计() {
    let 状态 = 构造状态();
    let id = 发布(&状态).await;

    let Json(响应) = 看板定向回退(
        State(状态.clone()),
        Path(id),
        Json(定向回退请求 {
            错误描述: "循环依赖".into(),
            建议根源层级: Some("火".into()),
        }),
    )
    .await
    .expect("回退应成功");
    assert!(响应.成功);
    assert_eq!(响应.回退到, "待圣人设计");

    let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
        panic!("查询应成功");
    };
    assert_eq!(任务.status, TaskStatus::待圣人设计);
    assert_eq!(任务.当前层级, tc_task::五行层级::火);
}

/// 测试3：未提供建议层级 → 追溯器自动判断（实现越界设计清单 → 根源=土）
#[tokio::test]
async fn rollback接口_无建议自动追溯() {
    let 状态 = 构造状态();
    let id = 发布(&状态).await;
    注入文档(&状态, id);

    let Json(响应) = 看板定向回退(
        State(状态.clone()),
        Path(id),
        Json(定向回退请求 {
            错误描述: "验收不通过".into(),
            建议根源层级: None,
        }),
    )
    .await
    .expect("回退应成功");

    assert!(响应.成功);
    assert_eq!(响应.回退到, "待修复", "追溯器判实现越界 → 土");
    let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
        panic!("查询应成功");
    };
    assert_eq!(任务.回退来源.as_ref().expect("回退记录").原因, "实现Bug");
}

/// 测试4：任务不存在 → 404
#[tokio::test]
async fn rollback接口_任务不存在404() {
    let 状态 = 构造状态();
    let 结果 = 看板定向回退(
        State(状态),
        Path(999),
        Json(定向回退请求 {
            错误描述: "不存在".into(),
            建议根源层级: Some("土".into()),
        }),
    )
    .await;
    assert!(结果.is_err(), "任务不存在应返回错误");
}

/// 测试5：手动回退联动召回——受影响依赖任务被置为「待重新*」并打召回标记
#[tokio::test]
async fn rollback接口_触发连带召回() {
    let 状态 = 构造状态();
    let 基础id = 发布(&状态).await;
    let 依赖id = 发布(&状态).await;
    {
        let mut board = 状态.任务看板.lock().expect("看板锁中毒");
        let 基础uuid = board.查询(基础id).expect("基础任务").任务标识.任务id;
        let 依赖uuid = board.查询(依赖id).expect("依赖任务").任务标识.任务id;
        board
            .按标识改写(&依赖uuid, |t| {
                t.任务标识.依赖任务 = vec![基础uuid];
                t.status = TaskStatus::已完成;
            })
            .expect("改写应成功");
    }

    let Json(响应) = 看板定向回退(
        State(状态.clone()),
        Path(基础id),
        Json(定向回退请求 {
            错误描述: "验收不通过".into(),
            建议根源层级: Some("土".into()),
        }),
    )
    .await
    .expect("回退应成功");

    assert!(响应.成功);
    assert_eq!(响应.回退到, "待修复");
    assert_eq!(响应.影响任务数, 1, "依赖者应被连带召回");

    let board = 状态.任务看板.lock().expect("看板锁中毒");
    let 依赖 = board.查询(依赖id).expect("依赖任务应存在");
    assert_eq!(依赖.status, TaskStatus::待重新验收, "已完成的依赖者被召回为待重新验收");
    assert!(依赖.召回标记, "应打召回标记");
}
