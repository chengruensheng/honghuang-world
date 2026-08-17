//! 观测探针-府（jiance_fu）—— 白箱可观测性（统一类型 · 交接处埋点 · 白箱还原）。
//!
//! 设计稿 §4.4。核心：**一个积和类型 `观测记录`** + 一个统一入口 `落`，
//! 在「命令↔角色↔LLM↔工具」的天然交接点发出，按 `关联` 贯穿还原任意任务执行链。
//!
//! - 集合：事件相关与业务逻辑同在本府，但任何生产 crate 只调一行 `jiance_fu::落(..)`。
//! - 可删：删本府 = 删 Cargo 成员 + 删各调用点，不影响其他（本府零上级依赖）。
//! - 可维护：改类型/落盘只动本府；可扩展：加交接点 = 加枚举 + 加一行。

use std::io::Write;
use std::path::PathBuf;

/// 观测域：信号类别。新增交接点在此加变体。
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum 观测域 {
    /// 角色→LLM 请求（提示词出站）
    提示词,
    /// LLM→角色 回复（含思考）
    回复思考,
    /// 角色→工具 调用（参数出站）
    工具调用,
    /// 工具→角色 返回（结果入站）
    工具返回,
    /// 跨角色 状态流转
    状态流转,
    /// 产物判定（构建/测试）
    产物判定,
}

/// 角色标识。
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum 观测角色 {
    界主, 鸿钧, 执行, 设计, 验收, 归因, 未知,
}

/// 关联标识：贯穿一次任务的执行链。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct 关联 {
    /// 任务线 id（可空）。
    pub 任务线: Option<String>,
    /// 要求 id（可空）。
    pub 要求: Option<String>,
    /// 轮次（可空）。
    pub 轮次: Option<usize>,
    /// 本交接点的调用/标识 id（可空，用于配对 请求↔回复、调用↔返回）。
    pub 标识: Option<String>,
}

impl 关联 {
    pub fn 新() -> 关联 { 关联 { 任务线: None, 要求: None, 轮次: None, 标识: None } }
    pub fn 任务线(mut self, v: &str) -> 关联 { self.任务线 = Some(v.to_string()); self }
    pub fn 要求(mut self, v: &str) -> 关联 { self.要求 = Some(v.to_string()); self }
    pub fn 轮次(mut self, v: usize) -> 关联 { self.轮次 = Some(v); self }
    pub fn 标识(mut self, v: impl Into<String>) -> 关联 { self.标识 = Some(v.into()); self }
}

/// 载荷：按域类型化的正文。只采能反推「设计/决策是否生效」的信号，砍纯噪音。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct 载荷 {
    /// 正文（提示词全文 / 思考 / 回复 / 工具参数 / 工具结果 / 状态 / 意见 / 构建输出）。
    pub 内容: String,
    /// 结构化附加（如 {工具名, 退出码, 用量}），可空。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub 附加: Option<serde_json::Value>,
}

impl 载荷 {
    pub fn 文本(内容: impl Into<String>) -> 载荷 { 载荷 { 内容: 内容.into(), 附加: None } }
    pub fn 结构化(内容: impl Into<String>, 附加: serde_json::Value) -> 载荷 {
        载荷 { 内容: 内容.into(), 附加: Some(附加) }
    }
}

/// 统一观测记录。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct 观测记录 {
    pub 时间戳: u64,
    pub 域: 观测域,
    pub 接口: String,
    pub 角色: 观测角色,
    pub 载荷: 载荷,
    pub 关联: 关联,
}

/// 观测根目录（工作区根/.上下文/观测），默认 .上下文；可用环境覆盖。
fn 观测目录() -> PathBuf {
    if let Ok(根) = std::env::var("WORLD_WORKSPACE_ROOT") {
        PathBuf::from(根).join(".上下文").join("观测")
    } else {
        PathBuf::from(".上下文").join("观测")
    }
}

/// 单条正文封顶字符（防暴涨；超出截头保尾）。
const 正文_封顶: usize = 200_000;

fn 限长(文本: &str) -> String {
    let 字符们: Vec<char> = 文本.chars().collect();
    if 字符们.len() <= 正文_封顶 { return 文本.to_string(); }
    let 头: String = 字符们[..50_000].iter().collect();
    let 尾: String = 字符们.iter().rev().take(100).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{头}\n……（正文过长，共 {} 字符，已截头保尾）\n……尾部：{尾}", 字符们.len())
}

fn 当前毫秒() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 统一入库入口：append 一条 观测记录 到 `.上下文/观测/记录.jsonl`。
/// 落盘失败静默（可观测性不阻断业务）。
pub fn 落(记录: 观测记录) {
    let 目录 = 观测目录();
    if std::fs::create_dir_all(&目录).is_err() { return; }
    let 路径 = 目录.join("记录.jsonl");
    if let Ok(行) = serde_json::to_string(&记录) {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&路径) {
            let _ = writeln!(f, "{行}");
        }
    }
}

/// 便捷：角色→LLM 请求（提示词出站）。域=提示词。
pub fn 记请求(角色: 观测角色, 接口: &str, 提示词: &str, 关联: 关联) {
    落(观测记录 { 时间戳: 当前毫秒(), 域: 观测域::提示词, 接口: 接口.to_string(), 角色, 载荷: 载荷::文本(限长(提示词)), 关联 });
}

/// 便捷：LLM→角色 回复（含思考）。域=回复思考。
pub fn 记回复(角色: 观测角色, 接口: &str, 思考: &str, 回复: &str, 关联: 关联, 附加: Option<serde_json::Value>) {
    落(观测记录 {
        时间戳: 当前毫秒(), 域: 观测域::回复思考, 接口: 接口.to_string(), 角色,
        载荷: 造载荷(format!("【回复】\n{}\n【思考】\n{}", 回复, 思考), 附加),
        关联,
    });
}

/// 便捷：角色→工具 调用（参数出站）。域=工具调用。
pub fn 记工具调用(角色: 观测角色, 接口: &str, 工具: &str, 参数: &str, 关联: 关联) {
    落(观测记录 {
        时间戳: 当前毫秒(), 域: 观测域::工具调用, 接口: 接口.to_string(), 角色,
        载荷: 载荷::结构化(参数.to_string(), serde_json::json!({ "工具": 工具 })),
        关联,
    });
}

/// 便捷：工具→角色 返回（结果入站）。域=工具返回。仅「看类」工具记正文。
pub fn 记工具返回(角色: 观测角色, 接口: &str, 工具: &str, 结果: &str, 关联: 关联) {
    落(观测记录 {
        时间戳: 当前毫秒(), 域: 观测域::工具返回, 接口: 接口.to_string(), 角色,
        载荷: 载荷::结构化(限长(结果), serde_json::json!({ "工具": 工具 })),
        关联,
    });
}

/// 便捷：跨角色 状态流转。域=状态流转。
pub fn 记状态(角色: 观测角色, 接口: &str, 要求: &str, 状态: &str, 附加: Option<serde_json::Value>) {
    落(观测记录 {
        时间戳: 当前毫秒(), 域: 观测域::状态流转, 接口: 接口.to_string(), 角色,
        载荷: 造载荷(状态, 附加),
        关联: 关联::新().要求(要求),
    });
}

/// 便捷：产物判定（构建/测试输出）。域=产物判定。
pub fn 记构建(角色: 观测角色, 接口: &str, 命令: &str, 退出码: Option<i32>, 输出: &str, 关联: 关联) {
    落(观测记录 {
        时间戳: 当前毫秒(), 域: 观测域::产物判定, 接口: 接口.to_string(), 角色,
        载荷: 载荷::结构化(限长(输出), serde_json::json!({ "命令": 命令, "退出码": 退出码 })),
        关联,
    });
}

/// 按是否有附加构造载荷（内容 + 可选结构化）。
fn 造载荷(内容: impl Into<String>, 附加: Option<serde_json::Value>) -> 载荷 {
    match 附加 {
        Some(附加) => 载荷::结构化(内容, 附加),
        None => 载荷::文本(内容),
    }
}
