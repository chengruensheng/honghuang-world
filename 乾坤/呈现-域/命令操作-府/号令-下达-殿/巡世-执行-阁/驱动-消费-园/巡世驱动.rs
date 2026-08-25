//! 调度驱动（2026-08-24 重命名：原「巡世驱动」→「调度驱动」语义升级）。
//!
//! 流程：
//!   1. 读世界状态（候选池 + 世界状态上下文）
//!   2. 候选池空 → 调 调度_润色_园::润色候选注入 产 S11
//!   3. 档位优先选下一个候选（不是 sort by priority，而是按 本质档位 S0→S1..→S11 插队）
//!   4. 5 维评估 + 12 红线（接单门）
//!   5. 接受 → 构造想法 → 投递 → 候选出池落盘
//!
//! 依据：多智能体架构设计.md §19 本质驱动调度 v2（档位优先调度 + 智能接单门 + 润色注入）。

use crate::号令_下达_殿::想法_投递_阁::原子_入池_园::投递想法;
use crate::状态目录;
use rizhi_fu::{error, info, warn};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 调度驱动：档位优先 + 接单门 + 润色注入。
///
/// 命名：原 `巡世驱动`，因语义升级（含接单门+润色注入），改名 `调度驱动`。
/// 老接口 `巡世驱动` 保留为 alias（向后兼容号令分发）。
pub fn 调度驱动() -> String {
    // 步骤 0 (2026-08-25 接替错误处理 agent)：消费待修正-测试标识名单（§19.4.2.3 + §19.8 实施规范）
    let 名单结果 = 消费待修正名单();
    if let Some(报告) = 名单结果 {
        if 报告.待执行总数 > 0 {
            info!(
                待执行总数 = 报告.待执行总数,
                "调度驱动：识别待修正条目（占位阶段）"
            );
        }
    }

    let 状态目录路径 = 状态目录();
    let mut 状态 = match tianting_fu::确保世界状态初始化(&状态目录路径) {
        Ok(状态) => 状态,
        Err(错误) => {
            error!(错误 = %错误, "调度驱动：世界状态初始化失败");
            return format!("调度驱动失败：{错误}");
        }
    };

    // 步骤 1:候选池空 → 调 润色注入 产 S11 候选
    let mut 来源 = "巡世";
    if 状态.巡世候选池.is_empty() {
        info!("候选池空，调润色注入器");
        let 工作区根 = 状态目录路径.parent().unwrap_or(Path::new("."));
        let 润色 = tianting_fu::调度_润色_园::润色候选注入(工作区根);
        if 润色.候选们.is_empty() {
            return "调度驱动：候选池空 + 润色无产出，请先执行「巡世 扫描」".to_string();
        }
        // 把润色候选并入候选池（入池去重按目标）
        for c in 润色.候选们 {
            if !状态.巡世候选池.iter().any(|x| x.目标 == c.目标) {
                状态.巡世候选池.push(c);
            }
        }
        来源 = "润色";
    }

    // 步骤 2:档位优先选下一个（不是 sort by priority，而是按本质档位 S0→S1→...→S11 顺序）
    let 候选 = match 档位优先选下一个(&状态.巡世候选池) {
        Some(c) => c.clone(),
        None => return format!("调度驱动：候选池空（{来源}）"),
    };

    // 步骤 3:5 维评估 + 12 红线（接单门）
    let 涉及路径: Vec<&Path> = 候选
        .依据
        .split_whitespace()
        .filter(|s| s.contains('/') || s.contains('\\'))
        .map(Path::new)
        .collect();
    let 工作区根 = 状态目录路径
        .parent()
        .map(Path::new)
        .unwrap_or(Path::new("."));
    let 候选池已空 = 状态.巡世候选池.len() == 1; // 只剩本候选时算空
    let 决策 = tianting_fu::调度_接单_园::评估接单(
        &候选,
        候选池已空,
        &涉及路径,
        工作区根,
        None::<fn(&str) -> bool>,
    );
    if let tianting_fu::调度_接单_园::接单决策::拒绝(原因) = &决策 {
        warn!(候选目标 = %候选.目标, 拒绝原因 = %原因, "调度驱动：接单门拒绝");
        // 拒绝时把候选标"已归档"（避免反复拒）
        状态.巡世候选池.retain(|c| c.目标 != 候选.目标);
        if let Err(错误) = tianting_fu::写世界状态(&状态目录路径, &状态) {
            error!(错误 = %错误, "调度驱动：拒绝后落盘失败");
        }
        return format!("调度驱动拒绝：{}（已归档候选）", 原因);
    }

    // 步骤 4:构造想法 → 投递
    let 想法文本 = format!(
        "【{}候选】{}（本质档位：{:?}）\n依据：{}",
        来源, 候选.目标, 候选.本质档位, 候选.依据
    );
    info!(候选目标 = %候选.目标, 本质档位 = ?候选.本质档位, 来源 = %来源, "调度驱动：投递候选为想法");
    let 回执 = 投递想法(&想法文本);

    // 步骤 5:候选出池
    状态.巡世候选池.retain(|c| c.目标 != 候选.目标);
    if let Err(错误) = tianting_fu::写世界状态(&状态目录路径, &状态) {
        error!(错误 = %错误, "调度驱动：候选池落盘失败");
        return format!("调度驱动失败（候选池未落盘）：{错误}\n{回执}");
    }
    info!(剩余候选数 = 状态.巡世候选池.len(), "调度驱动完成并落盘");
    format!(
        "调度驱动完成\n来源：{}\n消费候选：{}\n本质档位：{:?}\n剩余候选：{} 条\n投递回执：\n{}",
        来源,
        候选.目标,
        候选.本质档位,
        状态.巡世候选池.len(),
        回执
    )
}

/// 档位优先选下一个：按本质档位 S0→S1→...→S11 顺序，每档位取第一个。
///
/// 这是 §19.3 调度算法的实现——S0 抢占，S1~S5 升序，S6~S10 队列里有就推，S11 队尾。
fn 档位优先选下一个(
    候选池: &[tianting_fu::巡世候选],
) -> Option<&tianting_fu::巡世候选> {
    // 2026-08-25 修：本质档位枚举是升序（S0=0 < S11=11），PartialOrd 升序比较。
    // min_by 找最小 = 数字最小 = 数字最接近 0 = 档位最高 = 紧急。
    // 旧 max_by 找最大 = S11 = 最不紧急 = 永远接补测试/补占位，与设计相反。
    候选池.iter().min_by(|a, b| a.本质档位.cmp(&b.本质档位))
}

/// 向后兼容：原 `巡世驱动` 命令分发仍可调。
pub fn 巡世驱动() -> String {
    调度驱动()
}

// 待修正-测试标识 条目（§19.9 入稿，§19.4.2.3 实施规范）。
// 注：mingling_fu 只依赖 serde_json（按 §16 红线 4 不加 serde derive），本类型
// 用 `serde_json::Value` 处理 jsonl：写入时 `json!({...})`，读取时
// `Value["字段"]` 提取。jsonl 字段对齐 §19.8：
// - 文件路径: str
// - 实体键: str（按实体键幂等去重）
// - 动作: str（加 `#[test]` | 去 `#[test]` | 加注释审）
// - LLM摘录: str
// - 时间戳: int
// - 执行状态: str（待执行 | 已成功 | 已失败 | 已跳过）

/// 待修正名单消费报告（§19.4.2.3 实施规范的返回值）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct 名单消费报告 {
    /// 成功执行数。
    pub 已成功: usize,
    /// 失败回滚数。
    pub 已失败: usize,
    /// 跳过数（人工已加 "已跳过" 标记的）。
    pub 已跳过: usize,
    /// 读取到的 待执行 条目总数。
    pub 待执行总数: usize,
}

/// 消费待修正-测试标识.jsonl 名单（§19.4.2.3 实施规范）。
///
/// 当前为最小骨架：读 .jsonl + 按 实体键 去重 + 报告统计。
/// TODO（待后续 commit 补全）：
/// - 每条事务：① 回滚垫备份 ② edit 修改 ③ cargo check 验证 ④ 失败回滚
/// - 全部跑完：清空名单 / 归档到 .上下文/归档/
pub fn 消费待修正名单() -> Option<名单消费报告> {
    let 工作区根 = match std::env::var("WORLD_WORKSPACE_ROOT") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => {
            // 退回 .上下文/状态/ 父目录
            状态目录().parent()?.to_path_buf()
        }
    };
    let 名单路径 = 工作区根
        .join(".上下文")
        .join("状态")
        .join("待修正-测试标识.jsonl");
    if !名单路径.exists() {
        return None;
    }
    let 内容 = match fs::read_to_string(&名单路径) {
        Ok(c) => c,
        Err(错误) => {
            error!(错误 = %错误, 路径 = %名单路径.display(), "调度驱动：读待修正名单失败");
            return None;
        }
    };
    // 按行解析 jsonl（容错：单行解析失败跳过该行）
    let mut 实体键表: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    for 行 in 内容.lines() {
        if 行.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(行) {
            Ok(值) => {
                // 字段提取（serde_json::Value 容错：缺字段返回空串/false）
                let 实体键 = 值["实体键"].as_str().unwrap_or("").to_string();
                let 执行状态 = 值["执行状态"].as_str().unwrap_or("");
                if 实体键.is_empty() {
                    warn!(行 = 行, "调度驱动：待修正名单行缺实体键，跳过");
                    continue;
                }
                if 执行状态 == "待执行" {
                    实体键表.insert(实体键, 值);
                }
            }
            Err(错误) => {
                warn!(错误 = %错误, 行 = 行, "调度驱动：待修正名单行解析失败，跳过");
            }
        }
    }
    let 待执行总数 = 实体键表.len();
    if 待执行总数 == 0 {
        return Some(名单消费报告::default());
    }
    // 报告（TODO：未来 commit 在此加回滚垫 + cargo check 事务）
    info!(
        待执行总数,
        路径 = %名单路径.display(),
        "调度驱动：识别待修正条目（占位阶段，实际修改待后续 commit）"
    );
    // TODO 占位：等错误处理 agent 完工后做
    // - 实体键去重已做
    // - 待后续 commit 接入回滚垫 + cargo check 验证
    // - 跑完清空名单 / 归档
    Some(名单消费报告 {
        待执行总数,
        ..Default::default()
    })
}

/// 追加待修正-测试标识条目到 .上下文/状态/待修正-测试标识.jsonl。
///
/// 实体键 = 文件路径（链头去重，由消费端 §14.20 reducer 处理）。
/// 当前为骨架（不接回滚垫 + cargo check，调用方在批量阶段处理）。
///
/// 参数：serde_json::Value（用 `serde_json::json!({...})` 宏构造）。
pub fn 追加待修正测试标识(条目: serde_json::Value) -> Result<(), String> {
    let 工作区根 = match std::env::var("WORLD_WORKSPACE_ROOT") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => {
            // 退回 .上下文/状态/ 父目录
            状态目录()
                .parent()
                .ok_or_else(|| "无法定位工作区根".to_string())?
                .to_path_buf()
        }
    };
    let 名单路径 = 工作区根
        .join(".上下文")
        .join("状态")
        .join("待修正-测试标识.jsonl");
    if let Some(父) = 名单路径.parent() {
        fs::create_dir_all(父).map_err(|e| format!("建目录失败: {e}"))?;
    }
    let 行 = 条目.to_string();
    let mut 文件 = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&名单路径)
        .map_err(|e| format!("打开名单文件失败: {e}"))?;
    writeln!(文件, "{行}").map_err(|e| format!("写失败: {e}"))?;
    Ok(())
}
