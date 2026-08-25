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
use shihai_fu::{依赖图, 工作区};
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

    // §19.4.2.7 P0-1 + P0-2 闭环（2026-08-25）：接单门闭包真接入调度驱动。
    // 依赖图一次性加载（失败兜底空图，不阻塞调度）；闭包走 4 层瀑布
    // （§19.4.1 + §19.4.2.2）：层 1 路径前缀 + 层 2 缓存字段 + 层 4 反向匹配。
    // 模糊情况保守接受 `true`——让 §19.8 名单消费阶段调 LLM 兜底（§6.4 热路径不调 LLM）。
    let 依赖图_句柄 = {
        let 工作区_句柄 = 工作区::定位();
        依赖图::加载自工作区(&工作区_句柄).unwrap_or_default()
    };

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
                                                 // §19.4.2.7 P0-1：接单门闭包真接入（None 占位 → Some(测试覆盖查询)）。
                                                 // 4 层瀑布真生效，候选是否有测试覆盖由依赖图缓存 + 路径前缀 + 反向匹配共同判定。
    let 决策 = tianting_fu::调度_接单_园::评估接单(
        &候选,
        候选池已空,
        &涉及路径,
        工作区根,
        Some(|路径串: &str| {
            tianting_fu::调度_接单_园::测试覆盖查询(&依赖图_句柄, 路径串)
        }),
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

/// 消费待修正-测试标识.jsonl 名单（§19.4.2.3 实施规范 + §19.4.2.7 P0-3 完整版）。
///
/// 完整流程（事务 4 步 + 归档）：
/// 1. 读 `.上下文/状态/待修正-测试标识.jsonl`
/// 2. 过滤 `执行状态 = "待执行"` + 按 实体键 去重（§14.20 reducer 链头）
/// 3. 对每条「待执行」项执行 4 步事务：
///    ① 回滚垫备份（`shihai_fu::回滚垫::备份`）
///    ② edit 修改（按 `动作` 字段：加 `#[test]` / 去 `#[test]` / 加注释审）
///    ③ cargo check 验证（受影响 crate）
///    ④ 失败回滚（`撤销任务前缀`）+ 标记 `执行状态`
/// 4. 全部跑完：归档名单到 `.上下文/归档/待修正-测试标识-{时间}.jsonl`
///
/// **dry_run 默认 true**——`WORLD_TEST_PATCH_DRY_RUN=false` 才真改文件。
/// 依据：§19.7 LLM 调用不在热路径 + §6.4 不开新线程 + AGENTS §16 不可擅动用户数据。
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

    // §19.4.2.7 P0-3 完整版：事务 4 步 + 归档。
    // dry_run 默认 true（防误改）；环境变量 WORLD_TEST_PATCH_DRY_RUN=false 关闭。
    let dry_run = std::env::var("WORLD_TEST_PATCH_DRY_RUN")
        .map(|v| v != "false")
        .unwrap_or(true);
    let 任务id_prefix = format!("待修正-{}", shihai_fu::当前毫秒());

    let 工作区 = shihai_fu::工作区::定位();
    let 回滚垫_句柄 = shihai_fu::回滚垫::在工作区(&工作区);

    let mut 报告 = 名单消费报告 {
        待执行总数,
        ..Default::default()
    };

    for (实体键, 值) in &实体键表 {
        let 文件路径 = 值["文件路径"].as_str().unwrap_or("").to_string();
        let 动作 = 值["动作"].as_str().unwrap_or("").to_string();
        if 文件路径.is_empty() || 动作.is_empty() {
            warn!(实体键 = %实体键, "调度驱动：待修正行缺文件路径/动作，跳过");
            报告.已失败 += 1;
            continue;
        }

        // ① 回滚垫备份（dry_run 也备份，方便后续 dry_run=false 时启用）
        let 任务id = format!("{}-{}", 任务id_prefix, 实体键);
        let 绝对路径 = 工作区根.join(&文件路径);
        if let Err(错误) = 回滚垫_句柄.备份(&任务id, 绝对路径.to_string_lossy().as_ref())
        {
            warn!(任务id, 文件路径 = %文件路径, "调度驱动：回滚备份失败：{错误}");
            报告.已失败 += 1;
            continue;
        }

        // ② edit 修改（按动作字段；dry_run 不真改）
        if dry_run {
            info!(
                任务id, 文件路径 = %文件路径, 动作 = %动作,
                "调度驱动 [DRY_RUN] 待改（未真改文件）"
            );
            报告.已跳过 += 1;
            // dry_run 不跑 cargo check，不动名单（待后续 dry_run=false 时再消费）
            continue;
        }
        if let Err(错误) = 应用动作(&绝对路径, &动作) {
            warn!(任务id, 文件路径 = %文件路径, 动作 = %动作, "调度驱动：edit 修改失败：{错误}");
            // 失败回滚
            let _ = 回滚垫_句柄.撤销任务前缀(&任务id);
            报告.已失败 += 1;
            continue;
        }

        // ③ cargo check 验证（受影响 crate）
        let crate名 = 路径转crate名(&文件路径);
        let 验证结果 = match crate名 {
            Some(名) => crate_check(&名),
            None => {
                warn!(文件路径 = %文件路径, "调度驱动：无法推导 crate 名，兜底整 workspace");
                crate_check_workspace()
            }
        };
        if !验证结果 {
            warn!(任务id, 文件路径 = %文件路径, "调度驱动：cargo check 失败，回滚");
            let _ = 回滚垫_句柄.撤销任务前缀(&任务id);
            报告.已失败 += 1;
            continue;
        }

        info!(任务id, 文件路径 = %文件路径, 动作 = %动作, "调度驱动：edit 已成功");
        报告.已成功 += 1;
    }

    // 归档名单（全部跑完——不论成功失败）
    if let Err(错误) = 归档名单(&名单路径) {
        warn!(错误 = %错误, "调度驱动：归档名单失败");
    }

    Some(报告)
}

/// 应用动作（编辑文件）：按 §19.9 定义的三种动作。
///
/// - `加#[test]`：在第一行 `pub fn` 上方插入 `#[test]\n`
/// - `去#[test]`：删第一个 `#[test]` 行
/// - `加注释审`：在第一行 `pub fn` 上方插入 `// §19.4.2.5 LLM 判定：边缘（人工可审）\n`
fn 应用动作(文件路径: &Path, 动作: &str) -> Result<(), String> {
    let 内容 = fs::read_to_string(文件路径).map_err(|e| format!("读文件失败: {e}"))?;
    let 新内容 = match 动作 {
        "加#[test]" => 加测试标记(&内容),
        "去#[test]" => 去测试标记(&内容),
        "加注释审" => 加边缘注释(&内容),
        其他 => return Err(format!("未知动作: {其他}")),
    }?;
    fs::write(文件路径, &新内容).map_err(|e| format!("写文件失败: {e}"))?;
    Ok(())
}

/// 在第一个 `pub fn` 上方插入 `#[test]`。
fn 加测试标记(内容: &str) -> Result<String, String> {
    let 行们: Vec<&str> = 内容.lines().collect();
    if let Some(索引) = 行们
        .iter()
        .position(|行| 行.trim_start().starts_with("pub fn "))
    {
        // 已存在 #[test] 标记？跳过
        if 索引 > 0 && 行们[索引 - 1].trim_start().starts_with("#[test]") {
            return Ok(内容.to_string());
        }
        let mut 新行们 = 行们.clone();
        新行们.insert(索引, "#[test]");
        let mut 结果 = 新行们.join("\n");
        if 内容.ends_with('\n') {
            结果.push('\n');
        }
        Ok(结果)
    } else {
        Err("未找到 pub fn".to_string())
    }
}

/// 删除第一个 `#[test]` 行（若存在）。
fn 去测试标记(内容: &str) -> Result<String, String> {
    let 行们: Vec<&str> = 内容.lines().collect();
    if let Some(索引) = 行们
        .iter()
        .position(|行| 行.trim_start().starts_with("#[test]"))
    {
        let mut 新行们 = 行们.clone();
        新行们.remove(索引);
        let mut 结果 = 新行们.join("\n");
        if 内容.ends_with('\n') {
            结果.push('\n');
        }
        Ok(结果)
    } else {
        // 没有 #[test] 也是「成功」（已正确）
        Ok(内容.to_string())
    }
}

/// 在第一个 `pub fn` 上方插入「边缘」注释（人工可审）。
fn 加边缘注释(内容: &str) -> Result<String, String> {
    let 行们: Vec<&str> = 内容.lines().collect();
    if let Some(索引) = 行们
        .iter()
        .position(|行| 行.trim_start().starts_with("pub fn "))
    {
        let 注释 = "// §19.4.2.5 LLM 判定：边缘（人工可审）";
        // 已存在同样注释？跳过
        if 索引 > 0 && 行们[索引 - 1].trim() == 注释 {
            return Ok(内容.to_string());
        }
        let mut 新行们 = 行们.clone();
        新行们.insert(索引, 注释);
        let mut 结果 = 新行们.join("\n");
        if 内容.ends_with('\n') {
            结果.push('\n');
        }
        Ok(结果)
    } else {
        Err("未找到 pub fn".to_string())
    }
}

/// 路径前缀 → Cargo.toml lib name 映射（10 府 1 世界）。
///
/// 简化硬编码：路径首段 + 第二段「府」名 → lib name。
fn 路径转crate名(路径: &str) -> Option<String> {
    let 正斜杠 = 路径.replace('\\', "/");
    if 正斜杠.starts_with("鸿蒙/基础设施 - 域/") {
        if let Some(rest) = 正斜杠.strip_prefix("鸿蒙/基础设施 - 域/") {
            let 府 = rest.split('/').next()?;
            return 映射_鸿蒙_基础设施_域(府);
        }
    }
    if 正斜杠.starts_with("鸿蒙/世界配置 - 域/") {
        if let Some(rest) = 正斜杠.strip_prefix("鸿蒙/世界配置 - 域/") {
            let 府 = rest.split('/').next()?;
            return 映射_鸿蒙_世界配置_域(府);
        }
    }
    if 正斜杠.starts_with("乾坤/呈现-域/") {
        if let Some(rest) = 正斜杠.strip_prefix("乾坤/呈现-域/") {
            let 府 = rest.split('/').next()?;
            return 映射_乾坤_呈现_域(府);
        }
    }
    if 正斜杠.starts_with("证道/鸿蒙 - 域/") {
        if let Some(rest) = 正斜杠.strip_prefix("证道/鸿蒙 - 域/") {
            let 府 = rest.split('/').next()?;
            return 映射_证道_鸿蒙_域(府);
        }
    }
    if 正斜杠.starts_with("世界/") {
        return Some("世界".to_string());
    }
    None
}

fn 映射_鸿蒙_基础设施_域(府: &str) -> Option<String> {
    match 府 {
        "识海承载-府" => Some("shihai_fu".to_string()),
        "天庭治理-府" => Some("tianting_fu".to_string()),
        "道术施展-府" => Some("daoshu_fu".to_string()),
        "模型连接-府" => Some("moxing_fu".to_string()),
        "日志记录-府" => Some("rizhi_fu".to_string()),
        "事件总线-府" => Some("shijian_fu".to_string()),
        "状态共享-府" => Some("zhuangtai_fu".to_string()),
        "插件承载-府" => Some("chajian_fu".to_string()),
        "观测探针-府" => Some("jiance_fu".to_string()),
        _ => None,
    }
}

fn 映射_鸿蒙_世界配置_域(府: &str) -> Option<String> {
    match 府 {
        "配置管理-府" => Some("peizhi_fu".to_string()),
        _ => None,
    }
}

fn 映射_乾坤_呈现_域(府: &str) -> Option<String> {
    match 府 {
        "命令操作-府" => Some("mingling_fu".to_string()),
        _ => None,
    }
}

fn 映射_证道_鸿蒙_域(府: &str) -> Option<String> {
    match 府 {
        "单元测试-府" => Some("zhengdao_fu".to_string()),
        _ => None,
    }
}

/// cargo check -p <crate>（子进程调 cargo）。
/// 返 true 表示编译通过。
fn crate_check(crate名: &str) -> bool {
    let 输出 = match std::process::Command::new("cargo")
        .args(["check", "-p", crate名, "--quiet"])
        .output()
    {
        Ok(o) => o,
        Err(错误) => {
            warn!(crate名, "调度驱动：cargo check 子进程启动失败：{错误}");
            return false;
        }
    };
    if !输出.status.success() {
        let 错误串 = String::from_utf8_lossy(&输出.stderr);
        warn!(
            crate名,
            错误摘要 = %错误串.lines().take(5).collect::<Vec<_>>().join(" | "),
            "调度驱动：cargo check 失败"
        );
        return false;
    }
    true
}

/// cargo check --workspace 兜底（路径无法推导 crate 名时使用）。
fn crate_check_workspace() -> bool {
    let 输出 = match std::process::Command::new("cargo")
        .args(["check", "--workspace", "--quiet"])
        .output()
    {
        Ok(o) => o,
        Err(错误) => {
            warn!("调度驱动：cargo check --workspace 启动失败：{错误}");
            return false;
        }
    };
    输出.status.success()
}

/// 归档名单到 `.上下文/归档/待修正-测试标识-{时间}.jsonl`。
///
/// 归档后原名单**保留**（不删）——保留给人工复查与回溯；
/// 「清空名单」留给 §19.4.2.3 之外的单独 commit 决策（避免误清）。
fn 归档名单(名单路径: &Path) -> Result<(), String> {
    let 工作区根 = 名单路径
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| "无法推导工作区根".to_string())?;
    let 归档目录 = 工作区根.join(".上下文").join("归档");
    fs::create_dir_all(&归档目录).map_err(|e| format!("建归档目录失败: {e}"))?;
    let 时间戳 = shihai_fu::当前毫秒();
    let 归档路径 = 归档目录.join(format!("待修正-测试标识-{时间戳}.jsonl"));
    fs::copy(名单路径, &归档路径).map_err(|e| format!("归档失败: {e}"))?;
    info!(归档路径 = %归档路径.display(), "调度驱动：已归档待修正名单");
    Ok(())
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

// ─────────── §19.4.2.7 P0-3：步骤 5 完整版 4 步事务回归测试（2026-08-25）───────────
//
// 覆盖：
// - 应用动作：3 种动作（加/去 #[test] / 加注释审）
// - 路径转crate名：5 维（鸿蒙基础设施 / 鸿蒙世界配置 / 乾坤呈现 / 证道鸿蒙 / 世界）
// - 消费待修正名单 dry_run 路径：dry_run 不真改文件
//
// 依据：上下文 §8.5 + 多智能体 §19.4.2.3 + §19.4.2.7 P0-3 + AGENTS §16。

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::测试设施::工作区测试锁;

    #[test]
    fn 应用动作_加测试标记_在pubfn上方插入() {
        let 原内容 = "fn main() {}\npub fn 业务() {}\n";
        let 新内容 = 加测试标记(原内容).expect("应成功");
        assert!(
            新内容.contains("#[test]\npub fn 业务"),
            "应在 pub fn 上方插入 #[test]，实为：{新内容}"
        );
    }

    #[test]
    fn 应用动作_加测试标记_已存在则跳过() {
        let 原内容 = "#[test]\npub fn 业务() {}\n";
        let 新内容 = 加测试标记(原内容).expect("应成功");
        assert_eq!(新内容, 原内容, "已存在 #[test] 时应跳过，不重复添加");
    }

    #[test]
    fn 应用动作_去测试标记_删除第一个测试标记行() {
        let 原内容 = "#[test]\npub fn 业务() {}\n其他内容\n";
        let 新内容 = 去测试标记(原内容).expect("应成功");
        assert!(
            !新内容.starts_with("#[test]"),
            "应删除第一个 #[test] 行，实为：{新内容}"
        );
        assert!(
            新内容.contains("pub fn 业务"),
            "应保留 pub fn 业务，实为：{新内容}"
        );
    }

    #[test]
    fn 应用动作_去测试标记_不存在则返原内容() {
        let 原内容 = "pub fn 业务() {}\n";
        let 新内容 = 去测试标记(原内容).expect("应成功（无测试标记也算成功）");
        assert_eq!(新内容, 原内容, "没有 #[test] 时应返原内容（已正确）");
    }

    #[test]
    fn 应用动作_加边缘注释_插入人工可审注释() {
        let 原内容 = "pub fn 业务() {}\n";
        let 新内容 = 加边缘注释(原内容).expect("应成功");
        assert!(
            新内容.contains("§19.4.2.5 LLM 判定：边缘"),
            "应插入 LLM 边缘判定注释，实为：{新内容}"
        );
        assert!(
            新内容.contains("pub fn 业务"),
            "应保留 pub fn 业务，实为：{新内容}"
        );
    }

    #[test]
    fn 应用动作_加边缘注释_已存在则跳过() {
        let 注释 = "// §19.4.2.5 LLM 判定：边缘（人工可审）";
        let 原内容 = format!("{注释}\npub fn 业务() {{}}\n");
        let 新内容 = 加边缘注释(&原内容).expect("应成功");
        assert_eq!(新内容, 原内容, "已存在同样注释时跳过");
    }

    #[test]
    fn 路径转crate名_鸿蒙基础设施域() {
        assert_eq!(
            路径转crate名("鸿蒙/基础设施 - 域/识海承载-府/.../某.rs"),
            Some("shihai_fu".to_string())
        );
        assert_eq!(
            路径转crate名("鸿蒙/基础设施 - 域/天庭治理-府/.../某.rs"),
            Some("tianting_fu".to_string())
        );
        assert_eq!(
            路径转crate名("鸿蒙/基础设施 - 域/道术施展-府/.../某.rs"),
            Some("daoshu_fu".to_string())
        );
        assert_eq!(
            路径转crate名("鸿蒙/基础设施 - 域/观测探针-府/.../某.rs"),
            Some("jiance_fu".to_string())
        );
    }

    #[test]
    fn 路径转crate名_鸿蒙世界配置域() {
        assert_eq!(
            路径转crate名("鸿蒙/世界配置 - 域/配置管理-府/.../某.rs"),
            Some("peizhi_fu".to_string())
        );
    }

    #[test]
    fn 路径转crate名_乾坤呈现域() {
        assert_eq!(
            路径转crate名("乾坤/呈现-域/命令操作-府/.../某.rs"),
            Some("mingling_fu".to_string())
        );
    }

    #[test]
    fn 路径转crate名_证道鸿蒙域() {
        assert_eq!(
            路径转crate名("证道/鸿蒙 - 域/单元测试-府/.../某.rs"),
            Some("zhengdao_fu".to_string())
        );
    }

    #[test]
    fn 路径转crate名_世界顶级包() {
        assert_eq!(路径转crate名("世界/入口.rs"), Some("世界".to_string()));
    }

    #[test]
    fn 路径转crate名_未知路径返无() {
        assert_eq!(路径转crate名("某未知域/某府/.../某.rs"), None);
    }

    #[test]
    fn 消费待修正名单_名单不存在返空报告() {
        // 用 mingling_fu 测试设施的工作区锁序列化环境变量修改，避免并发污染。
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 临时根 = std::env::temp_dir().join(format!(
            "调度驱动-测试-不存在-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        ));
        let _ = std::fs::remove_dir_all(&临时根);
        std::fs::create_dir_all(&临时根).unwrap();
        let 原根 = std::env::var("WORLD_WORKSPACE_ROOT").ok();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &临时根);
        let 报告 = 消费待修正名单();
        // 还原（避免污染后续测试）
        match 原根 {
            Some(v) => std::env::set_var("WORLD_WORKSPACE_ROOT", v),
            None => std::env::remove_var("WORLD_WORKSPACE_ROOT"),
        }
        assert!(报告.is_none(), "名单不存在应返 None");
        let _ = std::fs::remove_dir_all(&临时根);
    }

    #[test]
    fn 消费待修正名单_dry_run_不真改文件() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 临时根 = std::env::temp_dir().join(format!(
            "调度驱动-测试-dryrun-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        ));
        let _ = std::fs::remove_dir_all(&临时根);
        std::fs::create_dir_all(临时根.join(".上下文").join("状态")).unwrap();
        std::fs::create_dir_all(临时根.join(".上下文").join("回滚垫")).unwrap();

        let 原根 = std::env::var("WORLD_WORKSPACE_ROOT").ok();
        let 原dry = std::env::var("WORLD_TEST_PATCH_DRY_RUN").ok();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &临时根);
        std::env::set_var("WORLD_TEST_PATCH_DRY_RUN", "true");

        let 目标 = 临时根.join("目标.rs");
        std::fs::write(&目标, "pub fn 业务() {}\n").unwrap();
        let 原内容 = std::fs::read_to_string(&目标).unwrap();

        let 名单路径 = 临时根
            .join(".上下文")
            .join("状态")
            .join("待修正-测试标识.jsonl");
        let 目标路径字符串 = 目标.to_string_lossy().replace('\\', "\\\\");
        let 行 = format!(
            "{{\"文件路径\":\"{}\",\"实体键\":\"目标.rs\",\"动作\":\"加#[test]\",\"执行状态\":\"待执行\",\"时间戳\":0}}",
            目标路径字符串,
        );
        std::fs::write(&名单路径, 行).unwrap();

        let 报告 = 消费待修正名单().expect("消费应返 Some");

        let 改后内容 = std::fs::read_to_string(&目标).unwrap();
        assert_eq!(改后内容, 原内容, "dry_run 不应修改文件");
        assert_eq!(报告.已跳过, 1, "dry_run 应报 1 已跳过");

        // 还原环境变量
        match 原根 {
            Some(v) => std::env::set_var("WORLD_WORKSPACE_ROOT", v),
            None => std::env::remove_var("WORLD_WORKSPACE_ROOT"),
        }
        match 原dry {
            Some(v) => std::env::set_var("WORLD_TEST_PATCH_DRY_RUN", v),
            None => std::env::remove_var("WORLD_TEST_PATCH_DRY_RUN"),
        }
        let _ = std::fs::remove_dir_all(&临时根);
    }

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
}
