use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
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

/// 定向回退请求体
#[derive(Deserialize)]
pub struct 定向回退请求 {
    pub 错误描述: String,
    /// 可选的建议根源层级；缺省时由追溯器按任务阶段文档纯规则判定
    #[serde(default)]
    pub 建议根源层级: Option<String>,
}

/// 定向回退响应
#[derive(Serialize)]
pub struct 定向回退响应 {
    pub 成功: bool,
    pub 回退到: String,
    pub 回退次数: u32,
    /// 连带召回（暂停）的受影响任务数
    #[serde(default)]
    pub 影响任务数: usize,
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

fn 解析层级(s: &str) -> Option<tc_task::五行层级> {
    match s {
        "木" => Some(tc_task::五行层级::木),
        "火" => Some(tc_task::五行层级::火),
        "土" => Some(tc_task::五行层级::土),
        "金" => Some(tc_task::五行层级::金),
        "水" => Some(tc_task::五行层级::水),
        _ => None,
    }
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

/// POST /api/board/{id}/rollback — 定向回退（提供建议根源层级则直接回退；否则追溯器按任务阶段文档纯规则判定）
pub async fn 看板定向回退(
    State(状态): State<数据服务状态>,
    Path(id): Path<u64>,
    Json(请求): Json<定向回退请求>,
) -> Result<Json<定向回退响应>, (StatusCode, String)> {
    let 建议 = 请求.建议根源层级.as_deref().and_then(解析层级);
    let (根源层级, 任务uuid) = {
        let board = 状态.任务看板.lock().expect("看板锁中毒");
        let 任务 = board
            .查询(id)
            .ok_or((StatusCode::NOT_FOUND, format!("任务 {id} 不存在")))?;
        let uuid = 任务.任务标识.任务id;
        let 层级 = match 建议 {
            Some(层级) => 层级,
            None => hm_agent::追溯器::新()
                .追溯(uuid, &请求.错误描述, 任务.设计文档.as_ref(), 任务.实现文档.as_ref(), &任务.description)
                .根源层级,
        };
        (层级, uuid)
    };
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    let (回退到, 次数) = board
        .定向回退(id, 根源层级, &请求.错误描述)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // 与驱动器内自动回退一致：沿依赖图召回受影响任务（依赖本任务的上游任务）
    let 召回器 = hm_agent::召回器::新();
    let 图 = board.构建依赖图();
    let 影响 = 召回器.影响分析(任务uuid, &图);
    let 事件们 = 召回器.执行召回(任务uuid, 影响, &请求.错误描述, &mut board);
    let 影响任务数 = 事件们.iter().map(|e| e.影响任务.len()).sum();
    Ok(Json(定向回退响应 {
        成功: true,
        回退到: format!("{回退到:?}"),
        回退次数: 次数,
        影响任务数,
    }))
}

/// POST /api/board/{id}/clean — 太乙金仙一键清理（前置交付核验门禁 + 承接+提交）
///
/// 清理前强制扫尾：扫尾执行者已装配时先执行交付证据核验，`通过=false`（漂移未兑现/多余新增/自检不过）
/// 则返回 400 并保持任务 `待清理`（`扫尾记录` 已写入）；`通过=true` 才承接+提交到 `清理完成`。
/// 扫尾执行者未装配时保持 v1.32 兼容行为（直接承接+提交，记录 None），不因缺执行者而阻断。
pub async fn 看板清理(
    State(状态): State<数据服务状态>,
    Path(id): Path<u64>,
) -> Result<Json<清理响应>, (StatusCode, String)> {
    // 前置强制门禁：已装配扫尾执行者则先核验交付证据，未通过拒绝清理
    if let Some(执行者) = 状态.扫尾执行者.as_ref() {
        let 记录 = 执行者.执行(id).map_err(|e| 映射扫尾错误(&e))?;
        if !记录.通过 {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("交付证据核验未通过，拒绝清理：未兑现 {} / 多余 {}。{}",
                    记录.未兑现数, 记录.多余数, 记录.说明),
            ));
        }
    }
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    board
        .承接任务(id, AgentRole::太乙金仙)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    board
        .提交任务(id, AgentRole::太乙金仙, TaskStatus::清理完成)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // 清理后的交付核验记录（未装配执行者的兼容模式为 None）
    let 记录 = board.查询(id).and_then(|t| t.扫尾记录.clone());
    Ok(Json(清理响应 { 任务id: id, 记录 }))
}

/// 清理响应：任务id + 清理前交付核验记录（未装配执行者兼容模式为 None）
#[derive(Serialize)]
pub struct 清理响应 {
    pub 任务id: u64,
    pub 记录: Option<tc_task::扫尾记录>,
}

/// 扫尾检查响应：交付证据链的机器核验报告
#[derive(Serialize)]
pub struct 扫尾检查响应 {
    pub 任务id: u64,
    pub 记录: tc_task::扫尾记录,
}

/// POST /api/board/{id}/sweep — 扫尾检查（太乙金仙清理后的交付核验）
///
/// 对比实现文档声明的代码变更与工作区实际文件，检测漂移（声明未兑现/多余新增）+ 自检，
/// 写入任务 `扫尾记录` 作为交付证据链；未装配执行者 → 503 fail-loud。
pub async fn 看板扫尾检查(
    State(状态): State<数据服务状态>,
    Path(id): Path<u64>,
) -> Result<Json<扫尾检查响应>, (StatusCode, String)> {
    let 执行者 = 状态
        .扫尾执行者
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "扫尾执行者未装配（需配置 run_dev_agent）".into()))?;
    let 执行者 = 执行者.clone();
    match 执行者.执行(id) {
        Ok(记录) => Ok(Json(扫尾检查响应 { 任务id: id, 记录 })),
        Err(e) => Err(映射扫尾错误(&e)),
    }
}

fn 映射扫尾错误(e: &hm_error::Error) -> (StatusCode, String) {
    use hm_error::Error;
    match e {
        Error::任务不存在(_) => (StatusCode::NOT_FOUND, e.to_string()),
        Error::Config(_) => (StatusCode::BAD_REQUEST, e.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// 澄清请求体：道祖对回退到木层任务的纠偏决策
#[derive(Deserialize)]
pub struct 澄清请求 {
    /// 澄清结论（需求纠偏/终止说明）
    pub 结论: String,
    /// true=澄清后重新进入设计；false=终止任务
    pub 继续: bool,
}

/// 澄清响应：目标任务推进后的最终状态
#[derive(Serialize)]
pub struct 澄清响应 {
    pub 任务id: u64,
    pub 新状态: String,
    pub 澄清记录: tc_task::澄清记录,
}

/// POST /api/board/{id}/clarify — 道祖澄清并推进木层回退任务
///
/// 定向回退到 `待道祖澄清`（需求偏差/回退超限）后，需用户扮演道祖给出澄清结论：
/// `继续=true` → 澄清中 → 待圣人设计（重新被驱动承接设计）；`继续=false` → 已取消。
/// 非 `待道祖澄清` 状态返回 400，状态不变。
pub async fn 看板澄清(
    State(状态): State<数据服务状态>,
    Path(id): Path<u64>,
    Json(请求): Json<澄清请求>,
) -> Result<Json<澄清响应>, (StatusCode, String)> {
    let 记录 = tc_task::澄清记录::新(
        hm_contract::当前时间戳(),
        请求.结论,
        请求.继续,
    );
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    let 新状态 = board
        .澄清并推进(id, 记录)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let 澄清记录 = board
        .查询(id)
        .and_then(|t| t.澄清记录.clone())
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "澄清推进成功但记录缺失".to_string()))?;
    Ok(Json(澄清响应 {
        任务id: id,
        新状态: format!("{新状态:?}"),
        澄清记录,
    }))
}