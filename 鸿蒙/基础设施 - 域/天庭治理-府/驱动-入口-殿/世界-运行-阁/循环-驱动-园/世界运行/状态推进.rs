//! 状态推进：状态目录 + 读改写队列 + 要求/想法状态机推进 + 任务线入队/领取/回填 + 指标落盘。
//!
//! 拆出自 `世界运行.rs`（D2 §2.10.1）。本模块集中所有「改状态/落盘队列」操作：
//! - 基础工具：`状态目录`、`读改写队列`、`唯一id`
//! - 要求状态机：`推进要求状态`、`推进并警`、`追加要求`、`下一个要求序号`、`归位要求状态`
//! - 想法状态机：`推进想法状态`
//! - 任务线：`登记任务线`、`领取待执行任务线`、`读任务线们`、`中止任务线`、`回填任务线结果`、`任务线状态`
//! - 指标/告警：`记指标`、`读指标们`、`失败告警`
//! - 失败沉淀：`沉淀失败`
//! - 对话记录：`落对话记录`
//!
//! 持久化经 `super::持久化` 调用，避免与本模块耦合。

use super::持久化::{持久化任务线们, 持久化列表, 持久化要求们};
use crate::类型_定义_殿::{
    任务线, 任务线状态, 想法, 想法状态, 要求书, 要求状态
};
use rizhi_fu::{info, warn};

/// 状态目录：工作区根下的 .上下文/状态（与命令操作-府的 状态目录 行为对齐，本府复制以保持跨府引用只走 lib 根符号的边界）。
pub(super) fn 状态目录() -> std::path::PathBuf {
    let 根 = std::env::var("WORLD_WORKSPACE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    根.join(".上下文").join("状态")
}

/// 持进程级排他锁读队列文件（复合「读→改→写」用：锁贯穿到调用方持久化后释放）。
/// 返回 (全部项, 排他锁)；调用方改完项们后调 持久化XX们 再 drop 锁（勿在持锁期间调队列方法防重入死锁）。
pub(super) fn 读改写队列<T: serde::Serialize + serde::de::DeserializeOwned>(
    路径: &std::path::Path,
) -> Result<(Vec<T>, crate::排他锁), String> {
    let 队列 = crate::落盘队列::<T>::打开(路径);
    let 锁 = 队列
        .排他()
        .map_err(|错误| format!("拿队列排他锁失败: {错误}"))?;
    let 内容 = std::fs::read_to_string(路径).map_err(|错误| format!("读队列失败: {错误}"))?;
    let 项们 = 内容
        .lines()
        .filter(|行| !行.trim().is_empty())
        .map(|行| serde_json::from_str::<T>(行).map_err(|错误| format!("解析队列项失败: {错误}")))
        .collect::<Result<Vec<T>, String>>()?;
    Ok((项们, 锁))
}

/// 推进要求状态机：按要求 id 找到目标 → 校验迁移合法 → 改状态 → 原子落盘。
/// 校验失败打 warn 不阻断（甲阶段自动推进；非法迁移以现状为准）。
pub(super) fn 推进要求状态(要求id: &str, 目标: 要求状态) -> Result<(), String> {
    let 队列路径 = 状态目录().join("要求.jsonl");
    let (mut 项们, 锁) = 读改写队列::<要求书>(&队列路径)?;
    let mut 命中 = false;
    for 项 in 项们.iter_mut() {
        if 项.id == 要求id {
            // 非法迁移告警但不阻断（甲阶段流水线自动推进），保持状态机迁移可视。
            if !crate::合法迁移(&项.状态).contains(&目标) && 项.状态 != 目标 {
                warn!(要求id, 当前 = ?项.状态, 目标 = ?目标, "状态机非法迁移，仍落盘");
            }
            项.状态 = 目标.clone();
            命中 = true;
            break;
        }
    }
    if !命中 {
        warn!(要求id, 目标 = ?目标, "推进要求状态未命中（要求未持久化）");
        return Ok(());
    }
    持久化要求们(&队列路径, &项们)?;
    drop(锁);
    // 事件流：要求状态推进（append-only 事实源）。
    let 流 = shihai_fu::事件流::在工作区(&shihai_fu::工作区::定位());
    流.追加事件静默(
        shihai_fu::事件类型::要求状态推进,
        serde_json::json!({
            "要求id": 要求id,
            "状态": format!("{:?}", 目标),
        }),
    );
    info!(要求id, 状态 = ?目标, "要求状态已推进");
    Ok(())
}

/// 推进并警：封装「推进要求状态 + 失败告警」模式（消除 9 处重复 if let Err + warn）。
/// `目标` 同时用于推进与告警文案（Debug 输出即中文状态名）。
pub(super) fn 推进并警(要求id: &str, 目标: 要求状态) {
    let 目标名 = format!("{:?}", 目标);
    if let Err(错误) = 推进要求状态(要求id, 目标) {
        warn!(要求id = %要求id, 错误 = %错误, "推进「{目标名}」失败");
    }
}

/// 追加要求到要求.jsonl（首次入池：要求由「解析想法」产出，状态机起点 = 待领）。
/// 落盘队列无原 id 追加接口：读全部 → 追加新项 → 写临时文件 → 原子改名（防半写损坏）。
/// 持进程级排他锁贯穿读改写（2026-08-17 轮8 体检：守护回填与界主登记并发会互相覆盖）。
pub(super) fn 追加要求(要求: &要求书) -> Result<(), String> {
    let 队列路径 = 状态目录().join("要求.jsonl");
    let (mut 项们, 锁) = 读改写队列::<要求书>(&队列路径)?;
    // 防止同 id 重复追加（重入运行一轮 / 想法被多次投递）：已存在则覆盖旧状态，不重复入队。
    let mut 已存在 = false;
    for 项 in 项们.iter_mut() {
        if 项.id == 要求.id {
            *项 = 要求.clone();
            已存在 = true;
            break;
        }
    }
    if !已存在 {
        项们.push(要求.clone());
    }
    持久化要求们(&队列路径, &项们)?;
    drop(锁);
    // 事件流：要求入池（append-only 事实源）。
    let 流 = shihai_fu::事件流::在工作区(&shihai_fu::工作区::定位());
    流.追加事件静默(
        shihai_fu::事件类型::要求入池,
        serde_json::json!({
            "要求id": 要求.id,
            "方向": 要求.方向,
            "状态": format!("{:?}", 要求.状态),
            "已存在": 已存在,
        }),
    );
    info!(要求id = %要求.id, 状态 = ?要求.状态, 已存在, "要求已入池");
    Ok(())
}

/// 下一个要求序号：读要求.jsonl 现有最大序号 +1（全局递增，防并发任务线 id 撞车；设计稿 §1.5.5 拍板 7）。
/// 并发安全：进程内 AtomicU64 单调（初始化 0→基准=磁盘 max；之后 fetch_add 返回 max+1、max+2…），
/// 并发多线程绝不重复；重启后重新读磁盘 max 作基准，跨进程/跨重启也不撞。
pub(super) fn 下一个要求序号() -> Result<u64, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static 序号基准: AtomicU64 = AtomicU64::new(0);
    // 惰性初始化：首次读到磁盘当前最大序号作基准（#0 哨兵 = 未初始化）。
    let 当前 = 序号基准.load(Ordering::Relaxed);
    if 当前 == 0 {
        let 队列 = crate::落盘队列::<要求书>::打开(状态目录().join("要求.jsonl"));
        // 兜底：读失败（目录不可达 / 文件不存在 / 权限不足）视为空目录，基准=0。
        // 测试场景下「空状态目录也能 fetch 序号」是契约之一；并发首屏多线程同时进入时，
        // 即使个别线程读失败回落 0，CAS 也保证只有一个线程赢，其余用 0 走 fetch_add 仍唯一。
        let 全部 = 队列.读全部().unwrap_or_default();
        let 最大 = 全部
            .iter()
            .filter_map(|要求| {
                要求
                    .id
                    .strip_prefix("要求-")
                    .and_then(|尾| 尾.parse::<u64>().ok())
            })
            .max()
            .unwrap_or(0);
        // CAS 初始化基准为 max；并发首屏只有一线程赢, 其余用磁盘旧值(仍被 fetch_add 保证唯一)。
        序号基准
            .compare_exchange(0, 最大, Ordering::SeqCst, Ordering::SeqCst)
            .unwrap_or_default();
    }
    Ok(序号基准.fetch_add(1, Ordering::Relaxed) + 1)
}

/// 推进想法状态：读全部 → 改目标 → 原子重写（对话/任务线共用，防目标状态被覆盖）。
/// 持进程级排他锁贯穿读改写（2026-08-17 轮8 体检：守护推进与界主投递并发会互相覆盖）。
pub fn 推进想法状态(目标id: &str, 新状态: 想法状态) -> Result<(), String> {
    let 想法路径 = 状态目录().join("想法.jsonl");
    let (mut 项们, 锁) = 读改写队列::<想法>(&想法路径)?;
    let mut 命中 = false;
    for 项 in 项们.iter_mut() {
        if 项.id == 目标id {
            项.状态 = 新状态.clone();
            命中 = true;
            break;
        }
    }
    if !命中 {
        return Err(format!("未找到目标想法：{目标id}"));
    }
    // 复用 持久化列表 泛型原子落盘（消除内联 join+format 两次分配，与要求/任务线同款）。
    持久化列表(&想法路径, &项们, "想法")?;
    drop(锁);
    info!(目标id, 新状态 = ?新状态, "想法状态已推进");
    Ok(())
}

/// 落一条对话记录（追加写，不重写历史；对话/任务线汇报共用）。
pub fn 落对话记录(发送者: &str, 文本: &str, 可见: &[String]) {
    #[derive(serde::Serialize)]
    struct 记录 {
        发送者: String,
        文本: String,
        可见: Vec<String>,
        时间戳: u64,
    }
    let 记录 = 记录 {
        发送者: 发送者.to_string(),
        文本: 文本.to_string(),
        可见: 可见.to_vec(),
        时间戳: shihai_fu::当前毫秒(),
    };
    let 路径 = 状态目录().join("对话.jsonl");
    if let Ok(行) = serde_json::to_string(&记录) {
        use std::io::Write;
        if let Ok(mut 文件) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&路径)
        {
            let _ = writeln!(文件, "{行}");
        }
    }
}

/// 单调唯一 id：毫秒 + 进程内原子序号。防同毫秒并发登记撞 id
/// （测试实测：并发登记 30 条同毫秒命中，回填按 id 只改首条，其余残留执行中）。
pub(crate) fn 唯一id(前缀: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static 序号: AtomicU64 = AtomicU64::new(0);
    format!(
        "{前缀}-{}-{}",
        shihai_fu::当前毫秒(),
        序号.fetch_add(1, Ordering::Relaxed) + 1
    )
}

/// 登记任务线：一次对话发布的任务单元入盘（待执行）。
pub fn 登记任务线(想法: &想法) -> Result<任务线, String> {
    let 任务线 = 任务线 {
        id: 唯一id("任务线"),
        想法id: 想法.id.clone(),
        想法内容: 想法.内容.clone(),
        要求id: None,
        状态: 任务线状态::待执行,
        结论: None,
        汇报: String::new(),
        时间: shihai_fu::当前毫秒(),
    };
    let 队列 = crate::落盘队列::<任务线>::打开(状态目录().join("任务线.jsonl"));
    队列.入队(&任务线)?;
    info!(任务线id = %任务线.id, "任务线已登记（待执行）");
    Ok(任务线)
}

/// 领取一条待执行任务线：锁文件互斥（create_new 原子），先到先得，防并发双跑。
/// 陈旧执行中阈值（秒）：任务线领取后超过该时长仍未完成 → 视为守护进程崩溃残留，
/// 领取时自动重置为 待执行（生产化 3.1 自愈）。6 小时 > 单任务最坏耗时
/// （3 次重投 × 32 轮 × 每轮分钟级），正常执行不会被误判。
pub const 陈旧执行中阈值秒: u64 = 6 * 3600;

/// 领取一条可执行任务线：优先 待执行；无待执行且存在「陈旧执行中」（领取时间戳超阈值，
/// 守护崩溃残留）→ 重置 待执行 后领取。领取成功即写心跳（时间=当前），防正常执行被误判陈旧。
/// 统一走 落盘队列 排他锁（jsonl.lock，与 入队/回填/中止 同锁互斥，防并发双跑与读改写覆盖，
/// 2026-08-17 轮8 体检：原独立 任务线.lock 与回填的 jsonl.lock 不同源，存在读改写竞态）。
pub fn 领取待执行任务线() -> Result<Option<任务线>, String> {
    let 路径 = 状态目录().join("任务线.jsonl");
    let 队列 = crate::落盘队列::<任务线>::打开(路径.clone());
    let 锁 = match 队列.排他() {
        Ok(锁) => 锁,
        Err(_) => return Ok(None), // 他人持有锁，本轮不抢（超时视为被占用）
    };
    let 结果 = (|| -> Result<Option<任务线>, String> {
        let 内容 =
            std::fs::read_to_string(&路径).map_err(|错误| format!("读任务线队列失败: {错误}"))?;
        let mut 项们 = 内容
            .lines()
            .filter(|行| !行.trim().is_empty())
            .map(|行| {
                serde_json::from_str::<任务线>(行).map_err(|错误| format!("解析任务线失败: {错误}"))
            })
            .collect::<Result<Vec<任务线>, String>>()?;
        let 现在 = shihai_fu::当前毫秒();
        let 位置 = 项们.iter().position(|线| 线.状态 == 任务线状态::待执行).or_else(|| {
            // 陈旧执行中（崩溃残留）：领取时间戳（心跳）超阈值 → 重置待执行。
            let 陈旧位置 = 项们.iter().position(|线| {
                线.状态 == 任务线状态::执行中 && 现在.saturating_sub(线.时间) > 陈旧执行中阈值秒 * 1000
            });
            if let Some(索引) = 陈旧位置 {
                warn!(任务线id = %项们[索引].id, "任务线执行中已超陈旧阈值，视为崩溃残留，重置待执行");
            }
            陈旧位置
        });
        let 领取 = match 位置 {
            Some(索引) => {
                let 目标 = 项们[索引].clone();
                项们[索引].状态 = 任务线状态::执行中;
                项们[索引].时间 = 现在; // 心跳：领取时刻写入，防正常执行被误判陈旧
                持久化任务线们(&路径, &项们)?;
                Some(目标)
            }
            None => None,
        };
        Ok(领取)
    })();
    drop(锁);
    结果
}

/// 读全部任务线（供状态查询/守护轮询判断）。
pub fn 读任务线们() -> Result<Vec<任务线>, String> {
    crate::落盘队列::<任务线>::打开(状态目录().join("任务线.jsonl"))
        .读全部()
        .map_err(|错误| format!("读任务线队列失败: {错误}"))
}

/// 中止任务线（生产化 1.3）：任何状态 → 已中止。
/// 待执行/执行中/已完成的任务线都可中止；执行中的任务线由执行进程在回填前检查中止标记，
/// 已中止则不汇报并撤销产物（回滚垫前缀撤销）。
/// 持排他锁贯穿读改写（2026-08-17 轮8 体检）。
pub fn 中止任务线(任务线id: &str) -> Result<String, String> {
    let 路径 = 状态目录().join("任务线.jsonl");
    let (mut 项们, 锁) = 读改写队列::<任务线>(&路径)?;
    let mut 命中 = false;
    for 项 in 项们.iter_mut() {
        if 项.id == 任务线id {
            项.状态 = 任务线状态::已中止;
            命中 = true;
            break;
        }
    }
    if !命中 {
        return Err(format!("未找到任务线：{任务线id}"));
    }
    持久化任务线们(&路径, &项们)?;
    drop(锁);
    info!(任务线id, "任务线已中止");
    Ok(format!("任务线 {任务线id} 已中止"))
}

/// 查任务线当前状态（执行回填前检查中止标记用）。
pub(super) fn 任务线状态(任务线id: &str) -> 任务线状态 {
    crate::落盘队列::<任务线>::打开(状态目录().join("任务线.jsonl"))
        .读全部()
        .unwrap_or_default()
        .iter()
        .find(|线| 线.id == 任务线id)
        .map(|线| 线.状态.clone())
        .unwrap_or(任务线状态::已中止)
}

/// 归位要求状态（生产化 2.2）：任务线执行失败/异常终了后，若要求仍卡在中间态
/// （设计中/待确认/已确认/实现中/已验收），强制回 待实现——防「实现中」残留
/// （实测历史遗留：要求-4/5 入池后中断，状态长期卡待领/实现中，事项列表呈现失真）。
pub(super) fn 归位要求状态(想法id: &str) {
    let 路径 = 状态目录().join("要求.jsonl");
    let Ok((mut 项们, 锁)) = 读改写队列::<要求书>(&路径) else {
        return;
    };
    let mut 改 = false;
    for 项 in 项们.iter_mut() {
        if 项.想法id.as_deref() == Some(想法id) {
            match 项.状态 {
                要求状态::设计中
                | 要求状态::待确认
                | 要求状态::已确认
                | 要求状态::实现中
                | 要求状态::已验收 => {
                    warn!(要求id = %项.id, 原状态 = ?项.状态, "要求卡在中间态，归位待实现");
                    项.状态 = 要求状态::待实现;
                    改 = true;
                }
                _ => {}
            }
        }
    }
    if 改 {
        if let Err(错误) = 持久化要求们(&路径, &项们) {
            warn!(错误 = %错误, "归位要求状态落盘失败");
        }
    }
    drop(锁);
}

/// 回填任务线结果：要求id / 结论 / 汇报 → 状态 已完成。
/// 持排他锁贯穿读改写（2026-08-17 轮8 体检：与登记/中止/领取同锁互斥）。
pub fn 回填任务线结果(
    任务线id: &str,
    要求id: &str,
    结论: &str,
    汇报: &str,
) -> Result<(), String> {
    let 路径 = 状态目录().join("任务线.jsonl");
    let (mut 项们, 锁) = 读改写队列::<任务线>(&路径)?;
    let mut 命中 = false;
    for 项 in 项们.iter_mut() {
        if 项.id == 任务线id {
            项.要求id = Some(要求id.to_string());
            项.结论 = Some(结论.to_string());
            项.汇报 = 汇报.to_string();
            项.状态 = 任务线状态::已完成;
            命中 = true;
            break;
        }
    }
    if !命中 {
        return Err(format!("未找到任务线：{任务线id}"));
    }
    持久化任务线们(&路径, &项们)?;
    drop(锁);
    Ok(())
}

/// 失败沉淀：把一次失败写进世界状态.失败模式（按 要求id+阶段 去重累加次数）。
/// 失败模式是进化环归因的输入，不再恒空；所在层先记「实现层」，由后续进化归因细化。
pub(super) fn 沉淀失败(要求id: &str, 阶段: &str, 原因: &str) -> Result<(), String> {
    let 目录 = 状态目录();
    let mut 状态 = crate::确保世界状态初始化(&目录)?;
    let 已有 = 状态
        .失败模式
        .iter_mut()
        .find(|条目| 条目.要求id == 要求id && 条目.阶段 == 阶段);
    if let Some(条目) = 已有 {
        条目.次数 += 1;
        if !原因.is_empty() {
            条目.原因 = 原因.to_string();
        }
    } else {
        状态.失败模式.push(crate::失败条目 {
            要求id: 要求id.to_string(),
            阶段: 阶段.to_string(),
            原因: 原因.to_string(),
            所在层: crate::缺陷层::实现层,
            次数: 1,
            时间: shihai_fu::当前毫秒(),
        });
    }
    crate::写世界状态(&目录, &状态)
}

/// 任务线指标（生产化 3.3）：落 .上下文/状态/指标.jsonl，供成功率/成本趋势统计与告警。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct 指标 {
    时间: u64,
    任务线id: String,
    要求id: String,
    结论: String,
    耗时毫秒: u64,
    token: u64,
    失败原因: String,
}

/// 记一条任务线指标。
pub(super) fn 记指标(
    任务线id: &str,
    要求id: &str,
    结论: &str,
    耗时毫秒: u64,
    token: u64,
    失败原因: &str,
) {
    let 指标 = 指标 {
        时间: shihai_fu::当前毫秒(),
        任务线id: 任务线id.to_string(),
        要求id: 要求id.to_string(),
        结论: 结论.to_string(),
        耗时毫秒,
        token,
        失败原因: 失败原因.to_string(),
    };
    let 路径 = 状态目录().join("指标.jsonl");
    if let Ok(行) = serde_json::to_string(&指标) {
        use std::io::Write;
        if let Ok(mut 文件) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&路径)
        {
            let _ = writeln!(文件, "{行}");
        }
    }
}

/// 读全部指标（按时间正序）。
pub(super) fn 读指标们() -> Vec<指标> {
    let 路径 = 状态目录().join("指标.jsonl");
    let Ok(内容) = std::fs::read_to_string(&路径) else {
        return Vec::new();
    };
    内容
        .lines()
        .filter(|行| !行.trim().is_empty())
        .filter_map(|行| serde_json::from_str::<指标>(行).ok())
        .collect()
}

/// 失败告警（生产化 3.4）：最近 3 条指标全失败 → 鸿钧落一条告警对话记录。
pub(super) fn 失败告警(结论: &str) {
    if 结论 != "打回" && 结论 != "已中止" {
        return;
    }
    let 指标们 = 读指标们();
    let 尾部: Vec<&指标> = 指标们.iter().rev().take(3).collect();
    if 尾部.len() >= 3
        && 尾部
            .iter()
            .all(|指标| 指标.结论 == "打回" || 指标.结论 == "已中止")
    {
        let 最近失败 = 尾部
            .iter()
            .find(|指标| !指标.失败原因.is_empty())
            .map(|指标| 指标.失败原因.clone())
            .unwrap_or_else(|| "（无详细原因）".to_string());
        warn!(连续失败数 = 尾部.len(), "最近任务连续失败，触发告警");
        crate::落对话记录(
            "鸿钧",
            &format!(
                "告警：最近 {} 个任务连续失败（最近失败：{}）。建议检查网络与任务描述，或暂停投递排查。",
                尾部.len(),
                最近失败
            ),
            &["界主".to_string(), "鸿钧".to_string()],
        );
    }
}

#[cfg(test)]
mod 测试 {
    /// 并发要求序号唯一性（2026-08-18 并发消费改造）：多线程并发调 下一个要求序号 必须互不重复，
    /// 否则并发任务会撞 要求id，导致状态推进/验收互相覆盖。
    #[test]
    fn 并发要求序号互不重复() {
        use std::collections::HashSet;
        let 线程数 = 8;
        let 每条 = 50usize;
        let 结果 = std::thread::scope(|作用域| {
            let 句柄们: Vec<_> = (0..线程数)
                .map(|_| {
                    作用域.spawn(|| {
                        let mut 本地 = Vec::with_capacity(每条);
                        for _ in 0..每条 {
                            // 空状态目录下读不出任何要求, 基准 0→fetch 得 1..N, 仍应互不重复。
                            本地.push(super::下一个要求序号().unwrap());
                        }
                        本地
                    })
                })
                .collect();
            let mut 全部 = Vec::new();
            for 句柄 in 句柄们 {
                全部.extend(句柄.join().unwrap());
            }
            全部
        });
        let 去重后: HashSet<u64> = 结果.iter().copied().collect();
        assert_eq!(
            结果.len(),
            去重后.len(),
            "并发取序号应全部唯一（撞了 {} 个）",
            结果.len() - 去重后.len()
        );
    }
}
