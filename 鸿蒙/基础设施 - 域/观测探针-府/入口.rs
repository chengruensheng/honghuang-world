//! 观测探针-府（jiance_fu）—— 白箱可观测性（统一类型 · 交接处埋点 · 白箱还原）。
//!
//! 设计稿 §4.4。核心：**一个积和类型 `观测记录`** + 一个统一入口 `落`，
//! 在「命令↔角色↔LLM↔工具」的天然交接点发出，按 `关联` 贯穿还原任意任务执行链。
//!
//! - 集合：事件相关与业务逻辑同在本府，但任何生产 crate 只调一行 `jiance_fu::落(..)`。
//! - 可删：删本府 = 删 Cargo 成员 + 删各调用点，不影响其他（本府零上级依赖）。
//! - 可维护：改类型/落盘只动本府；可扩展：加交接点 = 加枚举 + 加一行。

use std::io::Write;
use std::path::{Path, PathBuf};

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
    界主,
    鸿钧,
    执行,
    设计,
    验收,
    归因,
    未知,
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
    pub fn 新() -> 关联 {
        关联 {
            任务线: None,
            要求: None,
            轮次: None,
            标识: None,
        }
    }
    pub fn 任务线(mut self, v: &str) -> 关联 {
        self.任务线 = Some(v.to_string());
        self
    }
    pub fn 要求(mut self, v: &str) -> 关联 {
        self.要求 = Some(v.to_string());
        self
    }
    pub fn 轮次(mut self, v: usize) -> 关联 {
        self.轮次 = Some(v);
        self
    }
    pub fn 标识(mut self, v: impl Into<String>) -> 关联 {
        self.标识 = Some(v.into());
        self
    }
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
    pub fn 文本(内容: impl Into<String>) -> 载荷 {
        载荷 {
            内容: 内容.into(),
            附加: None,
        }
    }
    pub fn 结构化(内容: impl Into<String>, 附加: serde_json::Value) -> 载荷 {
        载荷 {
            内容: 内容.into(),
            附加: Some(附加),
        }
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

/// 是否记录完整观测内容（默认开）。从环境变量 `观测完整记录` 读取，设为 "关" 时只记摘要。
/// 摘要不带提示词/回复全文，仅保留前若干字符与总长，避免敏感内容落盘。
fn 完整记录开关() -> bool {
    match std::env::var("观测完整记录") {
        Ok(值) => 值.trim() != "关",
        Err(_) => true,
    }
}

/// 摘要封顶字符数（关闭完整记录时使用，远小于 `正文_封顶`）。
const 摘要_封顶: usize = 500;

/// 取摘要：超出 `摘要_封顶` 字符时截头并标注总长，未超则原样返回。
fn 摘要(文本: &str) -> String {
    let 字符数 = 文本.chars().count();
    if 字符数 <= 摘要_封顶 {
        return 文本.to_string();
    }
    let 头: String = 文本.chars().take(摘要_封顶).collect();
    format!("{头}……（共 {字符数} 字符，已截断为摘要）")
}

/// 按配置对载荷脱敏：完整记录开关关闭时仅保留内容摘要，结构化附加（工具名/退出码/用量等）照旧。
fn 脱敏载荷(载荷: 载荷) -> 载荷 {
    if 完整记录开关() {
        载荷
    } else {
        载荷 {
            内容: 摘要(&载荷.内容),
            附加: 载荷.附加,
        }
    }
}

/// 以 0o600 权限打开用于追加写入（Unix 下显式设权限，Windows 无此 API 走默认）。
fn 打开追加(路径: &Path) -> std::io::Result<std::fs::File> {
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut 选项 = std::fs::OpenOptions::new();
    选项.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        选项.mode(0o600);
    }
    选项.open(路径)
}

/// 单条正文封顶字符（防暴涨；超出截头保尾）。
const 正文_封顶: usize = 200_000;

fn 限长(文本: &str) -> String {
    let 字符们: Vec<char> = 文本.chars().collect();
    if 字符们.len() <= 正文_封顶 {
        return 文本.to_string();
    }
    let 头: String = 字符们[..50_000].iter().collect();
    let 尾: String = 字符们
        .iter()
        .rev()
        .take(100)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!(
        "{头}\n……（正文过长，共 {} 字符，已截头保尾）\n……尾部：{尾}",
        字符们.len()
    )
}

fn 当前毫秒() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 统一入库入口：append 一条 观测记录 到 `.上下文/观测/记录.jsonl`。
/// 落盘失败静默（可观测性不阻断业务）。完整内容受 `观测完整记录` 开关控制。
pub fn 落(记录: 观测记录) {
    let 记录 = 观测记录 {
        载荷: 脱敏载荷(记录.载荷),
        ..记录
    };
    let 目录 = 观测目录();
    if std::fs::create_dir_all(&目录).is_err() {
        return;
    }
    let 路径 = 目录.join("记录.jsonl");
    if let Ok(行) = serde_json::to_string(&记录) {
        if let Ok(mut f) = 打开追加(&路径) {
            let _ = writeln!(f, "{行}");
        }
    }
}

/// 便捷：角色→LLM 请求（提示词出站）。域=提示词。
pub fn 记请求(角色: 观测角色, 接口: &str, 提示词: &str, 关联: 关联) {
    落(观测记录 {
        时间戳: 当前毫秒(),
        域: 观测域::提示词,
        接口: 接口.to_string(),
        角色,
        载荷: 载荷::文本(限长(提示词)),
        关联,
    });
}

/// 便捷：LLM→角色 回复（含思考）。域=回复思考。
pub fn 记回复(
    角色: 观测角色,
    接口: &str,
    思考: &str,
    回复: &str,
    关联: 关联,
    附加: Option<serde_json::Value>,
) {
    落(观测记录 {
        时间戳: 当前毫秒(),
        域: 观测域::回复思考,
        接口: 接口.to_string(),
        角色,
        载荷: 造载荷(format!("【回复】\n{}\n【思考】\n{}", 回复, 思考), 附加),
        关联,
    });
}

/// 便捷：角色→工具 调用（参数出站）。域=工具调用。
pub fn 记工具调用(
    角色: 观测角色, 接口: &str, 工具: &str, 参数: &str, 关联: 关联
) {
    落(观测记录 {
        时间戳: 当前毫秒(),
        域: 观测域::工具调用,
        接口: 接口.to_string(),
        角色,
        载荷: 载荷::结构化(参数.to_string(), serde_json::json!({ "工具": 工具 })),
        关联,
    });
}

/// 便捷：工具→角色 返回（结果入站）。域=工具返回。仅「看类」工具记正文。
pub fn 记工具返回(
    角色: 观测角色, 接口: &str, 工具: &str, 结果: &str, 关联: 关联
) {
    落(观测记录 {
        时间戳: 当前毫秒(),
        域: 观测域::工具返回,
        接口: 接口.to_string(),
        角色,
        载荷: 载荷::结构化(限长(结果), serde_json::json!({ "工具": 工具 })),
        关联,
    });
}

/// 便捷：跨角色 状态流转。域=状态流转。
pub fn 记状态(
    角色: 观测角色,
    接口: &str,
    要求: &str,
    状态: &str,
    附加: Option<serde_json::Value>,
) {
    落(观测记录 {
        时间戳: 当前毫秒(),
        域: 观测域::状态流转,
        接口: 接口.to_string(),
        角色,
        载荷: 造载荷(状态, 附加),
        关联: 关联::新().要求(要求),
    });
}

/// 便捷：产物判定（构建/测试输出）。域=产物判定。
pub fn 记构建(
    角色: 观测角色,
    接口: &str,
    命令: &str,
    退出码: Option<i32>,
    输出: &str,
    关联: 关联,
) {
    落(观测记录 {
        时间戳: 当前毫秒(),
        域: 观测域::产物判定,
        接口: 接口.to_string(),
        角色,
        载荷: 载荷::结构化(
            限长(输出),
            serde_json::json!({ "命令": 命令, "退出码": 退出码 }),
        ),
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

/// 观测上下文的单档内容。
#[derive(Clone, Debug)]
pub struct 观测上下文档 {
    pub 角色: 观测角色,
    pub 任务线: Option<String>,
    pub 要求: Option<String>,
    pub 轮次: Option<u64>,
}

// 线程本地观测上下文栈：可嵌套（如 鸿钧 主循环 → 设计 子调用 → 鸿钧 终裁），
// 进入观测 push、守卫 drop 时 pop；当前观测 读栈顶。跨调用方签名不便改动时
// （如 moxing_fu 的模型调用）由调用方 进入观测 设置一次，下游埋点自动带 角色/任务线/要求/轮次。
thread_local! {
    static 观测上下文栈: std::cell::RefCell<Vec<观测上下文档>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 进入一段带观测上下文的调用栈：push 一档线程本地上下文，返回守卫在离开时 pop 恢复上层。
/// 用法：`let _守卫 = 进入观测(角色, 任务线, 要求, 轮次);`。可嵌套。
pub fn 进入观测(
    角色: 观测角色,
    任务线: Option<String>,
    要求: Option<String>,
    轮次: Option<u64>,
) -> impl Drop {
    struct 清理;
    impl Drop for 清理 {
        fn drop(&mut self) {
            观测上下文栈.with(|栈| {
                let mut 栈 = 栈.borrow_mut();
                栈.pop();
            });
        }
    }
    观测上下文栈.with(|栈| {
        栈.borrow_mut().push(观测上下文档 {
            角色,
            任务线,
            要求,
            轮次,
        });
    });
    清理
}

/// 读取当前线程观测上下文栈顶：返回 (角色, 关联)；无上下文则 (未知, 空关联)。
pub fn 当前观测() -> (观测角色, 关联) {
    let 顶 = 观测上下文栈.with(|栈| 栈.borrow().last().cloned());
    let Some(档) = 顶 else {
        return (观测角色::未知, 关联::新());
    };
    let mut 关联 = 关联::新();
    if let Some(任务线) = 档.任务线 {
        关联 = 关联.任务线(&任务线);
    }
    if let Some(要求) = 档.要求 {
        关联 = 关联.要求(&要求);
    }
    if let Some(轮次) = 档.轮次 {
        关联 = 关联.轮次(轮次 as usize);
    }
    (档.角色, 关联)
}

/// 读取当前线程观测上下文中的关联部分；无上下文则返回空关联。
pub fn 当前关联() -> 关联 {
    当前观测().1
}

#[cfg(test)]
mod 测试 {
    use super::*;

    /// 序列化形状契约：消费端（乾坤监控域 Python）按字面字段名解析，
    /// 这里的字段名/枚举名即对外 API，改动需同步消费端。
    #[test]
    fn 记录序列化字段名契约() {
        let 记录 = 观测记录 {
            时间戳: 1234,
            域: 观测域::提示词,
            接口: "模型连接-府::调用模型".to_string(),
            角色: 观测角色::执行,
            载荷: 载荷::结构化("你好", serde_json::json!({"模型": "m3"})),
            关联: 关联::新().任务线("要求-1-0").要求("要求-1").轮次(3),
        };
        let 文本 = serde_json::to_string(&记录).unwrap();
        let 值: serde_json::Value = serde_json::from_str(&文本).unwrap();
        assert_eq!(值["时间戳"], 1234);
        assert_eq!(值["域"], "提示词");
        assert_eq!(值["接口"], "模型连接-府::调用模型");
        assert_eq!(值["角色"], "执行");
        assert_eq!(值["载荷"]["内容"], "你好");
        assert_eq!(值["载荷"]["附加"]["模型"], "m3");
        assert_eq!(值["关联"]["任务线"], "要求-1-0");
        assert_eq!(值["关联"]["要求"], "要求-1");
        assert_eq!(值["关联"]["轮次"], 3);
    }

    #[test]
    fn 线程本地栈嵌套与恢复() {
        // 进入观测 push、守卫 drop 恢复上层：鸿钧 → 设计 → 回鸿钧。
        {
            let _a = 进入观测(观测角色::鸿钧, None, Some("要求-1".to_string()), None);
            {
                let _b = 进入观测(观测角色::设计, None, Some("要求-1".to_string()), None);
                assert_eq!(当前观测().0, 观测角色::设计);
                let (_, 关联) = 当前观测();
                assert_eq!(关联.要求.as_deref(), Some("要求-1"));
            }
            assert_eq!(当前观测().0, 观测角色::鸿钧, "drop 后恢复上层");
        }
        assert_eq!(当前观测().0, 观测角色::未知, "全部退出后为空");
        assert_eq!(当前关联().任务线, None);
    }

    #[test]
    fn 线程本地上下文带任务线轮次() {
        {
            let _g = 进入观测(
                观测角色::执行,
                Some("任务-9".to_string()),
                Some("要求-9".to_string()),
                Some(5),
            );
            let (角色, 关联) = 当前观测();
            assert_eq!(角色, 观测角色::执行);
            assert_eq!(关联.任务线.as_deref(), Some("任务-9"));
            assert_eq!(关联.轮次, Some(5));
            // 便捷函数等价
            assert_eq!(当前关联().要求.as_deref(), Some("要求-9"));
        }
    }

    #[test]
    fn 摘要_未超封顶原样返回() {
        assert_eq!(摘要("短文本"), "短文本");
        let 边界 = "甲".repeat(摘要_封顶);
        assert_eq!(摘要(&边界), 边界, "等于封顶不截断");
    }

    #[test]
    fn 摘要_超封顶截头标注总长() {
        let 长文本 = "甲".repeat(摘要_封顶 + 100);
        let 摘要结果 = 摘要(&长文本);
        assert!(
            摘要结果.starts_with(&"甲".repeat(摘要_封顶)),
            "摘要应保留封顶前原文"
        );
        assert!(摘要结果.contains("已截断为摘要"), "摘要应标注截断");
    }
}
