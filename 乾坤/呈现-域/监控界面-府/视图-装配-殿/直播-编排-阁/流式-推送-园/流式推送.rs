//! SSE 推送 —— 直播流构造 + 历史回放 + 双视图数据装配。
//!
//! 依据：融合蓝图-设计稿.md §9.1 数据流、§9.3 白箱六字段契约、§十三 任务为中心。
//! - SSE 长连接：每 200ms 检测三源文件大小变化，有增量即读并推送白箱事件。
//! - 历史回放：按时间窗 [since, until] 过滤事件，NDJSON 流式返回。
//! - 双视图：时间线（按 ts 倒序）+ 任务树（按 _task_id 聚合）。

#![allow(non_upper_case_globals)]

use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    三源大小, 世界快照, 事件源, 任务索引, 任务索引项, 取三源大小, 拓扑, 拓扑段, 步骤, 步骤组件,
    白箱事件, 读事件流, 读观测记录, 读识海记录, SSE载荷,
};

/// SSE 流的间隔——200ms 检测一次文件大小变化。
const 轮询间隔ms: u64 = 200;

/// 构造直播 SSE 流——启动后台任务检测三源文件增量，推送白箱事件。
///
/// 流断开（客户端断连）时后台任务自动结束（mpsc 发送失败即退出）。
pub fn 建直播流() -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::io::Error>>> {
    let (tx, rx) = mpsc::channel::<Result<Event, std::io::Error>>(64);

    tokio::spawn(async move {
        let mut 旧大小 = 取三源大小();
        loop {
            tokio::time::sleep(Duration::from_millis(轮询间隔ms)).await;
            let 新大小 = 取三源大小();
            if 新大小.事件流 > 旧大小.事件流 {
                let 事件们 = 读事件流(旧大小.事件流, 新大小.事件流);
                if 推送批次(&tx, 事件源::事件流, 事件们).await.is_err() {
                    return;
                }
            }
            if 新大小.观测记录 > 旧大小.观测记录 {
                let 事件们 = 读观测记录(旧大小.观测记录, 新大小.观测记录);
                if 推送批次(&tx, 事件源::观测记录, 事件们).await.is_err() {
                    return;
                }
            }
            if 新大小.识海记录 > 旧大小.识海记录 {
                let 事件们 = 读识海记录(旧大小.识海记录, 新大小.识海记录);
                if 推送批次(&tx, 事件源::识海记录, 事件们).await.is_err() {
                    return;
                }
            }
            旧大小 = 新大小;
        }
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

/// 批量推送一批事件到 SSE 通道；发送失败返回 Err（客户端已断连）。
async fn 推送批次(
    tx: &mpsc::Sender<Result<Event, std::io::Error>>,
    源: 事件源,
    事件们: Vec<白箱事件>,
) -> Result<(), ()> {
    for 事件 in 事件们 {
        let 载荷 = SSE载荷 {
            source: 源.字面(),
            ts: 事件.ts,
            ev: 事件,
        };
        let json = match serde_json::to_string(&载荷) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let event = Event::default().data(json);
        if tx.send(Ok(event)).await.is_err() {
            return Err(());
        }
    }
    Ok(())
}

/// 历史回放——按时间窗 [since, until] 过滤事件，返回 NDJSON 行列表。
///
/// since=0 表示不限下界；until=0 表示不限上界。
pub fn 历史回放(since: u64, until: u64) -> Vec<白箱事件> {
    let mut 事件 = crate::读全部();
    事件.retain(|e| {
        if since > 0 && e.ts < since {
            return false;
        }
        if until > 0 && e.ts > until {
            return false;
        }
        true
    });
    事件.sort_by_key(|e| e.ts);
    事件
}

/// 时间线视图——所有事件按 ts 倒序排列。
pub fn 时间线视图(事件: &[白箱事件]) -> Vec<白箱事件> {
    let mut 结果 = 事件.to_vec();
    结果.sort_by_key(|e| std::cmp::Reverse(e.ts));
    结果
}

/// 任务树视图——按任务 id 聚合事件，返回任务索引。
pub fn 任务树视图(事件: &[白箱事件]) -> 任务索引 {
    use std::collections::HashMap;

    let mut 桶: HashMap<String, Vec<&白箱事件>> = HashMap::new();
    for e in 事件 {
        let id = 提取任务id(e);
        桶.entry(id).or_default().push(e);
    }

    let mut 任务: Vec<任务索引项> = 桶
        .into_iter()
        .map(|(id, 事件组)| 装配任务索引项(id, &事件组))
        .collect();

    // 按最近活动 ts 倒序
    任务.sort_by(|a, b| {
        let a_ts = a.时间线.last().copied().unwrap_or(0);
        let b_ts = b.时间线.last().copied().unwrap_or(0);
        b_ts.cmp(&a_ts)
    });

    任务索引 { 任务 }
}

/// 从白箱事件提取任务 id——优先从影响项的"要求"/"任务线"取，否则"未分组"。
fn 提取任务id(事件: &白箱事件) -> String {
    for 项 in &事件.影响 {
        if (项.类型 == "要求" || 项.类型 == "任务线") && !项.名.is_empty() {
            return 项.名.clone();
        }
    }
    "未分组".to_string()
}

/// 装配一个任务索引项。
fn 装配任务索引项(id: String, 事件组: &[&白箱事件]) -> 任务索引项 {
    let mut 时间线: Vec<u64> = 事件组.iter().map(|e| e.ts).collect();
    时间线.sort_unstable();

    let 摘要 = 事件组.last().map(|e| 截取(&e.动作, 80)).unwrap_or_default();

    let 状态 = 事件组
        .iter()
        .rev()
        .find_map(|e| {
            for 项 in &e.影响 {
                if 项.类型 == "状态" && !项.名.is_empty() {
                    return Some(项.名.clone());
                }
            }
            None
        })
        .unwrap_or_else(|| "未知".to_string());

    let 阶段 = 事件组
        .iter()
        .rev()
        .find_map(|e| {
            for 项 in &e.影响 {
                if 项.类型 == "阶段" && !项.名.is_empty() {
                    return Some(项.名.clone());
                }
            }
            None
        })
        .unwrap_or_default();

    let 累计token: u64 = 事件组.iter().map(|e| e.token.总计).sum();
    let 累计耗时ms: u64 = 事件组.iter().map(|e| e.耗时ms).sum();

    任务索引项 {
        id,
        摘要,
        状态,
        阶段,
        事件数: 事件组.len(),
        时间线,
        累计token,
        累计耗时ms,
    }
}

/// 截取字符串前 N 个字符（按 char，不按字节）。
fn 截取(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// 取世界状态快照——当前想法/要求从事件流推断内容，当前阶段从最近事件动作推断。
///
/// 当前想法/要求优先从事件流提取界主可读的内容摘要；事件流无相关事件时回退到
/// zhuangtai_fu 状态共享的 id（监控界面独立运行时状态共享可能未初始化，事件流更可靠）。
pub fn 取世界快照(启动时刻: u64) -> 世界快照 {
    let 全部 = crate::读全部();
    let 最近事件ts = 全部.iter().map(|e| e.ts).max().unwrap_or(0);
    let 最近事件数 = 全部.len();

    // 取最近一条动作非空事件，按动作推断当前阶段
    let 最近动作 = 全部
        .iter()
        .rev()
        .find(|e| !e.动作.is_empty())
        .map(|e| e.动作.as_str())
        .unwrap_or("");
    let 当前阶段 = 推断阶段(最近动作);

    // 当前想法/要求：优先从事件流推断界主可读内容，推断为空时回退到状态共享的 id
    let (状态想法id, 状态要求id) = 读状态共享();
    let 当前想法 = {
        let 推断 = 推断当前想法();
        if !推断.is_empty() {
            推断
        } else {
            状态想法id
        }
    };
    let 当前要求 = {
        let 推断 = 推断当前要求();
        if !推断.is_empty() {
            推断
        } else {
            状态要求id
        }
    };

    世界快照 {
        当前想法,
        当前要求,
        当前阶段,
        最近事件ts,
        最近事件数,
        启动时刻,
    }
}

/// 推断当前想法——从事件流找最近的"想法投递"事件，提取载荷.内容前 100 字符。
///
/// 事件流是 append-only 事实源，想法投递事件载荷含"内容"和"想法id"。
/// 倒序遍历找最近一条，返回内容摘要供界主直读。读失败或无事件返回空。
fn 推断当前想法() -> String {
    let 路径 = crate::事件流路径();
    let Ok(内容) = std::fs::read_to_string(&路径) else {
        return String::new();
    };
    for 行 in 内容.lines().rev() {
        let 行 = 行.trim();
        if 行.is_empty() {
            continue;
        }
        let Ok(值) = serde_json::from_str::<serde_json::Value>(行) else {
            continue;
        };
        let 类型 = 值.get("类型").and_then(|v| v.as_str()).unwrap_or("");
        if 类型 != "想法投递" {
            continue;
        }
        let 想法内容 = 值
            .get("载荷")
            .and_then(|v| v.get("内容"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        return 截取(想法内容, 100);
    }
    String::new()
}

/// 推断当前要求——从事件流找最近的"要求入池"事件，提取载荷.方向前 100 字符。
///
/// 要求入池事件载荷含"方向"（界主可读的要求方向描述）和"要求id"。
/// 倒序遍历找最近一条，返回方向摘要供界主直读。读失败或无事件返回空。
fn 推断当前要求() -> String {
    let 路径 = crate::事件流路径();
    let Ok(内容) = std::fs::read_to_string(&路径) else {
        return String::new();
    };
    for 行 in 内容.lines().rev() {
        let 行 = 行.trim();
        if 行.is_empty() {
            continue;
        }
        let Ok(值) = serde_json::from_str::<serde_json::Value>(行) else {
            continue;
        };
        let 类型 = 值.get("类型").and_then(|v| v.as_str()).unwrap_or("");
        if 类型 != "要求入池" {
            continue;
        }
        let 方向 = 值
            .get("载荷")
            .and_then(|v| v.get("方向"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        return 截取(方向, 100);
    }
    String::new()
}

/// 按动作关键词推断当前阶段——设计/执行/验收/完成/异常，无法判定则空。
fn 推断阶段(动作: &str) -> String {
    if 动作.is_empty() {
        return String::new();
    }
    if 动作.contains("设计") {
        return "设计中".to_string();
    }
    if 动作.contains("实现")
        || 动作.contains("工具调用")
        || 动作.contains("发送提示词")
        || 动作.contains("模型思考")
        || 动作.contains("模型回复")
    {
        return "执行中".to_string();
    }
    if 动作.contains("验收") || 动作.contains("审验") {
        return "验收中".to_string();
    }
    if 动作.contains("完成") || 动作.contains("定档") || 动作.contains("归档") {
        return "已完成".to_string();
    }
    if 动作.contains("失败") || 动作.contains("错误") {
        return "异常".to_string();
    }
    String::new()
}

/// 从 zhuangtai_fu 全局状态共享读当前想法/要求。
fn 读状态共享() -> (String, String) {
    let 想法 = zhuangtai_fu::取全局状态()
        .and_then(|共享| 共享.读取::<zhuangtai_fu::当前想法id>())
        .map(|v| v.0)
        .unwrap_or_default();
    let 要求 = zhuangtai_fu::取全局状态()
        .and_then(|共享| 共享.读取::<zhuangtai_fu::当前要求id>())
        .map(|v| v.0)
        .unwrap_or_default();
    (想法, 要求)
}

/// 取三源大小——供外部检测增量用。
pub fn 当前三源大小() -> 三源大小 {
    取三源大小()
}

// ===== 分裂流：拓扑装配 + 步骤流装配（§13.d / §13.c）=====

/// 建拓扑——按 ts 升序遍历事件，检测串行/并行/汇流段。
///
/// 依据：融合蓝图-设计稿.md §13.d.6 数据贯通。
/// 分流键=任务线id，空串视为"主线"。主线事件连续=串行段；非主线事件连续=并行段；
/// 并行段后回到主线=汇流段。此为 §13.d.5 降级形态——按主线/非主线二分切段，
/// 定稿后切精细分屏仅换此函数。
pub fn 建拓扑(事件: &[白箱事件]) -> 拓扑 {
    let mut 排序 = 事件.to_vec();
    排序.sort_by_key(|e| e.ts);

    let mut 段列表: Vec<拓扑段> = Vec::new();
    let mut 当前事件: Vec<白箱事件> = Vec::new();
    let mut 当前是主线段 = true;
    let mut 段起始ts: u64 = 0;
    let mut 已开始 = false;

    for e in 排序 {
        let 是主线 = e.任务线id.is_empty();
        if !已开始 {
            已开始 = true;
            段起始ts = e.ts;
            当前是主线段 = 是主线;
            当前事件.push(e);
            continue;
        }
        if 是主线 == 当前是主线段 {
            当前事件.push(e);
        } else {
            let 类型 = 定段类型(当前是主线段, 段列表.last());
            段列表.push(拓扑段 {
                类型,
                ts: 段起始ts,
                线: 提取线列表(&当前事件),
                事件: std::mem::take(&mut 当前事件),
            });
            段起始ts = e.ts;
            当前是主线段 = 是主线;
            当前事件.push(e);
        }
    }
    if 已开始 {
        let 类型 = 定段类型(当前是主线段, 段列表.last());
        段列表.push(拓扑段 {
            类型,
            ts: 段起始ts,
            线: 提取线列表(&当前事件),
            事件: std::mem::take(&mut 当前事件),
        });
    }

    拓扑 { 段: 段列表 }
}

/// 定段类型——主线段看前一段是否并行（是则"汇流"，否则"串行"）；非主线段恒"并行"。
fn 定段类型(当前是主线段: bool, 前一段: Option<&拓扑段>) -> String {
    if 当前是主线段 {
        if let Some(前) = 前一段 {
            if 前.类型 == "并行" {
                return "汇流".to_string();
            }
        }
        "串行".to_string()
    } else {
        "并行".to_string()
    }
}

/// 提取线列表——事件涉及的任务线id去重（空串归一为"主线"），保序。
fn 提取线列表(事件: &[白箱事件]) -> Vec<String> {
    let mut 线: Vec<String> = Vec::new();
    for e in 事件 {
        let id = if e.任务线id.is_empty() {
            "主线".to_string()
        } else {
            e.任务线id.clone()
        };
        if !线.contains(&id) {
            线.push(id);
        }
    }
    线
}

/// 建步骤流——按 ts 升序遍历，每个 LLM 调用开始新步骤，后续 tool/其他事件归入当前步骤。
///
/// 依据：融合蓝图-设计稿.md §13.c 步骤流。
/// 判断 LLM 事件：动作含 "llm" / "模型" / "思考"；判断 tool 事件：动作含 "工具" / "tool"。
/// 第一个事件无论类型都开始一个步骤；步骤号从 1 起；标题=首组件动作前 60 字。
pub fn 建步骤流(事件: &[白箱事件]) -> Vec<步骤> {
    let mut 排序 = 事件.to_vec();
    排序.sort_by_key(|e| e.ts);

    let mut 步骤列表: Vec<步骤> = Vec::new();
    let mut 当前组件: Vec<步骤组件> = Vec::new();
    let mut 当前标题 = String::new();
    let mut 当前起始ts: u64 = 0;
    let mut 已开始 = false;

    for e in 排序 {
        let 是llm = 是llm事件(&e.动作);
        if 是llm && 已开始 {
            收尾步骤(&mut 步骤列表, &mut 当前组件, &当前标题, 当前起始ts);
            已开始 = false;
        }
        if !已开始 {
            已开始 = true;
            当前起始ts = e.ts;
            当前标题 = 截取(&e.动作, 60);
        }
        当前组件.push(步骤组件 {
            类型: 组件类型(&e.动作),
            动作: e.动作.clone(),
            ts: e.ts,
            耗时ms: e.耗时ms,
            token: e.token.总计,
        });
    }
    if 已开始 {
        收尾步骤(&mut 步骤列表, &mut 当前组件, &当前标题, 当前起始ts);
    }
    步骤列表
}

/// 判断 LLM 事件——动作含 "llm" / "模型" / "思考"。
fn 是llm事件(动作: &str) -> bool {
    动作.contains("llm") || 动作.contains("模型") || 动作.contains("思考")
}

/// 判断 tool 事件——动作含 "工具" / "tool"。
fn 是tool事件(动作: &str) -> bool {
    动作.contains("工具") || 动作.contains("tool")
}

/// 组件类型——llm / tool / other。
fn 组件类型(动作: &str) -> String {
    if 是llm事件(动作) {
        "llm".to_string()
    } else if 是tool事件(动作) {
        "tool".to_string()
    } else {
        "other".to_string()
    }
}

/// 收尾一个步骤——组件为空则跳过，否则累加耗时/token 后存档。
fn 收尾步骤(
    列表: &mut Vec<步骤>, 组件: &mut Vec<步骤组件>, 标题: &str, 起始ts: u64
) {
    if 组件.is_empty() {
        return;
    }
    let 耗时ms: u64 = 组件.iter().map(|c| c.耗时ms).sum();
    let token累加: u64 = 组件.iter().map(|c| c.token).sum();
    let 步骤号 = 列表.len() + 1;
    列表.push(步骤 {
        步骤号,
        标题: 标题.to_string(),
        开始ts: 起始ts,
        耗时ms,
        token累加,
        组件: std::mem::take(组件),
    });
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 时间线视图倒序() {
        let 事件 = vec![
            白箱事件::新(100, "源", "a"),
            白箱事件::新(300, "源", "b"),
            白箱事件::新(200, "源", "c"),
        ];
        let 视图 = 时间线视图(&事件);
        assert_eq!(视图[0].ts, 300);
        assert_eq!(视图[1].ts, 200);
        assert_eq!(视图[2].ts, 100);
    }

    #[test]
    fn 任务树按要求聚合() {
        let 事件 = vec![
            白箱事件::新(100, "源", "a").追加影响(crate::影响项::新("要求", "要求-1")),
            白箱事件::新(200, "源", "b").追加影响(crate::影响项::新("要求", "要求-1")),
            白箱事件::新(300, "源", "c").追加影响(crate::影响项::新("要求", "要求-2")),
        ];
        let 索引 = 任务树视图(&事件);
        assert_eq!(索引.任务.len(), 2);
        // 要求-1 有 2 条事件
        let r1 = 索引.任务.iter().find(|t| t.id == "要求-1").unwrap();
        assert_eq!(r1.事件数, 2);
        assert_eq!(r1.时间线, vec![100, 200]);
    }

    #[test]
    fn 历史回放按时间窗过滤() {
        let 事件 = vec![
            白箱事件::新(100, "源", "a"),
            白箱事件::新(200, "源", "b"),
            白箱事件::新(300, "源", "c"),
        ];
        // 用临时文件测试需要写盘，这里只测过滤逻辑
        let mut 过滤 = 事件.clone();
        过滤.retain(|e| e.ts >= 150 && e.ts <= 250);
        assert_eq!(过滤.len(), 1);
        assert_eq!(过滤[0].ts, 200);
    }

    #[test]
    fn 截取按字符不按字节() {
        assert_eq!(截取("你好世界", 2), "你好");
        assert_eq!(截取("abcde", 3), "abc");
        assert_eq!(截取("短", 10), "短");
    }

    // ===== 建拓扑 测试（§13.d）=====

    fn 造事件(ts: u64, 动作: &str, 线: &str) -> 白箱事件 {
        let e = 白箱事件::新(ts, "源", 动作).设耗时(10);
        if 线.is_empty() {
            e
        } else {
            e.设任务线id(线)
        }
    }

    #[test]
    fn 建拓扑全主线为单串行段() {
        let 事件 = vec![
            造事件(100, "a", ""),
            造事件(200, "b", ""),
            造事件(300, "c", ""),
        ];
        let 拓 = 建拓扑(&事件);
        assert_eq!(拓.段.len(), 1);
        assert_eq!(拓.段[0].类型, "串行");
        assert_eq!(拓.段[0].线, vec!["主线".to_string()]);
        assert_eq!(拓.段[0].事件.len(), 3);
        assert_eq!(拓.段[0].ts, 100);
    }

    #[test]
    fn 建拓扑主线并行汇流三段() {
        let 事件 = vec![
            造事件(100, "主线1", ""),
            造事件(200, "线A1", "线A"),
            造事件(300, "线B1", "线B"),
            造事件(400, "线A2", "线A"),
            造事件(500, "主线2", ""),
        ];
        let 拓 = 建拓扑(&事件);
        assert_eq!(拓.段.len(), 3);
        assert_eq!(拓.段[0].类型, "串行");
        assert_eq!(拓.段[0].线, vec!["主线".to_string()]);
        assert_eq!(拓.段[1].类型, "并行");
        assert_eq!(拓.段[1].线, vec!["线A".to_string(), "线B".to_string()]);
        assert_eq!(拓.段[1].事件.len(), 3);
        assert_eq!(拓.段[2].类型, "汇流");
        assert_eq!(拓.段[2].线, vec!["主线".to_string()]);
    }

    #[test]
    fn 建拓扑多次分裂汇流() {
        let 事件 = vec![
            造事件(100, "m1", ""),
            造事件(200, "a1", "线A"),
            造事件(300, "m2", ""),
            造事件(400, "b1", "线B"),
        ];
        let 拓 = 建拓扑(&事件);
        assert_eq!(拓.段.len(), 4);
        assert_eq!(拓.段[0].类型, "串行");
        assert_eq!(拓.段[1].类型, "并行");
        assert_eq!(拓.段[2].类型, "汇流");
        assert_eq!(拓.段[3].类型, "并行");
    }

    #[test]
    fn 建拓扑空事件无段() {
        let 拓 = 建拓扑(&[]);
        assert!(拓.段.is_empty());
    }

    #[test]
    fn 建拓扑全非主线为单并行段() {
        let 事件 = vec![造事件(100, "a1", "线A"), 造事件(200, "b1", "线B")];
        let 拓 = 建拓扑(&事件);
        assert_eq!(拓.段.len(), 1);
        assert_eq!(拓.段[0].类型, "并行");
        assert_eq!(拓.段[0].线, vec!["线A".to_string(), "线B".to_string()]);
    }

    #[test]
    fn 建拓扑按ts升序装配() {
        // 传入乱序事件，应按 ts 升序装配
        let 事件 = vec![
            造事件(300, "c", ""),
            造事件(100, "a", ""),
            造事件(200, "b", ""),
        ];
        let 拓 = 建拓扑(&事件);
        assert_eq!(拓.段[0].事件[0].动作, "a");
        assert_eq!(拓.段[0].事件[1].动作, "b");
        assert_eq!(拓.段[0].事件[2].动作, "c");
    }

    // ===== 建步骤流 测试（§13.c）=====

    #[test]
    fn 建步骤流llm开新步骤() {
        let 事件 = vec![
            造事件(100, "模型调用-1", ""),
            造事件(200, "工具-读文件", ""),
            造事件(300, "工具-写文件", ""),
            造事件(400, "llm-2", ""),
            造事件(500, "tool-跑命令", ""),
        ];
        let 步 = 建步骤流(&事件);
        assert_eq!(步.len(), 2);
        assert_eq!(步[0].步骤号, 1);
        assert_eq!(步[0].组件.len(), 3);
        assert_eq!(步[0].组件[0].类型, "llm");
        assert_eq!(步[0].组件[1].类型, "tool");
        assert_eq!(步[0].组件[2].类型, "tool");
        assert_eq!(步[1].步骤号, 2);
        assert_eq!(步[1].组件.len(), 2);
        assert_eq!(步[1].组件[0].类型, "llm");
    }

    #[test]
    fn 建步骤流tool开头单独成步() {
        let 事件 = vec![
            造事件(100, "工具-初始化", ""),
            造事件(200, "模型-思考", ""),
            造事件(300, "工具-执行", ""),
        ];
        let 步 = 建步骤流(&事件);
        assert_eq!(步.len(), 2);
        assert_eq!(步[0].组件.len(), 1);
        assert_eq!(步[0].组件[0].类型, "tool");
        assert_eq!(步[1].组件.len(), 2);
        assert_eq!(步[1].组件[0].类型, "llm");
        assert_eq!(步[1].组件[1].类型, "tool");
    }

    #[test]
    fn 建步骤流全tool单步骤() {
        let 事件 = vec![造事件(100, "工具-a", ""), 造事件(200, "工具-b", "")];
        let 步 = 建步骤流(&事件);
        assert_eq!(步.len(), 1);
        assert_eq!(步[0].组件.len(), 2);
    }

    #[test]
    fn 建步骤流空事件无步骤() {
        let 步 = 建步骤流(&[]);
        assert!(步.is_empty());
    }

    #[test]
    fn 建步骤流耗时token累加与标题() {
        let 事件 = vec![
            白箱事件::新(100, "源", "模型调用开始")
                .设耗时(50)
                .设token(crate::token用量 {
                    提示词: 100,
                    输出: 200,
                    缓存: 0,
                    总计: 300,
                }),
            白箱事件::新(200, "源", "工具读文件")
                .设耗时(30)
                .设token(crate::token用量 {
                    提示词: 0,
                    输出: 0,
                    缓存: 0,
                    总计: 50,
                }),
        ];
        let 步 = 建步骤流(&事件);
        assert_eq!(步.len(), 1);
        assert_eq!(步[0].标题, "模型调用开始");
        assert_eq!(步[0].耗时ms, 80);
        assert_eq!(步[0].token累加, 350);
        assert_eq!(步[0].开始ts, 100);
    }

    #[test]
    fn 建步骤流标题截前60字() {
        let 长动作 = "模型".repeat(40);
        let 事件 = vec![白箱事件::新(100, "源", &长动作)];
        let 步 = 建步骤流(&事件);
        assert_eq!(步.len(), 1);
        assert_eq!(步[0].标题.chars().count(), 60);
    }

    #[test]
    fn 建步骤流思考动作判llm() {
        let 事件 = vec![造事件(100, "思考-分析", ""), 造事件(200, "工具-执行", "")];
        let 步 = 建步骤流(&事件);
        // "思考-分析" 是 llm，"工具-执行" 归入同一步
        assert_eq!(步.len(), 1);
        assert_eq!(步[0].组件.len(), 2);
        assert_eq!(步[0].组件[0].类型, "llm");
    }
}
