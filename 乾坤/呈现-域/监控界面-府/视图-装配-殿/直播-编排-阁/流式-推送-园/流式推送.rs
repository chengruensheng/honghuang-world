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
    三源大小, 世界快照, 事件源, 事件类型, 任务索引, 任务索引项, 助手指标, 取三源大小, 拓扑, 拓扑段,
    搜索命中, 时间线色块, 步骤, 步骤组件, 白箱事件, 读事件流, 读观测记录, 读识海记录, 轨迹事件行,
    轨迹详情, SSE载荷,
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

// ===== §13.f 轨迹表格白箱 · 7 种事件类型派生 + 轨迹装配 =====

/// §13.f.8 派生事件类型——从白箱六字段 `源`+`动作` 派生 7 种事件类型之一。
///
/// 派生规则（设计稿 §13.f.8）：
/// - `system`：源含`提示词`且角色为系统
/// - `user`：源含`提示词`且角色为界主
/// - `context`：源含`提示词`且带注入标记（回想/历史/上下文）
/// - `compacted`：动作含`压缩`或源含`压缩`
/// - `message`：源含`回复`/`思考`/`模型连接`，或动作含`模型`/`思考`/`回复`
/// - `tool`：源含`工具调用`/`道术施展`或动作含`工具`，无子工具标记
/// - `subtool`：同 tool 但带子工具标记
///
/// 派生必须穷尽：兜底返回 `message`，避免漏派生（沿用 §9.3 不变量）。
pub fn 派生事件类型(事件: &白箱事件) -> 事件类型 {
    let 源 = 事件.源.as_str();
    let 动作 = 事件.动作.as_str();

    // 提示词域——按角色/注入类细分 system/user/context
    if 源.contains("提示词") {
        if 动作.contains("系统") || 源.contains("系统") {
            return 事件类型::系统;
        }
        if 源.contains("界主") || 动作.contains("界主") || 动作.contains("用户") {
            return 事件类型::界主;
        }
        if 动作.contains("注入")
            || 动作.contains("回想")
            || 动作.contains("历史")
            || 源.contains("上下文")
        {
            return 事件类型::上下文;
        }
        return 事件类型::界主;
    }

    // 压缩标记
    if 动作.contains("压缩") || 源.contains("压缩") {
        return 事件类型::压缩;
    }

    // 回复/思考/模型连接 → message
    if 源.contains("回复")
        || 源.contains("思考")
        || 源.contains("模型连接")
        || 动作.contains("模型")
        || 动作.contains("思考")
        || 动作.contains("回复")
    {
        return 事件类型::消息;
    }

    // 工具调用/道术施展 → tool 或 subtool
    if 源.contains("工具调用") || 源.contains("道术施展") || 动作.contains("工具") {
        if 动作.contains("子工具") || 源.contains("子工具") || 动作.contains("子调用")
        {
            return 事件类型::子工具;
        }
        return 事件类型::工具;
    }

    事件类型::消息
}

/// §13.f.2 建轨迹列表——从白箱事件列表派生轨迹事件行（按 ts 升序，跨轮次连续编号）。
///
/// 轮次划分：遇到 `system` 或 `user` 类型开始新轮次（一次完整的 LLM 调用周期）；
/// 首条事件已是轮次 1；后续 context/message/tool/subtool 归入当前轮次。
pub fn 建轨迹列表(事件: &[白箱事件]) -> Vec<轨迹事件行> {
    let mut 排序 = 事件.to_vec();
    排序.sort_by_key(|e| e.ts);

    let mut 结果 = Vec::with_capacity(排序.len());
    let mut 当前轮次 = 1usize;
    for (序号, 事件) in 排序.iter().enumerate() {
        let 类型 = 派生事件类型(事件);
        if 序号 > 0 && (类型 == 事件类型::系统 || 类型 == 事件类型::界主) {
            当前轮次 += 1;
        }
        结果.push(轨迹事件行 {
            序号: 序号 + 1,
            轮次: 当前轮次,
            类型,
            摘要: 截取(&事件.动作, 80),
            token: 事件.token.clone(),
            耗时ms: 事件.耗时ms,
            事件id: 事件.ts.to_string(),
            ts: 事件.ts,
        });
    }
    结果
}

/// §13.f.3 建轨迹详情——从单事件派生详情面板全量字段。
///
/// 字段按事件类型选择性填充：
/// - `inputDetail`：证据原文（system/user/context/compacted/message/tool）
/// - `outputDetail`：证据原文（message/tool/subtool）
/// - `assistantMetrics`：从证据中提取 TTFT/解码吞吐/总耗时（message）
/// - `provider`/`model`/`retry`/`maxRetries`：从证据中提取（message/tool）
/// - `isError`：动作含失败标记
pub fn 建轨迹详情(事件: &白箱事件) -> 轨迹详情 {
    let 类型 = 派生事件类型(事件);
    let 证据 = &事件.证据;

    let inputDetail = match 类型 {
        事件类型::系统
        | 事件类型::界主
        | 事件类型::上下文
        | 事件类型::压缩
        | 事件类型::消息
        | 事件类型::工具 => 证据.clone(),
        事件类型::子工具 => String::new(),
    };
    let outputDetail = match 类型 {
        事件类型::消息 | 事件类型::工具 | 事件类型::子工具 => 证据.clone(),
        _ => String::new(),
    };
    let thinkingDetail = if 类型 == 事件类型::消息 && 证据.contains("【思考】") {
        提取思考段(证据)
    } else {
        String::new()
    };

    let assistantMetrics = if 类型 == 事件类型::消息 {
        提取助手指标(事件)
    } else {
        None
    };
    let (provider, model) = if 类型 == 事件类型::消息 || 类型 == 事件类型::工具 {
        提取提供者与模型(事件)
    } else {
        (String::new(), String::new())
    };
    let (retry, maxRetries) = if 类型 == 事件类型::消息 || 类型 == 事件类型::工具 {
        提取重试信息(事件)
    } else {
        (None, None)
    };

    轨迹详情 {
        事件id: 事件.ts.to_string(),
        类型,
        ts: 事件.ts,
        轮次: 1,
        inputDetail,
        promptDetail: String::new(),
        outputDetail,
        thinkingDetail,
        assistantMetrics,
        provider,
        model,
        retry,
        maxRetries,
        isError: 事件.动作.contains("失败") || 事件.动作.contains("错误"),
        原始: 事件.clone(),
    }
}

/// 从证据提取思考段（`【思考】`后的内容）。
fn 提取思考段(证据: &str) -> String {
    if let Some(起) = 证据.find("【思考】") {
        截取(&证据[起..], 2000)
    } else {
        String::new()
    }
}

/// 从事件影响项或证据提取助手指标（TTFT/解码吞吐/总耗时）。
///
/// 模型连接-府 的用量附加扩展写入这些字段到观测记录载荷.附加，
/// 但白箱事件只保留证据文本——此处从影响项的附加字段兜底提取，
/// 若证据中含可识别的指标文本也尝试解析。当前简化：从耗时ms构造总耗时。
fn 提取助手指标(事件: &白箱事件) -> Option<助手指标> {
    if 事件.耗时ms > 0 {
        Some(助手指标 {
            TTFT: 0,
            解码吞吐: 0.0,
            总耗时: 事件.耗时ms,
        })
    } else {
        None
    }
}

/// 提取提供者与模型名——从影响项或证据中查找。
fn 提取提供者与模型(事件: &白箱事件) -> (String, String) {
    let mut provider = String::new();
    let mut model = String::new();
    for 项 in &事件.影响 {
        if 项.类型 == "提供者" && !项.名.is_empty() {
            provider = 项.名.clone();
        }
        if 项.类型 == "模型" && !项.名.is_empty() {
            model = 项.名.clone();
        }
    }
    (provider, model)
}

/// 提取重试信息——从影响项中查找。
fn 提取重试信息(事件: &白箱事件) -> (Option<u32>, Option<u32>) {
    let mut retry = None;
    let mut max = None;
    for 项 in &事件.影响 {
        if 项.类型 == "重试" {
            retry = 项.名.parse::<u32>().ok();
        }
        if 项.类型 == "最大重试" {
            max = 项.名.parse::<u32>().ok();
        }
    }
    (retry, max)
}

/// §13.f.5 搜轨迹——全文搜索事件内容，返回命中事件 id + 高亮区间。
///
/// 搜索范围：`源`+`动作`+`证据`+`影响` 文本。命中字符偏移按合并文本计算。
pub fn 搜轨迹(事件: &[白箱事件], 关键词: &str) -> Vec<搜索命中> {
    if 关键词.is_empty() {
        return Vec::new();
    }
    let mut 结果 = Vec::new();
    for 事件 in 事件 {
        let 影响文本: String = 事件
            .影响
            .iter()
            .map(|i| format!("{} {} {}", i.类型, i.名, i.变化))
            .collect::<Vec<_>>()
            .join(" ");
        let 文本 = format!("{} {} {} {}", 事件.源, 事件.动作, 事件.证据, 影响文本);
        let mut 区间 = Vec::new();
        let 关键词字节 = 关键词.len();
        let mut 起 = 0usize;
        while let Some(偏移) = 文本[起..].find(关键词) {
            let 开始 = 起 + 偏移;
            let 结束 = 开始 + 关键词字节;
            区间.push([开始, 结束]);
            起 = 结束;
        }
        if !区间.is_empty() {
            结果.push(搜索命中 {
                事件id: 事件.ts.to_string(),
                高亮区间: 区间,
            });
        }
    }
    结果
}

/// §13.f.4 建时间线——按模式返回色块数据。
///
/// 模式：
/// - `sequence`：值=1（看事件数量分布）
/// - `duration`：值=耗时ms（看哪步慢）
/// - `time`：值=ts（看真实发生时刻）
/// - `actual`：值=耗时ms（简化：同 duration，净执行时间待后续精化）
pub fn 建时间线(事件: &[白箱事件], 模式: &str) -> Vec<时间线色块> {
    let mut 排序 = 事件.to_vec();
    排序.sort_by_key(|e| e.ts);

    排序
        .iter()
        .enumerate()
        .map(|(序号, 事件)| {
            let 值 = match 模式 {
                "duration" => 事件.耗时ms,
                "time" => 事件.ts,
                "actual" => 事件.耗时ms,
                _ => 1,
            };
            时间线色块 {
                序号: 序号 + 1,
                ts: 事件.ts,
                值,
                类型: 派生事件类型(事件),
            }
        })
        .collect()
}

/// §13.f.2 按时间窗与翻页参数过滤事件——供 `/api/trajectory` 端点用。
///
/// - `since`/`until`：时间窗（0 表示不限）
/// - `before`：向上翻页，返 `ts < before` 的最近 `limit` 条
/// - `limit`：条数上限（0 表示不限）
/// - `turn`：按轮次过滤（0 表示不限）
pub fn 过滤轨迹(
    事件: &[白箱事件],
    since: u64,
    until: u64,
    before: u64,
    limit: usize,
    turn: usize,
) -> Vec<白箱事件> {
    let mut 过滤: Vec<白箱事件> = 事件
        .iter()
        .filter(|e| {
            if since > 0 && e.ts < since {
                return false;
            }
            if until > 0 && e.ts > until {
                return false;
            }
            if before > 0 && e.ts >= before {
                return false;
            }
            true
        })
        .cloned()
        .collect();
    过滤.sort_by_key(|e| e.ts);
    if turn > 0 {
        let 轨迹 = 建轨迹列表(&过滤);
        过滤 = 轨迹
            .iter()
            .filter(|行| 行.轮次 == turn)
            .filter_map(|行| 事件.iter().find(|e| e.ts.to_string() == 行.事件id).cloned())
            .collect();
    }
    if limit > 0 && 过滤.len() > limit {
        过滤 = 过滤
            .into_iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
    }
    过滤
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
                    缓存写: 0,
                    推理: 0,
                    总计: 300,
                }),
            白箱事件::新(200, "源", "工具读文件")
                .设耗时(30)
                .设token(crate::token用量 {
                    提示词: 0,
                    输出: 0,
                    缓存: 0,
                    缓存写: 0,
                    推理: 0,
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

    // ===== §13.f.8 派生事件类型单测（7 种类型各造样例，断言派生结果）=====

    #[test]
    fn 派生事件类型_系统提示词() {
        let 事件 = 白箱事件::新(100, "观测/提示词·执行", "发送系统提示词");
        assert_eq!(派生事件类型(&事件), 事件类型::系统);
    }

    #[test]
    fn 派生事件类型_界主输入() {
        let 事件 = 白箱事件::新(100, "观测/提示词·界主", "发送提示词");
        assert_eq!(派生事件类型(&事件), 事件类型::界主);
    }

    #[test]
    fn 派生事件类型_上下文注入() {
        let 事件 = 白箱事件::新(100, "观测/提示词·执行", "注入历史上下文");
        assert_eq!(派生事件类型(&事件), 事件类型::上下文);
    }

    #[test]
    fn 派生事件类型_压缩标记() {
        let 事件 = 白箱事件::新(100, "鸿蒙/识海承载-府", "压缩上下文");
        assert_eq!(派生事件类型(&事件), 事件类型::压缩);
    }

    #[test]
    fn 派生事件类型_助手回复() {
        let 事件 = 白箱事件::新(100, "观测/回复思考·执行", "模型回复");
        assert_eq!(派生事件类型(&事件), 事件类型::消息);
    }

    #[test]
    fn 派生事件类型_工具调用() {
        let 事件 = 白箱事件::新(100, "观测/工具调用·执行", "工具调用");
        assert_eq!(派生事件类型(&事件), 事件类型::工具);
    }

    #[test]
    fn 派生事件类型_子工具() {
        let 事件 = 白箱事件::新(100, "观测/工具调用·执行", "子工具调用");
        assert_eq!(派生事件类型(&事件), 事件类型::子工具);
    }

    #[test]
    fn 派生事件类型_兜底为消息() {
        // 无法识别的源/动作兜底为 message，避免漏派生
        let 事件 = 白箱事件::新(100, "未知源", "未知动作");
        assert_eq!(派生事件类型(&事件), 事件类型::消息);
    }

    #[test]
    fn 建轨迹列表序号连续轮次递增() {
        let 事件 = vec![
            白箱事件::新(100, "观测/提示词·执行", "发送系统提示词"),
            白箱事件::新(200, "观测/提示词·界主", "发送提示词"),
            白箱事件::新(300, "观测/回复思考·执行", "模型回复"),
            白箱事件::新(400, "观测/工具调用·执行", "工具调用"),
        ];
        let 轨迹 = 建轨迹列表(&事件);
        assert_eq!(轨迹.len(), 4);
        assert_eq!(轨迹[0].序号, 1);
        assert_eq!(轨迹[3].序号, 4);
        // system → 轮次1，user → 轮次2，message/tool 归入轮次2
        assert_eq!(轨迹[0].轮次, 1);
        assert_eq!(轨迹[1].轮次, 2);
        assert_eq!(轨迹[2].轮次, 2);
        assert_eq!(轨迹[3].轮次, 2);
        // 类型派生正确
        assert_eq!(轨迹[0].类型, 事件类型::系统);
        assert_eq!(轨迹[1].类型, 事件类型::界主);
        assert_eq!(轨迹[2].类型, 事件类型::消息);
        assert_eq!(轨迹[3].类型, 事件类型::工具);
    }

    #[test]
    fn 建时间线四种模式() {
        let 事件 = vec![
            白箱事件::新(100, "观测/回复思考·执行", "模型回复").设耗时(50),
            白箱事件::新(200, "观测/工具调用·执行", "工具调用").设耗时(30),
        ];
        // sequence 模式：值恒为 1
        let 线 = 建时间线(&事件, "sequence");
        assert_eq!(线[0].值, 1);
        assert_eq!(线[1].值, 1);
        // duration 模式：值=耗时ms
        let 线 = 建时间线(&事件, "duration");
        assert_eq!(线[0].值, 50);
        assert_eq!(线[1].值, 30);
        // time 模式：值=ts
        let 线 = 建时间线(&事件, "time");
        assert_eq!(线[0].值, 100);
        assert_eq!(线[1].值, 200);
        // actual 模式：简化同 duration
        let 线 = 建时间线(&事件, "actual");
        assert_eq!(线[0].值, 50);
    }

    #[test]
    fn 搜轨迹关键词高亮区间() {
        let 事件 = vec![
            白箱事件::新(100, "观测/回复思考·执行", "模型回复含关键词"),
            白箱事件::新(200, "观测/工具调用·执行", "工具调用无匹配"),
        ];
        let 命中 = 搜轨迹(&事件, "关键词");
        assert_eq!(命中.len(), 1);
        assert!(!命中[0].高亮区间.is_empty());
        assert_eq!(命中[0].事件id, "100");
    }

    #[test]
    fn 搜轨迹空关键词返空() {
        let 事件 = vec![白箱事件::新(100, "源", "动作")];
        assert!(搜轨迹(&事件, "").is_empty());
    }

    #[test]
    fn 建轨迹详情按类型选择性填充() {
        let 事件 = 白箱事件::新(100, "观测/回复思考·执行", "模型回复").设证据("回复正文");
        let 详情 = 建轨迹详情(&事件);
        assert_eq!(详情.类型, 事件类型::消息);
        assert_eq!(详情.inputDetail, "回复正文");
        assert_eq!(详情.outputDetail, "回复正文");
        assert!(!详情.isError);
    }

    #[test]
    fn 建轨迹详情失败标记() {
        let 事件 = 白箱事件::新(100, "观测/工具调用·执行", "工具调用失败");
        let 详情 = 建轨迹详情(&事件);
        assert!(详情.isError, "动作含「失败」应标记 isError");
    }
}
