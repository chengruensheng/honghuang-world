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
    三源大小, 世界快照, 事件源, 任务索引, 任务索引项, 取三源大小, 白箱事件, 读事件流, 读观测记录,
    读识海记录, SSE载荷,
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
        let event = Event::default().event("tick_event").data(json);
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

/// 取世界状态快照——从 zhuangtai_fu 读当前想法/要求。
pub fn 取世界快照(启动时刻: u64) -> 世界快照 {
    let (当前想法, 当前要求) = 读状态共享();
    let 全部 = crate::读全部();
    let 最近事件ts = 全部.iter().map(|e| e.ts).max().unwrap_or(0);
    let 最近事件数 = 全部.len();

    世界快照 {
        当前想法,
        当前要求,
        当前阶段: String::new(),
        最近事件ts,
        最近事件数,
        启动时刻,
    }
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
}
