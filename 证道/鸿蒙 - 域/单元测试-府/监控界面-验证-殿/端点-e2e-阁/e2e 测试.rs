//! §11.f.4 监控界面端点 e2e 测试（HTTP axum TestServer）。
//!
//! 镜像：依据融合蓝图 §11.6.2 第 4 条 + §11.f.4。
//! 之前 §13.d / §11.f 的 7 个 e2e 测试内联在 jiankong_fu 内 mod self_check_e2e，本阁
//! 把 e2e 测试迁移到镜像殿（监控界面-验证-殿 / 端点-e2e-阁）。
//!
//! 测试运行：`cargo test -p zhengdao_fu --lib 监控界面_验证_殿::端点_e2e_阁`。

#![cfg(test)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use jiankong_fu::建路由;

/// 让 工作区::定位() 在镜像殿 cwd 下能找到项目根。
/// 注意：必须每次调用都设（不能 call_once），因为其他测试可能先改 WORLD_WORKSPACE_ROOT。
fn 初始化工作区() {
    // 先 unset（防止上游测试留下污染值）
    std::env::remove_var("WORLD_WORKSPACE_ROOT");
    // 再向上找项目根
    let 当前 = std::env::current_dir().unwrap();
    for 目录 in 当前.ancestors() {
        if 目录.join("AGENTS.md").exists() {
            std::env::set_var("WORLD_WORKSPACE_ROOT", 目录);
            return;
        }
    }
}

async fn 构造应用() -> axum::Router {
    初始化工作区();
    建路由()
}

async fn get_json(router: &axum::Router, path: &str) -> (StatusCode, serde_json::Value) {
    let resp = router
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// §13.d /api/self-check/targets 返回 ≥ 14 个目标（含 all + 13 个 crate）
#[tokio::test]
async fn self_check_targets_列表完整() {
    let router = 构造应用().await;
    let (status, json) = get_json(&router, "/api/self-check/targets").await;
    assert_eq!(status, StatusCode::OK);
    let targets = json["targets"].as_array().expect("targets 应为数组");
    assert!(
        targets.len() >= 14,
        "targets 应 ≥ 14 个，实际 {}",
        targets.len()
    );
    let has_all = targets.iter().any(|t| t["target"].as_str() == Some("all"));
    assert!(has_all, "应包含 'all' target");
    let has_mingling = targets
        .iter()
        .any(|t| t["target"].as_str() == Some("命令操作-府"));
    assert!(has_mingling, "应包含 '命令操作-府' target");
}

/// §13.d /api/self-check?target=命令操作-府 返回健康分 100
#[tokio::test]
async fn self_check_命令操作_府_健康分100() {
    let router = 构造应用().await;
    let (status, json) = get_json(&router, "/api/self-check?target=命令操作-府").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.get("error").is_none(), "不应有 error: {:?}", json);
    let score = json["health"]["score"].as_u64().expect("score 应为 u64");
    assert_eq!(score, 100, "命令操作-府 应 100 分，实际 {}", score);
    let 等级 = json["health"]["等级"].as_str().expect("等级 应为字符串");
    assert_eq!(等级, "绿", "命令操作-府 应为绿，实际 {}", 等级);
}

/// §13.d /api/self-check 报告字段齐全
#[tokio::test]
async fn self_check_报告字段齐全() {
    let router = 构造应用().await;
    let (status, json) = get_json(&router, "/api/self-check").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["target"].is_object(), "target 应是对象");
    assert!(json["daoyun"].is_object(), "daoyun 应是对象");
    assert!(json["workspace"].is_object(), "workspace 应是对象");
    assert!(json["tests"].is_object(), "tests 应是对象");
    assert!(json["docs"].is_object(), "docs 应是对象");
    assert!(json["health"].is_object(), "health 应是对象");
    let target = &json["target"];
    assert!(target["输入"].is_string(), "target.输入 应是字符串");
    assert!(target["路径"].is_string(), "target.路径 应是字符串");
    assert!(target["标签"].is_string(), "target.标签 应是字符串");
    assert!(
        json["workspace"]["files"].is_number(),
        "workspace.files 应是数字"
    );
    assert!(
        json["workspace"]["lines"].is_number(),
        "workspace.lines 应是数字"
    );
    assert!(json["health"]["score"].is_number(), "health.score 应是数字");
    assert!(json["health"]["等级"].is_string(), "health.等级 应是字符串");
    assert!(
        json["health"]["扣分项"].is_array(),
        "health.扣分项 应是数组"
    );
}

/// §13.d /api/self-check?target=bad 错误处理
#[tokio::test]
async fn self_check_无效target_返回错误() {
    let router = 构造应用().await;
    let (status, json) = get_json(&router, "/api/self-check?target=不存在的crate名xyz").await;
    assert!(
        json.get("error").is_some() || status.is_client_error() || status.is_server_error(),
        "无效 target 应有错误响应，实际 status={} json={:?}",
        status,
        json
    );
}

/// §11.f /api/rooms 返回 ≥ 9 个房间（含 9 府）
#[tokio::test]
async fn rooms_返回九府配置() {
    let router = 构造应用().await;
    let (status, json) = get_json(&router, "/api/rooms").await;
    assert_eq!(status, StatusCode::OK);
    let rooms = json["rooms"].as_array().expect("rooms 应为数组");
    assert!(rooms.len() >= 9, "rooms 应 ≥ 9 个，实际 {}", rooms.len());
    let ids: Vec<String> = rooms
        .iter()
        .filter_map(|v| v["id"].as_str().map(String::from))
        .collect();
    assert!(ids.contains(&"shihai_fu".to_string()), "应含 shihai_fu");
    assert!(ids.contains(&"mingling_fu".to_string()), "应含 mingling_fu");
    assert!(ids.contains(&"zhengdao_fu".to_string()), "应含 zhengdao_fu");
}

/// §11.f /api/cards 返回 9 张卡片摘要
#[tokio::test]
async fn cards_返回九卡片摘要() {
    let router = 构造应用().await;
    let (status, json) = get_json(&router, "/api/cards").await;
    assert_eq!(status, StatusCode::OK);
    let cards = json["cards"].as_array().expect("cards 应为数组");
    assert_eq!(cards.len(), 9, "cards 应有 9 项，实际 {}", cards.len());
    let first = &cards[0];
    assert!(first["id"].is_string(), "卡片 id 应是字符串");
    assert!(first["摘要"].is_object(), "摘要应是对象");
}

/// §11.f /api/settings 无令牌被拒
#[tokio::test]
async fn settings_无令牌被拒() {
    let router = 构造应用().await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/settings")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"间隔":2000}"#))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert!(resp.status().is_success(), "settings 应 200");
    let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["状态"], "拒绝", "无令牌应被拒，实际 {:?}", json);
}
