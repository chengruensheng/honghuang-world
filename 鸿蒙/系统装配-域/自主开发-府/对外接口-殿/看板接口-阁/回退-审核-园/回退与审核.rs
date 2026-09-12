//! 对外接口-殿/看板接口-阁/回退-审核-园：看板定向回退、清理、澄清与审核接口。

use axum::extract::Path;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tc_task::{AgentRole, TaskStatus, 审核来源, 驳回原因};
use crate::{接口状态, 追溯器, 召回器, 影响项};

/// 恢复方式：失败任务恢复时的策略选择
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum 恢复方式 {
    /// 回退当前任务并沿依赖图召回下游（现行为，默认）
    #[default]
    回退并召回,
    /// 仅回退当前任务，不召回下游
    仅回退,
    /// 仅写回退记录标记问题，不改变任务状态
    仅标记,
    /// 取消本次恢复，不做任何变更
    取消,
}

/// 定向回退请求体
#[derive(Deserialize, Default)]
pub struct 定向回退请求 {
    pub 错误描述: String,
    /// 可选的建议根源层级；缺省时由追溯器按任务阶段文档纯规则判定
    #[serde(default)]
    pub 建议根源层级: Option<String>,
    /// 恢复方式（默认「回退并召回」保持现行为）
    #[serde(default)]
    pub 恢复方式: 恢复方式,
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

/// 驳回原因字符串 → 枚举（与 tc-task 审核记录驳回原因枚举同源）
fn 解析驳回原因(s: &str) -> Option<驳回原因> {
    match s {
        "需求不清" => Some(驳回原因::需求不清),
        "设计不符" => Some(驳回原因::设计不符),
        "实现错误" => Some(驳回原因::实现错误),
        "测试不足" => Some(驳回原因::测试不足),
        "产出不完整" => Some(驳回原因::产出不完整),
        "扩大范围" => Some(驳回原因::扩大范围),
        "缩小范围" => Some(驳回原因::缩小范围),
        _ => None,
    }
}

/// POST /api/board/{id}/rollback — 定向回退（按「恢复方式」四选一执行）
///
/// 恢复方式：
/// - 回退并召回（默认）：定向回退当前任务 + 沿依赖图召回上游受影响任务
/// - 仅回退：仅定向回退当前任务，不召回下游
/// - 仅标记：仅写回退记录标记问题，不改变任务状态
/// - 取消：不做任何变更
/// 提供建议根源层级则直接回退；否则追溯器按任务阶段文档纯规则判定。
pub async fn 看板定向回退<M: Send + Sync + 'static>(
    状态: 接口状态<M>,
    Path(id): Path<u64>,
    Json(请求): Json<定向回退请求>,
) -> Result<Json<定向回退响应>, (StatusCode, String)> {
    let 任务uuid = {
        let board = 状态.任务看板.lock().expect("看板锁中毒");
        board
            .查询(id)
            .map(|t| t.任务标识.任务id)
            .ok_or((StatusCode::NOT_FOUND, format!("任务 {id} 不存在")))?
    };
    // 取消：不做任何变更（用户显式放弃本次恢复）
    if 请求.恢复方式 == 恢复方式::取消 {
        return Ok(Json(定向回退响应 {
            成功: true,
            回退到: "未变更".into(),
            回退次数: 0,
            影响任务数: 0,
        }));
    }
    let 建议 = 请求.建议根源层级.as_deref().and_then(解析层级);
    let 根源层级 = {
        let board = 状态.任务看板.lock().expect("看板锁中毒");
        let 任务 = board
            .查询(id)
            .ok_or((StatusCode::NOT_FOUND, format!("任务 {id} 不存在")))?;
        match 建议 {
            Some(层级) => 层级,
            None => 追溯器::新()
                .追溯(任务uuid, &请求.错误描述, 任务.设计文档.as_ref(), 任务.实现文档.as_ref(), &任务.description)
                .根源层级,
        }
    };
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    match 请求.恢复方式 {
        恢复方式::仅标记 => {
            let 次数 = board
                .标记回退(id, 根源层级, &请求.错误描述)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            Ok(Json(定向回退响应 {
                成功: true,
                回退到: 根源层级.名().to_string(),
                回退次数: 次数,
                影响任务数: 0,
            }))
        }
        恢复方式::仅回退 => {
            let (回退到, 次数) = board
                .定向回退(id, 根源层级, &请求.错误描述)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            Ok(Json(定向回退响应 {
                成功: true,
                回退到: format!("{回退到:?}"),
                回退次数: 次数,
                影响任务数: 0,
            }))
        }
        恢复方式::回退并召回 => {
            let (回退到, 次数) = board
                .定向回退(id, 根源层级, &请求.错误描述)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            // 与驱动器内自动回退一致：沿依赖图召回受影响任务（依赖本任务的上游任务）
            let 召回器 = 召回器::新();
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
        恢复方式::取消 => unreachable!("取消已在入口短路"),
    }
}

/// 影响分析响应：回退任务上游受影响任务的结构化预览（纯只读）
#[derive(Serialize)]
pub struct 影响分析响应 {
    pub 任务id: u64,
    pub 影响任务: Vec<影响项>,
}

/// GET /api/board/{id}/impact — 影响范围预览（纯只读，不执行召回）
///
/// 给定任务 id，返回若对其定向回退，将连带召回的受影响任务清单（id/标题/当前状态/将变更为）。
/// 复用召回器「影响分析 + 目标召回状态」纯规则，不改变任何任务状态。
pub async fn 看板影响分析<M: Send + Sync + 'static>(
    状态: 接口状态<M>,
    Path(id): Path<u64>,
) -> Result<Json<影响分析响应>, (StatusCode, String)> {
    let board = 状态.任务看板.lock().expect("看板锁中毒");
    let 任务 = board
        .查询(id)
        .ok_or((StatusCode::NOT_FOUND, format!("任务 {id} 不存在")))?;
    let 任务uuid = 任务.任务标识.任务id;
    let 依赖图 = board.构建依赖图();
    let 影响任务 = 召回器::新().影响预览(任务uuid, &依赖图, &board);
    Ok(Json(影响分析响应 { 任务id: id, 影响任务 }))
}

/// POST /api/board/{id}/clean — 太乙金仙一键清理（前置交付核验门禁 + 承接+提交）
///
/// 清理前强制扫尾：扫尾执行者已装配时先执行交付证据核验，`通过=false`（漂移未兑现/多余新增/自检不过）
/// 则返回 400 并保持任务 `待清理`（`扫尾记录` 已写入）；`通过=true` 才承接+提交到 `清理完成`。
/// 扫尾执行者未装配时保持 v1.32 兼容行为（直接承接+提交，记录 None），不因缺执行者而阻断。
pub async fn 看板清理<M: Send + Sync + 'static>(
    状态: 接口状态<M>,
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
pub async fn 看板扫尾检查<M: Send + Sync + 'static>(
    状态: 接口状态<M>,
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
pub async fn 看板澄清<M: Send + Sync + 'static>(
    状态: 接口状态<M>,
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

/// 审核请求体：人工覆盖最终审核结论（通过/驳回）
#[derive(Deserialize)]
pub struct 审核请求 {
    /// 是否通过
    pub 通过: bool,
    /// 驳回原因（驳回时必填，通过时为 null/缺省）
    #[serde(default)]
    pub 驳回原因: Option<String>,
    /// 审核评语
    #[serde(default)]
    pub 评语: String,
}

/// 审核响应：目标任务的最终审核结论与推进后状态
#[derive(Serialize)]
pub struct 审核响应 {
    pub 任务id: u64,
    pub 新状态: String,
    pub 审核记录: tc_task::审核记录,
}

/// POST /api/board/{id}/review — 人工覆盖最终审核结论（人可看可不看）
///
/// 任务处于 `待人工验收` 时可经此接口人工覆盖 LLM 自动审核结论：
/// `通过=true` → 人工验收中 → 待清理；`通过=false` → 人工验收中 → 待修复 → 按驳回原因定向回退。
/// 非 `待人工验收` 状态返回 400，状态不变。
pub async fn 看板审核<M: Send + Sync + 'static>(
    状态: 接口状态<M>,
    Path(id): Path<u64>,
    Json(请求): Json<审核请求>,
) -> Result<Json<审核响应>, (StatusCode, String)> {
    let 原因 = 请求.驳回原因.as_deref().and_then(解析驳回原因);
    if 请求.通过 && 原因.is_some() {
        return Err((StatusCode::BAD_REQUEST, "通过时驳回原因必须为空".to_string()));
    }
    if !请求.通过 && 原因.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "驳回时必须提供驳回原因（需求不清/设计不符/实现错误/测试不足/产出不完整/扩大范围/缩小范围）".to_string(),
        ));
    }
    let 记录 = tc_task::审核记录::新(
        hm_contract::当前时间戳(),
        请求.通过,
        原因,
        请求.评语,
        审核来源::人工,
    );
    // 先取任务标识（用于驳回召回），再原子审核推进
    let 任务uuid = {
        let board = 状态.任务看板.lock().expect("看板锁中毒");
        board
            .查询(id)
            .map(|t| t.任务标识.任务id)
            .ok_or((StatusCode::NOT_FOUND, format!("任务 {id} 不存在")))?
    };
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    let (新状态, 回退次数) = board
        .审核并推进(id, 记录)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    // 驳回回退：沿依赖图召回受影响任务（与看板定向回退 handler 一致）
    if 回退次数.is_some() {
        let 评语 = board
            .查询(id)
            .and_then(|t| t.审核记录.as_ref())
            .map(|r| r.评语.clone())
            .unwrap_or_default();
        let 召回器 = 召回器::新();
        let 图 = board.构建依赖图();
        let 影响 = 召回器.影响分析(任务uuid, &图);
        let _事件们 = 召回器.执行召回(任务uuid, 影响, &评语, &mut board);
    }
    let 审核记录 = board
        .查询(id)
        .and_then(|t| t.审核记录.clone())
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "审核推进成功但记录缺失".to_string()))?;
    Ok(Json(审核响应 {
        任务id: id,
        新状态: format!("{新状态:?}"),
        审核记录,
    }))
}
