//! 路由挂载 —— axum 路由定义 + 启动 HTTP 服务函数。
//!
//! 依据：融合蓝图-设计稿.md §4.3 路由设计、§11.4.2 Rust HTTP 服务。
//! 8 个端点：主页 + 静态资源 + 快照 + 最近事件 + SSE 直播 + 回放 + 任务索引 + 健康检查。
//! 启动函数 `启动监控(端口)` 透出到 lib 根，供命令操作府或独立 bin 调用。

use std::time::SystemTime;

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;

use crate::{
    三源就绪, 三源就绪 as 三源, 任务树视图, 健康状态, 历史回放, 取世界快照, 建拓扑, 建时间线,
    建步骤流, 建直播流, 建轨迹列表, 建轨迹详情, 搜轨迹, 过滤轨迹,
};

/// 共享状态——启动时刻（毫秒），经 axum State 注入各 handler。
#[derive(Clone, Copy)]
struct 共享状态 {
    启动时刻: u64,
}

/// 当前毫秒（UNIX 纪元）。
fn 当前毫秒() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 构造监控界面路由——15 个端点（含分裂流拓扑/步骤流 + §13.f 轨迹表格白箱）。
pub fn 建路由() -> Router {
    let 状态 = 共享状态 {
        启动时刻: 当前毫秒(),
    };
    Router::new()
        .route("/", get(主页))
        .route("/trajectory.html", get(时序子页))
        .route("/starmap.html", get(星图子页))
        .route("/static/starmap.css", get(星图CSS))
        .route("/static/starmap.js", get(星图JS))
        .route("/static/style.css", get(样式))
        .route("/static/app.js", get(脚本))
        .route("/static/trajectory.css", get(时序CSS))
        .route("/static/trajectory.js", get(时序JS))
        .route("/api/snapshot", get(快照))
        .route("/api/events/recent", get(最近事件))
        .route("/api/events/stream", get(直播))
        .route("/api/replay", get(回放))
        .route("/api/tasks", get(任务))
        .route("/api/topology", get(拓扑视图))
        // §十三 道韵接入：候选池端点（英文 alias 兼容 PowerShell，中文路径给浏览器）
        .route("/api/daoyun", get(候选池))
        .route("/api/候选池", get(候选池))
        .route("/api/starmap", get(星图))
        .route("/api/lines/:id/steps", get(任务线步骤))
        .route("/api/health", get(健康))
        // §13.f.11 轨迹表格白箱端点
        .route("/api/trajectory", get(轨迹列表))
        .route("/api/trajectory/event/:id", get(轨迹详情端点))
        .route("/api/trajectory/search", get(轨迹搜索))
        .route("/api/trajectory/timeline", get(轨迹时间线))
        .route("/api/trajectory/stream", get(轨迹直播))
        // §十三.d 动态项目自检
        .route("/api/self-check", get(自检))
        .route("/api/self-check/targets", get(自检目标们))
        // §十三.e 自检历史
        .route("/api/self-check/history", get(自检历史))
        // §11.f 监控界面核心契约补齐：九卡片清单 + 写自己配置 + 卡片摘要
        .route("/api/rooms", get(房间清单))
        .route("/api/cards", get(卡片列表))
        .route("/api/settings", axum::routing::post(写配置))
        .with_state(状态)
}

/// 启动监控 HTTP 服务——透出到 lib 根，供外部调用。
///
/// 默认端口 8080。返回 Result 让 bin 入口能区分"绑定失败"与"服务异常"：
/// - 绑定失败 → `Err(String)`，调用方应 eprintln + 非零退码
/// - 服务运行中 → 持续监听 Ctrl+C（由 bin 入口的 select 处理）
///
/// 错误日志同时写 rizhi_fu（订阅器已初始化场景）与 eprintln（无订阅器场景）双通道。
pub async fn 启动监控(端口: u16) -> Result<(), String> {
    let 监听地址 = format!("0.0.0.0:{端口}");
    rizhi_fu::info!("监控界面启动于 {监听地址}");
    eprintln!("监控界面启动于 http://{监听地址}");

    let 监听器 = match tokio::net::TcpListener::bind(&监听地址).await {
        Ok(l) => l,
        Err(e) => {
            let 消息 = format!("绑定 {监听地址} 失败: {e}");
            rizhi_fu::error!("{}", 消息);
            eprintln!("错误：{消息}");
            return Err(消息);
        }
    };
    let 路由 = 建路由();
    if let Err(e) = axum::serve(监听器, 路由).await {
        let 消息 = format!("服务异常: {e}");
        rizhi_fu::error!("{}", 消息);
        eprintln!("错误：{消息}");
        return Err(消息);
    }
    Ok(())
}

// ===== handlers =====

/// GET / —— 主页 HTML。
async fn 主页() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::HTML_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::主页HTML,
    )
}

/// GET /static/style.css —— CSS 样式。
async fn 样式() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::CSS_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::样式CSS,
    )
}

/// GET /static/app.js —— 前端逻辑。
async fn 脚本() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::JS_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::脚本JS,
    )
}

/// GET /trajectory.html —— §13.f 时序·历史子页（对标 Chrome Network 面板）。
async fn 时序子页() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::HTML_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::时序HTML,
    )
}

/// GET /static/trajectory.css —— 时序子页 CSS。
async fn 时序CSS() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::CSS_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::时序CSS,
    )
}

/// GET /static/trajectory.js —— 时序子页 JS（表格行 + Turn 分组 + 思考折叠）。
async fn 时序JS() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::JS_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::时序JS,
    )
}

/// GET /api/snapshot —— 当前世界状态快照。
async fn 快照(State(状态): State<共享状态>) -> impl IntoResponse {
    let 快照 = 取世界快照(状态.启动时刻);
    Json(快照)
}

/// GET /api/events/recent?limit=200 —— 最近 N 条事件（三源合并，倒序）。
async fn 最近事件(Query(参数): Query<最近事件参数>) -> impl IntoResponse {
    let 条数 = 参数.limit.unwrap_or(200).min(1000);
    let 事件 = crate::读最近(条数);
    Json(事件)
}

#[derive(Deserialize)]
struct 最近事件参数 {
    limit: Option<usize>,
}

/// GET /api/events/stream —— SSE 直播长连接。
async fn 直播() -> impl IntoResponse {
    rizhi_fu::info!("SSE 直播连接建立");
    建直播流()
}

/// GET /api/replay?since=&until= —— 历史回放（按时间窗）。
async fn 回放(Query(参数): Query<回放参数>) -> impl IntoResponse {
    let 事件 = 历史回放(参数.since.unwrap_or(0), 参数.until.unwrap_or(0));
    Json(事件)
}

#[derive(Deserialize)]
struct 回放参数 {
    since: Option<u64>,
    until: Option<u64>,
}

/// §11.f 九卡片摘要 —— 按 monitor.rooms.json 抓 9 府关切字段。
/// §17 写回识海：抓完后调 写回识海_结构 把 9 卡片写到 shihai_fu 的「结构」格位（§8.1）。
async fn 卡片列表() -> impl IntoResponse {
    let 卡片们 = crate::数据_抓取_殿::抓全部卡片();
    // §17 写回识海（失败不阻塞响应 — §11.6.3 异常兼容）
    if let Err(e) = crate::数据_抓取_殿::写回识海_结构(&卡片们) {
        rizhi_fu::warn!(错误 = %e, "§17 写回识海 卡片失败");
    }
    axum::response::Json(serde_json::json!({ "cards": 卡片们 }))
}

/// §11.f 房间清单（九卡片）—— 读 monitor.rooms.json 配置园资产。
async fn 房间清单() -> impl IntoResponse {
    let 路径 = shihai_fu::工作区::定位()
        .根路径()
        .join("乾坤")
        .join("呈现-域")
        .join("监控界面-府")
        .join("monitor.rooms.json");
    let 内容 = std::fs::read_to_string(&路径).unwrap_or_else(|_| "{\"rooms\":[]}".to_string());
    let v: serde_json::Value =
        serde_json::from_str(&内容).unwrap_or(serde_json::json!({"rooms":[]}));
    axum::response::Json(v)
}

/// §11.f 写配置（写自己配置，需 AI 令牌）。
async fn 写配置(axum::Json(载荷): axum::Json<serde_json::Value>) -> axum::response::Response {
    use axum::response::IntoResponse;
    // 写命令须 AI 令牌（-t <令牌> 或环境变量 WORLD_AI_TOKEN）— §11.5.1 + §11.6.3 安全副作用
    let 令牌 = 载荷
        .get("令牌")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let 环境令牌 = std::env::var("WORLD_AI_TOKEN").ok();
    if 令牌.is_empty() && 环境令牌.is_none() {
        return axum::response::Json(serde_json::json!({
            "error": "写命令须 AI 令牌",
            "状态": "拒绝",
        }))
        .into_response();
    }
    let 路径 = shihai_fu::工作区::定位()
        .根路径()
        .join("乾坤")
        .join("呈现-域")
        .join("监控界面-府")
        .join("monitor.settings.json");
    let 新值 = serde_json::json!({
        "间隔": 载荷.get("间隔").and_then(|v| v.as_u64()).unwrap_or(1000),
        "主题": 载荷.get("主题").and_then(|v| v.as_str()).unwrap_or("默认"),
        "端口": 载荷.get("端口").and_then(|v| v.as_u64()).unwrap_or(8080),
    });
    if let Err(_e) = std::fs::write(
        &路径,
        serde_json::to_string_pretty(&新值).unwrap_or_default(),
    ) {
        return axum::response::Json(serde_json::json!({
            "error": "写配置失败",
            "状态": "失败",
        }))
        .into_response();
    }
    axum::response::Json(serde_json::json!({ "状态": "ok", "配置": 新值 })).into_response()
}

/// GET /api/tasks —— 任务索引（按 _task_id 聚合）。
async fn 任务() -> impl IntoResponse {
    let 全部 = crate::读全部();
    let 索引 = 任务树视图(&全部);
    Json(索引)
}

/// GET /api/topology —— 分裂流拓扑段列表（串行/并行/汇流）。
///
/// 依据：融合蓝图-设计稿.md §13.d.6。返回 `建拓扑(&读全部())`。
async fn 拓扑视图() -> impl IntoResponse {
    let 全部 = crate::读全部();
    let 拓 = 建拓扑(&全部);
    Json(拓)
}

/// GET /api/lines/:id/steps —— 单任务线的步骤流（§13.c 形态）。
///
/// 该任务线事件 = 读全部().filter(|e| e.任务线id == id)。
async fn 任务线步骤(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let 全部 = crate::读全部();
    let 该线: Vec<_> = 全部.into_iter().filter(|e| e.任务线id == id).collect();
    let 步 = 建步骤流(&该线);
    Json(步)
}

/// GET /api/health —— 健康检查。
async fn 健康(State(状态): State<共享状态>) -> impl IntoResponse {
    let 运行秒 = 当前毫秒().saturating_sub(状态.启动时刻) / 1000;
    let 就绪 = 三源就绪();
    let 状态字 = if 就绪.事件流 || 就绪.观测记录 || 就绪.识海记录 {
        "ok"
    } else {
        "degraded"
    };
    Json(健康状态 {
        状态: 状态字.to_string(),
        运行秒,
        三源就绪: 就绪,
    })
}

// ===== §13.f.11 轨迹表格白箱端点 =====

/// GET /api/trajectory?since=&until=&before=&limit=&turn= —— 轨迹事件列表。
///
/// 依据：融合蓝图-设计稿.md §13.f.11。返回 `[{序号, 轮次, 类型, 摘要, token, 耗时ms, 事件id}]`。
/// - `since`/`until`：时间窗（0 表示不限）
/// - `before`：向上翻页，返 `ts < before` 的最近 `limit` 条
/// - `limit`：条数上限（默认 200，最大 1000）
/// - `turn`：按轮次过滤（0 表示不限）
async fn 轨迹列表(Query(参数): Query<轨迹列表参数>) -> impl IntoResponse {
    let 条数 = 参数.limit.unwrap_or(200).min(1000);
    let 全部 = crate::读全部();
    let 过滤 = 过滤轨迹(
        &全部,
        参数.since.unwrap_or(0),
        参数.until.unwrap_or(0),
        参数.before.unwrap_or(0),
        条数,
        参数.turn.unwrap_or(0),
    );
    Json(建轨迹列表(&过滤))
}

#[derive(Deserialize)]
struct 轨迹列表参数 {
    since: Option<u64>,
    until: Option<u64>,
    before: Option<u64>,
    limit: Option<usize>,
    turn: Option<usize>,
}

/// GET /api/trajectory/event/{id} —— 单事件详情面板全量（§13.f.3 字段全集）。
///
/// `id` 为事件 ts 字符串。返回该事件的完整详情，含 inputDetail/outputDetail/thinkingDetail 等。
async fn 轨迹详情端点(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let 全部 = crate::读全部();
    // 按 ts 字符串匹配；ts 重复时返回第一条
    let 命中 = 全部.iter().find(|e| e.ts.to_string() == id);
    Json(命中.map(建轨迹详情))
}

/// GET /api/trajectory/search?q=&since=&until= —— 全文搜索（§13.f.5）。
///
/// 返回 `[{事件id, 高亮区间[]}]`。搜索范围：源+动作+证据+影响文本。
async fn 轨迹搜索(Query(参数): Query<轨迹搜索参数>) -> impl IntoResponse {
    let 关键词 = 参数.q.unwrap_or_default();
    let 全部 = crate::读全部();
    let 过滤: Vec<_> = 全部
        .iter()
        .filter(|e| {
            if let Some(since) = 参数.since {
                if e.ts < since {
                    return false;
                }
            }
            if let Some(until) = 参数.until {
                if e.ts > until {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();
    Json(搜轨迹(&过滤, &关键词))
}

#[derive(Deserialize)]
struct 轨迹搜索参数 {
    q: Option<String>,
    since: Option<u64>,
    until: Option<u64>,
}

/// GET /api/trajectory/timeline?mode=&since=&until= —— 时间线色块数据（§13.f.4）。
///
/// 返回 `[{序号, ts, 值, 类型}]`。模式：sequence/duration/time/actual。
async fn 轨迹时间线(Query(参数): Query<轨迹时间线参数>) -> impl IntoResponse {
    let 模式 = 参数.mode.as_deref().unwrap_or("sequence");
    let 全部 = crate::读全部();
    let 过滤: Vec<_> = 全部
        .iter()
        .filter(|e| {
            if let Some(since) = 参数.since {
                if e.ts < since {
                    return false;
                }
            }
            if let Some(until) = 参数.until {
                if e.ts > until {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();
    Json(建时间线(&过滤, 模式))
}

#[derive(Deserialize)]
struct 轨迹时间线参数 {
    mode: Option<String>,
    since: Option<u64>,
    until: Option<u64>,
}

/// GET /api/trajectory/stream —— SSE 直播（复用 §9.1 主链路）。
///
/// 与 `/api/events/stream` 同源，payload 加 `序号/轮次/类型` 由前端从白箱事件派生。
async fn 轨迹直播() -> impl IntoResponse {
    rizhi_fu::info!("轨迹 SSE 直播连接建立");
    建直播流()
}

// 抑制未使用警告——三源类型仅在健康检查中通过 三源就绪() 间接使用
#[allow(dead_code)]
#[allow(clippy::items_after_test_module)]
fn _三源类型引用() -> 三源 {
    三源 {
        事件流: false,
        观测记录: false,
        识海记录: false,
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod 测试 {
    use super::*;

    #[test]
    fn 路由构造不panic() {
        let _路由 = 建路由();
    }

    #[test]
    fn 当前毫秒非零() {
        let ms = 当前毫秒();
        assert!(ms > 0);
    }
}

/// GET /api/starmap —— 函数级调用图谱·星空视图（§13.f.10.3b）。
///
/// 从 shihai_fu::依赖图 投影为精简节点 + 边。
async fn 星图() -> impl IntoResponse {
    Json(crate::加载星图())
}

/// GET /starmap.html —— §13.f.10.3b 函数级调用图谱·星空视图。
async fn 星图子页() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::HTML_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::星图HTML,
    )
}

/// GET /static/starmap.css —— 星图子页 CSS。
async fn 星图CSS() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::CSS_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::星图CSS,
    )
}

/// GET /static/starmap.js —— 星图子页 JS（SVG 力导向 + 节点交互）。
async fn 星图JS() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, crate::JS_MIME),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        crate::星图JS,
    )
}

/// GET /api/候选池 —— 道韵违逆候选 + 法则违逆报告（实时呈现道韵扫描结果）。
///
/// 数据源：.上下文/状态/世界状态.jsonl。读最后一条世界状态，返回 巡世候选们 + 天道报告库末条。
async fn 候选池() -> impl IntoResponse {
    use std::sync::OnceLock;
    static 缓存: OnceLock<String> = OnceLock::new();
    let json = 缓存.get_or_init(|| {
        // 读 .上下文/状态/世界状态.jsonl 末条
        let 工作区 = shihai_fu::工作区::定位();
        let 路径 = 工作区.上下文目录().join("状态").join("世界状态.jsonl");
        if !路径.exists() {
            return r#"{"候选们":[],"最新报告":null}"#.to_string();
        }
        let 内容 = std::fs::read_to_string(&路径).unwrap_or_default();
        let 末条 = 内容.lines().rfind(|l| !l.trim().is_empty());
        match 末条 {
            Some(line) => {
                // 简化提取：返回最后一行原 JSON + 额外补一个简化视图
                // 前端可解析巡世候选们字段
                line.to_string()
            }
            None => r#"{"候选们":[],"最新报告":null}"#.to_string(),
        }
    });
    axum::response::Response::builder()
        .header("content-type", "application/json; charset=utf-8")
        .body(axum::body::Body::from(json.clone()))
        .unwrap()
}

// ============================================================================
// §十三.d 动态项目自检
// ============================================================================

use serde::Serialize;
use shihai_fu::{工作区, 扫描违逆_路径};
use std::path::Path as StdPath;

/// 目标解析结果：把入参 target 解析为绝对路径 + crate 名（如果有）。
#[derive(Debug, Clone, Serialize)]
struct 解析目标 {
    输入: String,
    路径: String,
    crate名: Option<String>,
    标签: String, // 显示在 UI
}

/// 解析 target 入参：
/// - 空 / "all" → 工作区根 + 标签"整个项目"
/// - crate 名（从 workspace members 找）→ crate 根 + 标签 crate 名
/// - 路径（相对工作区根或绝对路径）→ 路径 + 标签
/// - 都不是 → 错误
fn 解析目标(target: &str) -> Result<解析目标, String> {
    let 工作区 = 工作区::定位();
    let 根 = 工作区.根路径();
    if target.is_empty() || target == "all" {
        return Ok(解析目标 {
            输入: target.to_string(),
            路径: 根.to_string_lossy().to_string(),
            crate名: None,
            标签: "整个项目".to_string(),
        });
    }
    // 1) 尝试 crate 名匹配
    let 工作区_toml = std::fs::read_to_string(根.join("Cargo.toml")).unwrap_or_default();
    if let Some(name) = 提取crate成员(&工作区_toml, target) {
        let crate_根 = 根.join(&name);
        let 路径 = crate_根.to_string_lossy().to_string();
        if crate_根.is_dir() {
            return Ok(解析目标 {
                输入: target.to_string(),
                路径,
                crate名: Some(name.clone()),
                标签: name,
            });
        }
    }
    // 2) 尝试相对路径或绝对路径
    let 候选 = if target.contains(':') || target.starts_with('/') || target.contains('\\') {
        std::path::PathBuf::from(target)
    } else {
        根.join(target)
    };
    if !候选.is_dir() {
        return Err(format!("target 不是有效 crate 名或目录: {target}"));
    }
    let crate名 = extract_crate_from_path(&候选, 根);
    Ok(解析目标 {
        输入: target.to_string(),
        路径: 候选.to_string_lossy().to_string(),
        crate名,
        标签: 候选
            .strip_prefix(根)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| 候选.to_string_lossy().to_string()),
    })
}

/// 从工作区 Cargo.toml 提取 members 列表中匹配 name 的路径。
fn 提取crate成员(toml_text: &str, name: &str) -> Option<String> {
    let 搜索短 = if name.ends_with("_fu") {
        name.strip_suffix("_fu").unwrap_or(name).to_string()
    } else {
        name.to_string()
    };
    if let Some(start) = toml_text.find("members") {
        let 余 = &toml_text[start..];
        if let Some(开) = 余.find('[') {
            if let Some(关) = 余[开..].find(']') {
                let 列表 = &余[开 + 1..开 + 关];
                for 项 in 列表.split(',') {
                    let 项 = 项.trim().trim_matches('"').trim_matches('\'').to_string();
                    if 项 == name || 项.ends_with(name) || 项.ends_with(&搜索短) || 项 == 搜索短
                    {
                        return Some(项);
                    }
                }
            }
        }
    }
    None
}

/// 从路径提取 crate 名（路径最后一段的 -XX-府 格式）。
fn extract_crate_from_path(路径: &StdPath, 工作区根: &StdPath) -> Option<String> {
    let 名称 = 路径.file_name()?.to_string_lossy().to_string();
    if 名称.ends_with("-府")
        || 名称.ends_with("-殿")
        || 名称.ends_with("-阁")
        || 名称.ends_with("-园")
    {
        // 向上找 -府 祖先
        let mut 当前 = 路径.to_path_buf();
        while 当前.starts_with(工作区根) {
            if let Some(名) = 当前.file_name() {
                let 名 = 名.to_string_lossy().to_string();
                if 名.ends_with("-府") {
                    return Some(名);
                }
            }
            if !当前.pop() {
                break;
            }
        }
    }
    Some(名称)
}

/// 自检报告（§十三.d）。
#[derive(Debug, Clone, Serialize)]
struct 自检报告 {
    target: 解析目标,
    daoyun: Daoyun指标,
    workspace: Workspace指标,
    tests: Tests指标,
    docs: Docs指标,
    health: Health指标,
}

#[derive(Debug, Clone, Serialize)]
struct Daoyun指标 {
    violations: u64,
    warnings: u64,
    errors: u64,
    rules: Vec<规则条目>,
}

#[derive(Debug, Clone, Serialize)]
struct 规则条目 {
    类型: String,
    严重度: String,
    路径: String,
    描述: String,
}

#[derive(Debug, Clone, Serialize)]
struct Workspace指标 {
    files: u64,
    lines: u64,
    dirs: u64,
}

#[derive(Debug, Clone, Serialize)]
struct Tests指标 {
    count: u64,
    cfgs: u64,
}

#[derive(Debug, Clone, Serialize)]
struct Docs指标 {
    count: u64,
    public_count: u64,
    coverage_ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
struct Health指标 {
    score: u64,
    等级: String, // 绿/黄/红
    扣分项: Vec<String>,
}

/// 递归统计目录中的 .rs 文件数和总行数。
fn 扫workspace统计(
    根: &StdPath,
    已访问: &mut std::collections::HashSet<std::path::PathBuf>,
) -> (u64, u64, u64) {
    if !根.is_dir() {
        return (0, 0, 0);
    }
    let canonical = match 根.canonicalize() {
        Ok(p) => p,
        Err(_) => return (0, 0, 0),
    };
    if !已访问.insert(canonical) {
        return (0, 0, 0);
    } // 防循环

    let mut 文件数 = 0u64;
    let mut 行数 = 0u64;
    let mut 目录数 = 0u64;
    let Ok(entries) = std::fs::read_dir(根) else {
        return (0, 0, 0);
    };
    for entry in entries.flatten() {
        let 路径 = entry.path();
        let 名 = entry.file_name().to_string_lossy().to_string();
        // 跳过构建产物/依赖/临时目录
        if 名 == "target"
            || 名 == "node_modules"
            || 名 == "道果树"
            || 名 == ".git"
            || 名 == "临时文件夹"
            || 名.starts_with(".上下文")
        {
            continue;
        }
        if 路径.is_dir() {
            目录数 += 1;
            let (f, l, d) = 扫workspace统计(&路径, 已访问);
            文件数 += f;
            行数 += l;
            目录数 += d;
        } else if 路径.extension().map(|e| e == "rs").unwrap_or(false) {
            文件数 += 1;
            if let Ok(内容) = std::fs::read_to_string(&路径) {
                行数 += 内容.lines().count() as u64;
            }
        }
    }
    (文件数, 行数, 目录数)
}

/// 统计 .rs 文件中的 #[test] / #[cfg(test)] / /// 文档注释 + pub 公开 API。
fn 扫代码指标(根: &StdPath) -> (Tests指标, Docs指标) {
    let mut tests_count = 0u64;
    let mut cfgs_count = 0u64;
    let mut docs_count = 0u64;
    let mut public_count = 0u64;

    fn visit(路径: &StdPath, tests: &mut u64, cfgs: &mut u64, docs: &mut u64, publics: &mut u64) {
        if 路径.is_dir() {
            let 名 = 路径
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if 名 == "target"
                || 名 == "node_modules"
                || 名 == "道果树"
                || 名 == ".git"
                || 名 == "临时文件夹"
                || 名.starts_with(".上下文")
            {
                return;
            }
            if let Ok(entries) = std::fs::read_dir(路径) {
                for entry in entries.flatten() {
                    visit(&entry.path(), tests, cfgs, docs, publics);
                }
            }
        } else if 路径.extension().map(|e| e == "rs").unwrap_or(false) {
            if let Ok(内容) = std::fs::read_to_string(路径) {
                for line in 内容.lines() {
                    let t = line.trim_start();
                    if t.starts_with("///") || t.starts_with("//!") {
                        *docs += 1;
                    }
                    if line.contains("#[test]") || line.contains("#[test(") {
                        *tests += 1;
                    }
                    if line.contains("#[cfg(test)]") {
                        *cfgs += 1;
                    }
                    if line.contains("pub fn ")
                        || line.contains("pub struct ")
                        || line.contains("pub enum ")
                        || line.contains("pub trait ")
                        || line.contains("pub type ")
                    {
                        *publics += 1;
                    }
                }
            }
        }
    }

    visit(
        根,
        &mut tests_count,
        &mut cfgs_count,
        &mut docs_count,
        &mut public_count,
    );

    let 比率 = if public_count > 0 {
        docs_count as f64 / public_count as f64
    } else {
        0.0
    };
    (
        Tests指标 {
            count: tests_count,
            cfgs: cfgs_count,
        },
        Docs指标 {
            count: docs_count,
            public_count,
            coverage_ratio: (比率 * 100.0).round() / 100.0,
        },
    )
}

/// 计算健康分（0-100）。
fn 计算健康分(
    报告: &shihai_fu::违逆报告, docs_ratio: f64, tests_count: u64
) -> Health指标 {
    let mut 分 = 100i64;
    let mut 扣分项 = Vec::new();
    // 错误违逆 -20
    if 报告.错误数 > 0 {
        let 扣 = (报告.错误数 as i64) * 20;
        分 -= 扣;
        扣分项.push(format!("{} 个错误级违逆 -{} 分", 报告.错误数, 扣));
    }
    // 警告违逆 -5
    if 报告.警告数 > 0 {
        let 扣 = (报告.警告数 as i64) * 5;
        分 -= 扣.min(40);
        扣分项.push(format!("{} 个警告级违逆 -{} 分", 报告.警告数, 扣.min(40)));
    }
    // 文档覆盖率 <30%
    if docs_ratio < 0.3 {
        分 -= 10;
        扣分项.push(format!(
            "文档覆盖率 {:.0}% < 30% -10 分",
            docs_ratio * 100.0
        ));
    }
    // 测试数 == 0
    if tests_count == 0 {
        分 -= 15;
        扣分项.push("无测试 -15 分".to_string());
    }
    let 分 = 分.max(0) as u64;
    let 等级 = if 分 >= 80 {
        "绿".to_string()
    } else if 分 >= 50 {
        "黄".to_string()
    } else {
        "红".to_string()
    };
    Health指标 {
        score: 分,
        等级,
        扣分项,
    }
}

/// GET /api/self-check?target=X —— 动态项目自检报告。
async fn 自检(Query(参数): Query<自检参数>) -> impl IntoResponse {
    let target = 参数.target.clone().unwrap_or_default();
    let 解析 = match 解析目标(&target) {
        Ok(t) => t,
        Err(e) => {
            return axum::response::Json(serde_json::json!({
                "error": e,
                "target": target,
            }))
            .into_response();
        }
    };
    let 根 = std::path::PathBuf::from(解析.路径.as_str());
    let _工作区 = 工作区::新(&根);

    // 道韵违逆
    let 报告 = 扫描违逆_路径(&根);
    let daoyun = Daoyun指标 {
        violations: 报告.总数 as u64,
        warnings: 报告.警告数 as u64,
        errors: 报告.错误数 as u64,
        rules: 报告
            .条目们
            .iter()
            .map(|e| 规则条目 {
                类型: format!("{:?}", e.类型),
                严重度: format!("{:?}", e.严重度),
                路径: e.路径.clone(),
                描述: e.描述.clone(),
            })
            .collect(),
    };

    // workspace 统计
    let mut 已访问 = std::collections::HashSet::new();
    let (files, lines, dirs) = 扫workspace统计(&根, &mut 已访问);
    let workspace = Workspace指标 { files, lines, dirs };

    // tests + docs 统计
    let (tests, docs) = 扫代码指标(&根);
    let health = 计算健康分(&报告, docs.coverage_ratio, tests.count);

    let 报告_总 = 自检报告 {
        target: 解析,
        daoyun,
        workspace,
        tests,
        docs,
        health,
    };

    let 报告_json = serde_json::to_value(&报告_总).unwrap_or_default();

    // §十三.e 写入自检历史快照（append-only）
    if let Err(_e) = 追加自检历史(&报告_总) {
        // 写入失败不影响响应（用户能看到本次自检结果）
    }

    // §17 写回识海·结构格位（让监控界面的「看见」进入项目心智模型）
    if let Err(e) = crate::数据_抓取_殿::写回自检_结构(&报告_json.to_string()) {
        rizhi_fu::warn!(错误 = %e, "§17 写回识海·自检失败");
    }

    axum::response::Json(报告_json).into_response()
}

/// §十三.e 自检历史快照：append 到 .上下文/状态/自检历史.jsonl
fn 追加自检历史(报告: &自检报告) -> Result<(), String> {
    use std::io::Write;
    let dir = shihai_fu::工作区::定位().上下文目录().join("状态");
    std::fs::create_dir_all(&dir).map_err(|e| format!("建状态目录失败: {e}"))?;
    let 路径 = dir.join("自检历史.jsonl");
    let 快照 = serde_json::json!({
        "ts": shihai_fu::当前毫秒(),
        "target": 报告.target.输入,
        "target_label": 报告.target.标签,
        "score": 报告.health.score,
        "等级": 报告.health.等级,
        "violations": 报告.daoyun.violations,
        "errors": 报告.daoyun.errors,
        "warnings": 报告.daoyun.warnings,
        "tests_count": 报告.tests.count,
        "files": 报告.workspace.files,
        "lines": 报告.workspace.lines,
    });
    let 行 = serde_json::to_string(&快照).map_err(|e| format!("序列化失败: {e}"))?;
    let mut 文件 = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&路径)
        .map_err(|e| format!("打开自检历史失败: {e}"))?;
    writeln!(文件, "{行}").map_err(|e| format!("写入失败: {e}"))?;
    Ok(())
}

/// GET /api/self-check/history —— 最近 N 条自检快照（趋势图用）。
async fn 自检历史(Query(参数): Query<自检历史参数>) -> impl IntoResponse {
    let 限制 = 参数.限制.unwrap_or(50);
    let 路径 = shihai_fu::工作区::定位()
        .上下文目录()
        .join("状态")
        .join("自检历史.jsonl");
    let 内容 = std::fs::read_to_string(&路径).unwrap_or_default();
    let mut 快照们: Vec<serde_json::Value> = Vec::new();
    for line in 内容.lines().rev() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            快照们.push(v);
        }
    }
    快照们.truncate(限制);
    axum::response::Json(serde_json::json!({ "snapshots": 快照们 }))
}

#[derive(Deserialize)]
struct 自检历史参数 {
    限制: Option<usize>,
}

/// GET /api/self-check/targets —— 列出可选 target（workspace crates + 根）。
async fn 自检目标们() -> impl IntoResponse {
    let 工作区 = 工作区::定位();
    let 根 = 工作区.根路径();
    let cargo_toml = std::fs::read_to_string(根.join("Cargo.toml")).unwrap_or_default();

    let mut 列表 = vec![serde_json::json!({
        "target": "all",
        "标签": "整个项目",
        "路径": 根.to_string_lossy().to_string(),
        "crate": serde_json::Value::Null,
    })];

    if let Some(start) = cargo_toml.find("members") {
        let 余 = &cargo_toml[start..];
        if let Some(开) = 余.find('[') {
            if let Some(关) = 余[开..].find(']') {
                let 列表_str = &余[开 + 1..开 + 关];
                for 项 in 列表_str.split(',') {
                    let 项 = 项.trim().trim_matches('"').trim_matches('\'');
                    if !项.is_empty() {
                        let 路径 = 根.join(项);
                        if 路径.is_dir() {
                            // 提取 crate 名（路径最后一段）
                            let crate名 = 路径
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            列表.push(serde_json::json!({
                                "target": crate名,
                                "标签": crate名,
                                "路径": 路径.to_string_lossy().to_string(),
                                "crate": crate名,
                            }));
                        }
                    }
                }
            }
        }
    }

    axum::response::Json(serde_json::json!({ "targets": 列表 }))
}

#[derive(Deserialize)]
struct 自检参数 {
    target: Option<String>,
}

// =========================================================================
