//! 对外接口-殿/路由组装-阁/服务-路由-园：本府对外 HTTP 路由片段。
//!
//! 由启动入口收集后注入数据服务（数据服务不再反向依赖本府），
//! 路径与 JSON 结构与原数据服务实现逐字一致，前端零漂移。

use axum::routing::{get, post};
use axum::Router;
use crate::{
    开发服务状态,
    看板列表, 看板查询, 看板发布, 看板承接, 看板提交,
    看板清理, 看板定向回退, 看板扫尾检查, 看板澄清, 看板审核, 看板影响分析,
    受理开发任务接口, 停止执行, 更新工作区, 事件查询,
    道祖对话接口, 道祖对话流式接口, 道祖确认接口,
    看板驱动接口, 看板驱动到空闲接口, 看板驱动状态接口, 看板驱动事件接口, 看板驱动过程接口,
    看板驱动过程流接口, 看板驱动阶段流接口,
    看板驱动过程流_agui接口, 看板驱动阶段流_agui接口,
    会话清单接口, 会话回放接口, 会话恢复接口, 会话分叉接口,
    文件清单接口, 文件内容接口, 规则写入接口,
    工作区查询, 工作区设置,
};

/// 构建本府对外路由片段：看板 + 开发受理/驱动/会话 + 文件/工作区。
pub fn 路由片段<M: Send + Sync + 'static>(状态: 开发服务状态<M>) -> Router {
    Router::new()
        .route("/api/board", get(看板列表::<M>).post(看板发布::<M>))
        .route("/api/board/{id}", get(看板查询::<M>))
        .route("/api/board/{id}/accept", post(看板承接::<M>))
        .route("/api/board/{id}/submit", post(看板提交::<M>))
        .route("/api/board/{id}/clean", post(看板清理::<M>))
        .route("/api/board/{id}/rollback", post(看板定向回退::<M>))
        .route("/api/board/{id}/sweep", post(看板扫尾检查::<M>))
        .route("/api/board/{id}/clarify", post(看板澄清::<M>))
        .route("/api/board/{id}/review", post(看板审核::<M>))
        .route("/api/board/{id}/impact", get(看板影响分析::<M>))
        .route("/api/dev/agent", post(受理开发任务接口::<M>))
        .route("/api/dev/agent/stop", post(停止执行::<M>))
        .route("/api/dev/chat", post(道祖对话接口::<M>))
        .route("/api/dev/chat/stream", post(道祖对话流式接口::<M>))
        .route("/api/dev/chat/confirm", post(道祖确认接口::<M>))
        .route("/api/dev/workspace", post(更新工作区::<M>))
        .route("/api/dev/events", get(事件查询::<M>))
        .route("/api/dev/pilot", post(看板驱动接口::<M>))
        .route("/api/dev/pilot/drain", post(看板驱动到空闲接口::<M>))
        .route("/api/dev/pilot/status", get(看板驱动状态接口::<M>))
        .route("/api/dev/pilot/events", get(看板驱动事件接口::<M>))
        .route("/api/dev/pilot/process", get(看板驱动过程接口::<M>))
        .route("/api/dev/stream", get(看板驱动过程流接口::<M>))
        .route("/api/dev/stream/state", get(看板驱动阶段流接口::<M>))
        .route("/api/dev/stream/agui", get(看板驱动过程流_agui接口::<M>))
        .route("/api/dev/stream/agui/state", get(看板驱动阶段流_agui接口::<M>))
        .route("/api/dev/sessions", get(会话清单接口::<M>))
        .route("/api/dev/sessions/{id}", get(会话回放接口::<M>))
        .route("/api/dev/sessions/{id}/resume", post(会话恢复接口::<M>))
        .route("/api/dev/sessions/{id}/fork", post(会话分叉接口::<M>))
        .route("/api/files", get(文件清单接口::<M>))
        .route("/api/files/content", get(文件内容接口::<M>))
        .route("/api/rules/write", post(规则写入接口::<M>))
        .route("/api/workspace", get(工作区查询::<M>).post(工作区设置::<M>))
        .with_state(状态)
}
