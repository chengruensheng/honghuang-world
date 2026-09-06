use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use tc_task::{AgentRole, Task, TaskPriority, TaskScene, TaskStatus};
use crate::数据服务状态;

/// 看板任务列表查询参数（可选筛选）
#[derive(Deserialize, Default)]
pub struct 看板筛选参数 {
    pub status: Option<String>,
    pub role: Option<String>,
}

/// 发布任务请求体
#[derive(Deserialize)]
pub struct 发布任务请求 {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub scene: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
}

/// 承接任务请求体
#[derive(Deserialize)]
pub struct 承接任务请求 {
    pub role: String,
}

/// 提交任务请求体
#[derive(Deserialize)]
pub struct 提交任务请求 {
    pub role: String,
    pub next_status: String,
}

fn 解析角色(s: &str) -> Option<AgentRole> {
    match s {
        "道祖" => Some(AgentRole::道祖),
        "圣人" => Some(AgentRole::圣人),
        "大罗金仙" => Some(AgentRole::大罗金仙),
        "准圣" => Some(AgentRole::准圣),
        "太乙金仙" => Some(AgentRole::太乙金仙),
        _ => None,
    }
}

fn 解析状态(s: &str) -> Option<TaskStatus> {
    match s {
        "待受理" => Some(TaskStatus::待受理),
        "进行中" => Some(TaskStatus::进行中),
        "已完成" => Some(TaskStatus::已完成),
        "已取消" => Some(TaskStatus::已取消),
        "待圣人设计" => Some(TaskStatus::待圣人设计),
        "圣人设计中" => Some(TaskStatus::圣人设计中),
        "待大罗金仙实现" => Some(TaskStatus::待大罗金仙实现),
        "大罗金仙实现中" => Some(TaskStatus::大罗金仙实现中),
        "待准圣验收" => Some(TaskStatus::待准圣验收),
        "准圣验收中" => Some(TaskStatus::准圣验收中),
        "待修复" => Some(TaskStatus::待修复),
        "待道祖终审" => Some(TaskStatus::待道祖终审),
        "道祖终审中" => Some(TaskStatus::道祖终审中),
        "待清理" => Some(TaskStatus::待清理),
        "清理中" => Some(TaskStatus::清理中),
        "清理完成" => Some(TaskStatus::清理完成),
        _ => None,
    }
}

fn 解析场景(s: &str) -> Option<TaskScene> {
    TaskScene::解析(s)
}

fn 解析优先级(s: &str) -> Option<TaskPriority> {
    TaskPriority::解析(s)
}

/// GET /api/board — 看板任务列表（可选筛选）
pub async fn 看板列表(
    State(状态): State<数据服务状态>,
    Query(筛选): Query<看板筛选参数>,
) -> Json<Vec<Task>> {
    let board = 状态.任务看板.lock().expect("看板锁中毒");
    let status_filter = 筛选.status.as_deref().and_then(解析状态);
    let role_filter = 筛选.role.as_deref().and_then(解析角色);
    let 任务: Vec<Task> = board
        .筛选(status_filter, role_filter)
        .into_iter()
        .cloned()
        .collect();
    drop(board);
    Json(任务)
}

/// GET /api/board/{id} — 查询单个任务详情
pub async fn 看板查询(
    State(状态): State<数据服务状态>,
    Path(id): Path<u64>,
) -> Result<Json<Task>, StatusCode> {
    let board = 状态.任务看板.lock().expect("看板锁中毒");
    match board.查询(id) {
        Some(任务) => Ok(Json(任务.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// POST /api/board — 发布任务
pub async fn 看板发布(
    State(状态): State<数据服务状态>,
    Json(请求): Json<发布任务请求>,
) -> Result<Json<u64>, (StatusCode, String)> {
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    let mut task = tc_task::Task::新建(0, 请求.title, 请求.description, hm_contract::当前时间戳());
    task.status = TaskStatus::待圣人设计;
    task.发起人 = AgentRole::道祖;
    if let Some(s) = 请求.scene.as_deref().and_then(解析场景) {
        task.场景 = s;
    }
    if let Some(p) = 请求.priority.as_deref().and_then(解析优先级) {
        task.优先级 = p;
    }
    board
        .发布任务(task)
        .map(|id| {
            // 发布即驱动：看板驱动台就绪且空闲时自动驱动一轮（尽力而为，不影响发布结果）
            状态.看板驱动台.自动驱动一轮();
            Json(id)
        })
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// POST /api/board/{id}/accept — 承接任务
pub async fn 看板承接(
    State(状态): State<数据服务状态>,
    Path(id): Path<u64>,
    Json(请求): Json<承接任务请求>,
) -> Result<StatusCode, (StatusCode, String)> {
    let role = 解析角色(&请求.role)
        .ok_or((StatusCode::BAD_REQUEST, format!("未知角色: {}", 请求.role)))?;
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    board
        .承接任务(id, role)
        .map(|_| StatusCode::OK)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// POST /api/board/{id}/submit — 提交任务（流转到下一状态）
pub async fn 看板提交(
    State(状态): State<数据服务状态>,
    Path(id): Path<u64>,
    Json(请求): Json<提交任务请求>,
) -> Result<StatusCode, (StatusCode, String)> {
    let role = 解析角色(&请求.role)
        .ok_or((StatusCode::BAD_REQUEST, format!("未知角色: {}", 请求.role)))?;
    let next = 解析状态(&请求.next_status)
        .ok_or((StatusCode::BAD_REQUEST, format!("未知状态: {}", 请求.next_status)))?;
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    board
        .提交任务(id, role, next)
        .map(|_| StatusCode::OK)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

/// POST /api/board/{id}/clean — 太乙金仙一键清理（承接+提交）
pub async fn 看板清理(
    State(状态): State<数据服务状态>,
    Path(id): Path<u64>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    board
        .承接任务(id, AgentRole::太乙金仙)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    board
        .提交任务(id, AgentRole::太乙金仙, TaskStatus::清理完成)
        .map(|_| StatusCode::OK)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}