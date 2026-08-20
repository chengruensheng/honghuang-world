//! 三档 - 拼装 - 园：最前/中间/最后三档投影拼装，格位独立 token 计数。
//!
//! 设计稿 §4.2 规则3 第 1 件：元数据层化初始背景。
//! 首屏只拼最前+最后档（首因+近因），按较紧的首屏预算；中间档不直接喂，
//! 由模型按需调用 读格位/查格位历史 工具展开，避免一开始就把百万字节背景灌给模型。
//! 固定规则段（§14.19 规则五分类）：按实体键分组过滤失效链（键最新失效则整键跳过）后，
//! 按规则级别排序注入（临时天道 > 天道项目 > 大道项目 > 天道全局 > 大道全局），临时天道任务级规则任务结束即失效清理。
//! 准圣验收方法论：准圣验收以产物原文摘录（预算内截断）注入提示词后核对实现细节，以真实内容为准、不凭字节数推断。
//! 六准圣审验并发执行（每准圣独立线程），各线程内先建立验收观测上下文再调用模型，模型观测按要求id正确关联。

use crate::{
    依赖图, 可见工具, 工作区, 扫描排除项, 格位, 模型存储, 范畴, 规则级别, 记录, 调用方层级,
    顺序档位,
};
use rizhi_fu::{debug, info};
use std::path::Path;

/// 元数据层化初始背景：先拼最前+最后档（首因+近因），按较紧的首屏预算；
/// 中间档不直接喂，在末尾注脚列出名字供模型按需调用 读格位/查格位历史 工具展开。
///
/// 预算建议（与设计稿 §4.2 规则3 口径一致，按字符计数）：
/// - 甲阶段首次上下文接入：首屏默认 **3000 字符**，够喂最前档铁律/价值观/身份 + 最后档最新事件；
/// - 若任务涉及大量中间档（如扫描扫描结果/教训沉淀），可调高，但超预算时应优先用 读格位 按需展开而非一次性拼装。
///
/// 继承制权限矩阵（2026-08-19 界主定义）：调用方按层级过滤可见规则——
/// 大道为底座（所有人看），天道按场景（执行看任务级，验收/终裁看项目级），临时天道任务填充。
/// `级别=None`（无规则级别）按"大道项目"（权重 3）处理——历史记录无级别字段，兼容旧数据。
fn 规则级别对调用方可见(
    级别: Option<&规则级别>, 调用方: 调用方层级
) -> bool {
    use 调用方层级::*;
    match 级别 {
        None => true,                     // 旧记录无级别→全可见（向后兼容）
        Some(规则级别::大道全局) => true, // 大道底座，所有人看
        Some(规则级别::临时天道) => true, // 任务级，任务相关方都看
        Some(规则级别::天道全局) => matches!(调用方, 设计 | 验收 | 终裁),
        Some(规则级别::大道项目) => matches!(调用方, 设计 | 验收 | 终裁),
        Some(规则级别::天道项目) => matches!(调用方, 验收 | 终裁),
    }
}

/// 与 拼装投影 的差异：拼装投影按"格位们"全量拼装，预算字符是全局兜底；
/// 元数据层化只拼首因+近因，中间档独立列出，避免中间档占用首屏配额。
///
/// workspace members + 府间依赖映射注入（M3 探索空转修复 + 府间依赖注入，2026-08-20 入稿）：
/// 从根 Cargo.toml 读 workspace members，再读各府 Cargo.toml 的 [lib] name 与 [dependencies]，
/// 构造「府→lib名→依赖列表」映射注入项目背景——让模型知道 shihai_fu 是哪个府的 lib 名、
/// tianting_fu 依赖 shihai_fu 等府间关系，不硬编码。
fn 读workspace成员() -> Option<String> {
    读workspace成员在(&工作区::定位())
}

/// 在指定工作区读 workspace members + 府间依赖映射（供测试注入临时工作区）。
fn 读workspace成员在(工作区: &工作区) -> Option<String> {
    let 根 = 工作区.根路径();
    let 内容 = std::fs::read_to_string(根.join("Cargo.toml")).ok()?;
    let members: Vec<String> = 内容
        .lines()
        .filter(|行| 行.contains("-府\""))
        .map(|行| {
            行.trim()
                .trim_start_matches('"')
                .trim_end_matches(',')
                .trim_end_matches('"')
                .to_string()
        })
        .collect();
    if members.is_empty() {
        return None;
    }
    let mut 段 = format!("\n【workspace members】{}\n", members.join("、"));
    // 府间依赖映射：读各府 Cargo.toml 的 [lib] name 与 [dependencies]。
    let mut 依赖映射 = String::from("【府间依赖】\n");
    let mut 有映射 = false;
    for member in &members {
        let 府cargo = 根.join(member).join("Cargo.toml");
        let Ok(府内容) = std::fs::read_to_string(&府cargo) else {
            continue;
        };
        let 府名 = Path::new(member)
            .file_name()
            .map(|名| 名.to_string_lossy().to_string())
            .unwrap_or_else(|| member.clone());
        let lib名 = 解析lib名(&府内容).unwrap_or_else(|| 府名.clone());
        let 依赖们 = 解析依赖段(&府内容);
        依赖映射.push_str(&format!(
            "{府名}: lib={lib名}, 依赖=[{}]\n",
            依赖们.join("、")
        ));
        有映射 = true;
    }
    if 有映射 {
        段.push_str(&依赖映射);
        debug!(府数 = members.len(), "府间依赖映射已注入");
    }
    Some(段)
}

/// 解析 Cargo.toml 的 [lib] name。
fn 解析lib名(内容: &str) -> Option<String> {
    let mut 在lib段 = false;
    for 行 in 内容.lines() {
        let 行 = 行.trim();
        if 行.starts_with('[') {
            在lib段 = 行 == "[lib]";
            continue;
        }
        if 在lib段 {
            if let Some(值) = 行.strip_prefix("name") {
                let 值 = 值.trim_start();
                if 值.starts_with('=') {
                    let 值 = 值.trim_start_matches('=').trim().trim_matches('"');
                    if !值.is_empty() {
                        return Some(值.to_string());
                    }
                }
            }
        }
    }
    None
}

/// 解析 Cargo.toml 的 [dependencies] 段依赖名列表。
fn 解析依赖段(内容: &str) -> Vec<String> {
    let mut 依赖们 = Vec::new();
    let mut 在依赖段 = false;
    for 行 in 内容.lines() {
        let 行 = 行.trim();
        if 行.starts_with('[') {
            在依赖段 = 行 == "[dependencies]";
            continue;
        }
        if 在依赖段 && !行.is_empty() && !行.starts_with('#') {
            if let Some(名) = 行
                .split(|字符: char| ['=', ' ', '{'].contains(&字符))
                .next()
            {
                let 名 = 名.trim();
                if !名.is_empty() {
                    依赖们.push(名.to_string());
                }
            }
        }
    }
    依赖们
}

/// 结构树摘要注入（执行背景注入结构树，2026-08-20 入稿）：
/// 从依赖图加载结构树渲染为文本摘要注入执行背景——让执行层知道项目整体结构
/// （府→殿→阁→园），不靠记忆猜落点。依赖图不存在或结构树为空时从工作区目录扫描生成。
fn 读结构树摘要() -> Option<String> {
    读结构树摘要在(&工作区::定位())
}

/// 在指定工作区读结构树摘要（供测试注入临时工作区）。
fn 读结构树摘要在(工作区: &工作区) -> Option<String> {
    // 优先从依赖图结构树渲染（已落盘的扫描产物，含 crate 内目录层级）。
    if let Ok(图) = 依赖图::加载自工作区(工作区) {
        let 摘要 = 图.下探(&[]);
        if !摘要.is_empty() {
            debug!(字符数 = 摘要.chars().count(), "结构树摘要已从依赖图渲染");
            return Some(format!("\n【结构树】\n{}\n", 限制字符(摘要, 2000)));
        }
    }
    // 依赖图不存在或结构树为空 → 从工作区目录扫描生成府级摘要。
    let 摘要 = 扫描结构摘要(工作区.根路径());
    if 摘要.is_empty() {
        return None;
    }
    debug!(字符数 = 摘要.chars().count(), "结构树摘要已从目录扫描生成");
    Some(format!("\n【结构树】\n{}\n", 摘要))
}

/// 从工作区目录扫描生成府级结构摘要（依赖图不存在时的兜底）。
fn 扫描结构摘要(根: &Path) -> String {
    let 排除项 = 扫描排除项(根);
    let mut crate们: Vec<String> = Vec::new();
    递归找cargo(根, 根, &排除项, &mut crate们);
    crate们.sort();
    crate们.dedup();
    crate们.join("\n")
}

/// 递归找 Cargo.toml，收集 crate 相对路径。
fn 递归找cargo(根: &Path, 目录: &Path, 排除项: &[String], 结果: &mut Vec<String>) {
    let Ok(条目们) = std::fs::read_dir(目录) else {
        return;
    };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 排除项.iter().any(|项| 项 == &名) {
                continue;
            }
            递归找cargo(根, &路径, 排除项, 结果);
        } else if 名 == "Cargo.toml" {
            if let Some(父) = 路径.parent() {
                if let Ok(相对) = 父.strip_prefix(根) {
                    let 相对 = 相对.display().to_string().replace('\\', "/");
                    if !相对.is_empty() {
                        结果.push(相对);
                    }
                }
            }
        }
    }
}

/// 限制字符串字符数，超限截断加省略号。
fn 限制字符(文本: String, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        return 文本;
    }
    let mut 截断: String = 文本.chars().take(上限).collect();
    截断.push('…');
    截断
}

pub fn 元数据层化(
    存储: &模型存储,
    角色: &str,
    格位们: &[格位],
    首屏预算字符: usize,
    调用方: 调用方层级,
) -> Result<String, String> {
    let 首屏格位们: Vec<格位> = 格位们
        .iter()
        .filter(|格位| 格位.顺序档位 != 顺序档位::中间)
        .cloned()
        .collect();
    let 中间档们: Vec<&格位> = 格位们
        .iter()
        .filter(|格位| 格位.顺序档位 == 顺序档位::中间)
        .collect();
    // 固定规则格位：范畴=规则的中间档格位（细则·解读 / 环节规则 / 铁律·总纲 / 标准 等）。
    // 作为「固定高价值信息」随背景始终加载，不被中间档"按需展开"省略——规则是硬约束，
    // 应让模型第一眼看到（设计稿 §4.2 规则7 / 项目心智模型 §6.5）。
    let 规则格位们: Vec<&格位> = 格位们
        .iter()
        .filter(|格位| 格位.范畴 == 范畴::规则 && 格位.顺序档位 == 顺序档位::中间)
        .collect();

    debug!(
        总格位 = 格位们.len(),
        首屏格位 = 首屏格位们.len(),
        中间档 = 中间档们.len(),
        规则格位 = 规则格位们.len(),
        首屏预算 = 首屏预算字符,
        "元数据层化拼装开始"
    );
    let mut 首屏 = 拼装投影(存储, 角色, &首屏格位们, 首屏预算字符)?;

    // 固定规则段：规则格位链头内容注入背景（独立小预算，单格位 300 字符，总量 1600 封顶）。
    // §14.19 规则五分类：铁律·总纲 的记录按 规则级别 分层注入——优先级 临时>项目>全局
    // （临时天道最前最醒目），内容去重（同内容只注一次，跨级别不重复）。
    // 2026-08-19 改进：总量 800→1600——细则·解读 承载「准圣审验标准清单」（可操作审验标准），
    // 须完整注入准圣提示词让世界自审；预算不足会被截断。细则·解读 优先于 铁律·总纲 注入
    // （审验标准是执行/验收的可操作约束，铁律是结构性约束，标准优先喂给模型）。
    if !规则格位们.is_empty() {
        let mut 规则段 = String::from("\n【固定规则·高价值信息】\n");
        let mut 规则已用字符 = 0usize;
        let 规则预算 = 1600usize;
        let mut 已见内容 = std::collections::HashSet::new();
        // 注入顺序：细则·解读/标准（可操作审验标准）优先，其余按原顺序。
        let mut 排序后: Vec<&格位> = 规则格位们.clone();
        排序后.sort_by_key(|格位| {
            if 格位.名字 == "细则·解读" {
                0
            } else {
                1
            }
        });
        for 格位 in 排序后 {
            // 规则格位读全量（非 读链头集——实体键分组会顶掉多条规则；规则应全部注入）。
            let mut 记录们: Vec<记录> = 存储.读格位(&格位.名字)?;
            // §14.19 实体键级失效：键最新记录失效 → 该键所有记录跳过（临时天道任务结束清理语义，
            // 防止旧的有效临时天道记录继续注入）。
            let 失效键们: std::collections::HashSet<String> = {
                let mut 键最新 = std::collections::HashMap::new();
                for 记录 in &记录们 {
                    键最新.insert(记录.实体键.clone(), 记录.失效);
                }
                键最新
                    .iter()
                    .filter(|(_, 失效)| **失效)
                    .map(|(键, _)| 键.clone())
                    .collect()
            };
            记录们.retain(|记录| !失效键们.contains(&记录.实体键));
            // 继承制权限矩阵（2026-08-19）：按调用方层级过滤铁律·总纲 规则级别——
            // 执行只看任务级（临时天道）+ 大道全局（避免项目级天道噪音）；
            // 验收/终裁看全部（含天道项目=审验标准）；设计看任务级+大道级（不含天道项目）。
            if 格位.名字 == "铁律·总纲" {
                记录们
                    .retain(|记录| 规则级别对调用方可见(记录.规则级别.as_ref(), 调用方));
                记录们.sort_by_key(|记录| {
                    记录.规则级别.as_ref().map(|级别| 级别.权重()).unwrap_or(3)
                });
            }
            for 记录 in 记录们 {
                if 记录.失效 || 记录.内容.trim().is_empty() {
                    continue;
                }
                // 去重：同内容只注入一次（§14.19 不同级别无重复——写入校验 + 注入兜底去重）。
                if !已见内容.insert(记录.内容.clone()) {
                    continue;
                }
                let 级别注 = 记录
                    .规则级别
                    .as_ref()
                    .map(|级别| format!("[{级别:?}] "))
                    .unwrap_or_default();
                let 行 = format!("【{}】{}{}\n", 格位.名字, 级别注, 记录.内容);
                if 规则已用字符 + 行.chars().count() > 规则预算 {
                    debug!(格位 = %格位.名字, 规则预算, "固定规则格位达段预算上限");
                    break;
                }
                规则已用字符 += 行.chars().count();
                规则段.push_str(&行);
            }
        }
        if 规则段.trim().len() > "【固定规则·高价值信息】".len() {
            首屏.push_str(&规则段);
        }
    }

    // workspace members + 府间依赖映射注入项目背景（M3 探索空转修复 + 府间依赖注入，2026-08-20 入稿）：
    // 从 Cargo.toml 动态读取 workspace members 与各府 [lib] name/[dependencies]，
    // 让执行者从项目结构自己分辨源码目录与构建产物目录，并知道府间依赖关系
    // （如 shihai_fu 是识海承载-府的 lib 名、tianting_fu 依赖 shihai_fu），不硬编码。
    if let Some(members段) = 读workspace成员() {
        首屏.push_str(&members段);
    }

    // 结构树摘要注入执行背景（执行背景注入结构树，2026-08-20 入稿）：
    // 从依赖图加载结构树渲染为文本摘要，让执行层知道项目整体结构（府→殿→阁→园），
    // 不靠记忆猜落点。依赖图不存在时从工作区目录扫描生成府级摘要。
    if let Some(结构树段) = 读结构树摘要() {
        首屏.push_str(&结构树段);
    }

    if !中间档们.is_empty() {
        首屏.push_str("\n【中间档（按需展开）】\n");
        首屏.push_str("以下格位未直接注入背景，必要时调用 读格位 或 查格位历史 工具按名展开：\n");
        for 格位 in 中间档们 {
            首屏.push_str(&format!("- {}（{}）\n", 格位.名字, 格位.种子提示词));
        }
    }
    info!(首屏长度 = 首屏.chars().count(), "元数据层化已拼装");
    Ok(首屏)
}

/// 拼装投影：最前 → 最后 → 中间（首因+近因优先），最前注入可用工具（按角色过滤）。
/// 每个格位按自身 token 上限独立计数，达到上限即停该格位，不挤占其他格位；
/// 全局按预算字符兜底，防总输出超限（与格位独立上限同口径，设计稿 §4.2 规则3）。
pub fn 拼装投影(
    存储: &模型存储,
    角色: &str,
    格位们: &[格位],
    预算字符: usize,
) -> Result<String, String> {
    let mut 输出 = String::new();
    // 元信息前缀：可用工具（按角色过滤，模型第一眼就知道世界给不给手脚）
    let 工具们 = 可见工具(角色);
    if !工具们.is_empty() {
        输出.push_str(&format!("【可用工具】{}\n", 工具们.join("、")));
        debug!(角色, 工具数 = 工具们.len(), "工具清单已注入投影");
    }

    let mut 顺序: Vec<&格位> = Vec::new();
    顺序.extend(格位们.iter().filter(|格位| 格位.顺序档位 == 顺序档位::最前));
    顺序.extend(格位们.iter().filter(|格位| 格位.顺序档位 == 顺序档位::最后));
    顺序.extend(格位们.iter().filter(|格位| 格位.顺序档位 == 顺序档位::中间));

    for 格位 in 顺序 {
        let 上限 = 格位.token上限;
        let mut 已用字符 = 0usize;
        for 记录 in 存储.读链头集(&格位.名字)? {
            let 行 = format!("【{}】{}（证据：{}）\n", 格位.名字, 记录.内容, 记录.证据);
            // 格位独立计数：达到自身 token 上限即停该格位，不影响其他格位
            if 已用字符 + 行.chars().count() > 上限 {
                debug!(格位 = %格位.名字, 上限, 已用字符, "格位达独立上限");
                break;
            }
            // 全局兜底：总输出超预算字符即整体停（与设计稿 3000 字符口径一致）
            if 输出.chars().count() + 行.chars().count() > 预算字符 {
                debug!(
                    格位数 = 格位们.len(),
                    预算字符,
                    输出长度 = 输出.chars().count(),
                    "投影按全局预算截断"
                );
                return Ok(输出);
            }
            已用字符 += 行.chars().count();
            输出.push_str(&行);
        }
    }
    debug!(
        格位数 = 格位们.len(),
        输出长度 = 输出.chars().count(),
        "投影已拼装"
    );
    Ok(输出)
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::{共享度, 固化度, 结构节点, 范畴, 记录};

    /// 设计稿 §4.2 规则3：首屏预算按字符计数（3000 字符口径），全局兜底紧额时应截断输出。
    #[test]
    fn 元数据层化_按字符预算截断() {
        let 目录 = std::env::temp_dir().join(format!("三档拼装-测试-{}", std::process::id()));
        let 存储 = 模型存储::打开(&目录);
        let 格位 = 格位::新(
            "测试-格位",
            范畴::规则,
            "测试摘要",
            "设计稿",
            固化度::权,
            共享度::私有,
            顺序档位::最前,
        );
        for i in 0..5 {
            存储
                .写记录(&记录::新(
                    "测试-格位",
                    &format!("第{i}条：天地玄黄宇宙洪荒，机制叠加验证字符预算截断行为。"),
                    "测试证据",
                    "设计稿",
                ))
                .unwrap();
        }
        let 全量 = 元数据层化(
            &存储,
            "多宝",
            std::slice::from_ref(&格位),
            100_000,
            调用方层级::验收,
        )
        .unwrap();
        let 紧额 = 元数据层化(
            &存储,
            "多宝",
            std::slice::from_ref(&格位),
            50,
            调用方层级::验收,
        )
        .unwrap();
        assert!(全量.contains("第4条"), "大预算应收全部记录，缺失：{全量}");
        assert!(
            紧额.chars().count() < 全量.chars().count(),
            "紧额预算应按字符截断：{} vs {}",
            紧额.chars().count(),
            全量.chars().count()
        );
        let _ = std::fs::remove_dir_all(&目录);
    }

    /// 固定规则段（设计稿 §4.2 规则7）：范畴=规则的中间档格位列链头内容注入背景，
    /// 即使其顺序档位=中间也不被省略——规则是固定高价值信息。
    #[test]
    fn 元数据层化_固定规则段注入规则格位() {
        let 目录 = std::env::temp_dir().join(format!("三档拼装-规则-{}", std::process::id()));
        let 存储 = 模型存储::打开(&目录);
        // 造一个 范畴=规则、顺序=中间 的格位，模拟 细则·解读。
        let 规则格位 = 格位::新(
            "细则·解读",
            范畴::规则,
            "可操作规范与解读",
            "LLM",
            固化度::权,
            共享度::共享,
            顺序档位::中间,
        );
        存储
            .写记录(&记录::新(
                "细则·解读",
                "全中文输出纪律：所有输出用简体中文",
                "界主规则",
                "人类",
            ))
            .unwrap();
        // 造一个 非规则 中间档格位（如 结构 = 范畴::世界），不应进固定规则段。
        let 世界格位 = 格位::新(
            "结构",
            范畴::世界,
            "目录组织",
            "代码",
            固化度::权,
            共享度::共享,
            顺序档位::中间,
        );
        存储
            .写记录(&记录::新("结构", "目录快照", "扫描", "代码"))
            .unwrap();

        let 背景 = 元数据层化(
            &存储,
            "多宝",
            &[规则格位, 世界格位],
            100_000,
            调用方层级::验收,
        )
        .unwrap();
        assert!(背景.contains("固定规则"), "应含固定规则段：{背景}");
        assert!(
            背景.contains("全中文输出纪律"),
            "应注入规则格位内容：{背景}"
        );
        assert!(
            !背景.contains("目录快照"),
            "非规则中间档(世界)不应进固定规则段：{背景}"
        );
        let _ = std::fs::remove_dir_all(&目录);
    }

    /// 固定规则段注入顺序（2026-08-19 改进）：细则·解读（可操作审验标准）优先于
    /// 铁律·总纲——预算有限时先保证审验标准完整注入，模型第一眼看到可操作约束。
    #[test]
    fn 元数据层化_细则解读优先于铁律注入() {
        let 目录 = std::env::temp_dir().join(format!("三档拼装-优先-{}", std::process::id()));
        let 存储 = 模型存储::打开(&目录);
        let 细则 = 格位::新(
            "细则·解读",
            范畴::规则,
            "可操作规范",
            "LLM",
            固化度::权,
            共享度::共享,
            顺序档位::中间,
        );
        let 铁律 = 格位::新(
            "铁律·总纲",
            范畴::规则,
            "最高准则",
            "人类",
            固化度::经,
            共享度::共享,
            顺序档位::中间,
        );
        // 细则·解读 内容较长（模拟审验标准清单），铁律 内容短。
        let 长细则 = format!("审验标准A：{} 审验标准Z", "长内容".repeat(200));
        存储
            .写记录(&记录::新("细则·解读", &长细则, "界主", "人类"))
            .unwrap();
        存储
            .写记录(&记录::新("铁律·总纲", "不可破硬约束", "界主", "人类"))
            .unwrap();
        let 背景 = 元数据层化(&存储, "多宝", &[细则, 铁律], 100_000, 调用方层级::验收).unwrap();
        // 细则·解读 完整注入（预算 1600 内），且排在 铁律·总纲 之前。
        let 细则位 = 背景.find("审验标准A").unwrap();
        let 铁律位 = 背景.find("不可破硬约束").unwrap();
        assert!(细则位 < 铁律位, "细则·解读 应优先于 铁律·总纲 注入");
        assert!(
            背景.contains("审验标准Z"),
            "细则·解读 长内容应完整注入（预算内）"
        );
        let _ = std::fs::remove_dir_all(&目录);
    }

    /// 固定规则段空记录零开销：规则格位无记录时无「固定规则」段头。
    #[test]
    fn 元数据层化_固定规则段空记录不注入() {
        let 目录 = std::env::temp_dir().join(format!("三档拼装-规则空-{}", std::process::id()));
        let 存储 = 模型存储::打开(&目录);
        let 规则格位 = 格位::新(
            "细则·解读",
            范畴::规则,
            "可操作规范",
            "LLM",
            固化度::权,
            共享度::共享,
            顺序档位::中间,
        );
        let 背景 = 元数据层化(&存储, "多宝", &[规则格位], 100_000, 调用方层级::验收).unwrap();
        assert!(
            !背景.contains("固定规则"),
            "空规则格位不应出固定规则段：{背景}"
        );
        let _ = std::fs::remove_dir_all(&目录);
    }

    /// 继承制权限矩阵（2026-08-19 界主定义）：执行只见任务级（临时天道）+ 大道全局底座，
    /// 看不到项目级天道（天道项目/全局）——避免执行层被审验标准噪音淹没。
    #[test]
    fn 元数据层化_执行只见任务级与大道全局() {
        use crate::规则级别;
        let 目录 = std::env::temp_dir().join(format!("三档拼装-权限矩阵-{}", std::process::id()));
        let 存储 = 模型存储::打开(&目录);
        let 铁律 = 格位::新(
            "铁律·总纲",
            范畴::规则,
            "最高准则",
            "人类",
            固化度::经,
            共享度::共享,
            顺序档位::中间,
        );
        // 大道全局：所有人看（含执行）
        存储
            .写记录(&记录::新带级别(
                "铁律·总纲",
                "跨府引用只认 lib 根",
                "界主",
                "人类",
                规则级别::大道全局,
            ))
            .unwrap();
        // 大道项目：执行不看（项目级约束）
        存储
            .写记录(&记录::新带级别(
                "铁律·总纲",
                "本项目全中文输出",
                "界主",
                "人类",
                规则级别::大道项目,
            ))
            .unwrap();
        // 天道项目（审验标准）：执行不看
        存储
            .写记录(&记录::新带级别(
                "铁律·总纲",
                "审验标准：产物须真实达成要求",
                "界主",
                "人类",
                规则级别::天道项目,
            ))
            .unwrap();
        // 临时天道：执行看
        存储
            .写记录(&记录::新带级别(
                "铁律·总纲",
                "本任务只改三档拼装.rs",
                "界主",
                "人类",
                规则级别::临时天道,
            ))
            .unwrap();

        let 执行背景 = 元数据层化(
            &存储,
            "多宝",
            std::slice::from_ref(&铁律),
            100_000,
            调用方层级::执行,
        )
        .unwrap();
        // 执行：可见 大道全局 + 临时天道
        assert!(
            执行背景.contains("跨府引用只认 lib 根"),
            "大道全局应注入：{执行背景}"
        );
        assert!(
            执行背景.contains("本任务只改三档拼装"),
            "临时天道应注入：{执行背景}"
        );
        // 执行：不可见 大道项目 + 天道项目（项目级天道对执行无意义）
        assert!(
            !执行背景.contains("本项目全中文输出"),
            "执行不应见大道项目（项目级约束）：{执行背景}"
        );
        assert!(
            !执行背景.contains("审验标准：产物须真实达成要求"),
            "执行不应见天道项目（审验标准是给验收层）：{执行背景}"
        );

        let 验收背景 = 元数据层化(
            &存储,
            "多宝",
            std::slice::from_ref(&铁律),
            100_000,
            调用方层级::验收,
        )
        .unwrap();
        // 验收：可见全部（含项目级审验标准）
        assert!(
            验收背景.contains("本项目全中文输出"),
            "验收应见大道项目：{验收背景}"
        );
        assert!(
            验收背景.contains("审验标准：产物须真实达成要求"),
            "验收应见天道项目：{验收背景}"
        );

        let _ = std::fs::remove_dir_all(&目录);
    }

    /// §14.19：铁律·总纲 带级别记录注入——临时天道最前（优先级高），失效记录被过滤。
    #[test]
    fn 元数据层化_铁律按级别注入且失效过滤() {
        use crate::规则级别;
        let 目录 = std::env::temp_dir().join(format!("三档拼装-级别-{}", std::process::id()));
        let 存储 = 模型存储::打开(&目录);
        let 铁律 = 格位::新(
            "铁律·总纲",
            范畴::规则,
            "最高准则",
            "人类",
            固化度::经,
            共享度::共享,
            顺序档位::中间,
        );
        // 大道项目（宪法）与 临时天道（任务级）各两条。
        // 临时天道用独立实体键"临时天道"（与守卫一致）：任务结束写失效标记后，
        // 实体键级失效判定可跳过该键全部旧记录（§14.19 清理语义）。
        存储
            .写记录(&记录::新带级别(
                "铁律·总纲",
                "全项目使用中文",
                "界主",
                "人类",
                规则级别::大道项目,
            ))
            .unwrap();
        let mut 临时1 = 记录::新带级别(
            "铁律·总纲",
            "本任务只改指定文件",
            "任务规则",
            "人类",
            规则级别::临时天道,
        );
        临时1.实体键 = "临时天道".to_string();
        存储.写记录(&临时1).unwrap();
        // 阶段1：大道项目（宪法）与 临时天道（任务级）两条有效记录。
        存储
            .写记录(&记录::新带级别(
                "铁律·总纲",
                "全项目使用中文",
                "界主",
                "人类",
                规则级别::大道项目,
            ))
            .unwrap();
        let mut 临时2 = 记录::新带级别(
            "铁律·总纲",
            "本任务只改指定文件",
            "任务规则",
            "人类",
            规则级别::临时天道,
        );
        临时2.实体键 = "临时天道".to_string();
        存储.写记录(&临时2).unwrap();

        let 背景 = 元数据层化(
            &存储,
            "多宝",
            std::slice::from_ref(&铁律),
            100_000,
            调用方层级::验收,
        )
        .unwrap();
        assert!(
            背景.contains("全项目使用中文"),
            "应注入大道项目规则：{背景}"
        );
        assert!(
            背景.contains("本任务只改指定文件"),
            "应注入临时天道规则：{背景}"
        );
        // 临时天道在最前（优先级高）：找两条规则的相对位置。
        let 临时位 = 背景.find("本任务只改指定文件").unwrap();
        let 大道位 = 背景.find("全项目使用中文").unwrap();
        assert!(临时位 < 大道位, "临时天道应在最前（优先级高）");

        // 阶段2：任务结束写失效标记（实体键=临时天道，与守卫一致）→ 后续注入过滤。
        let mut 失效 = 记录::新带级别(
            "铁律·总纲",
            "已清理的临时规则",
            "任务结束",
            "代码",
            规则级别::临时天道,
        );
        失效.失效 = true;
        失效.实体键 = "临时天道".to_string();
        存储.写记录(&失效).unwrap();
        let 背景后 = 元数据层化(&存储, "多宝", &[铁律], 100_000, 调用方层级::验收).unwrap();
        assert!(
            背景后.contains("全项目使用中文"),
            "清理临时天道不应影响大道项目：{背景后}"
        );
        assert!(
            !背景后.contains("已清理的临时规则"),
            "失效记录不应注入：{背景后}"
        );
        assert!(
            !背景后.contains("本任务只改指定文件"),
            "临时天道失效后不应再注入：{背景后}"
        );
        let _ = std::fs::remove_dir_all(&目录);
    }

    /// 府间依赖映射（问题6，2026-08-20 入稿）：读各府 Cargo.toml 的 [lib] name 与
    /// [dependencies]，构造「府→lib名→依赖列表」映射注入项目背景。
    /// 验证 shihai_fu 是识海承载-府的 lib 名、tianting_fu 依赖 shihai_fu 等府间关系。
    #[test]
    fn 读workspace成员_府间依赖映射正确() {
        let 根 = std::env::temp_dir().join(format!(
            "三档拼装-依赖映射-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&根);
        // 根 Cargo.toml：两个府成员。
        std::fs::create_dir_all(&根).unwrap();
        std::fs::write(
            根.join("Cargo.toml"),
            "[workspace]\nresolver = \"2\"\nmembers = [\n    \"鸿蒙/基础设施 - 域/识海承载-府\",\n    \"鸿蒙/基础设施 - 域/天庭治理-府\",\n]\n",
        )
        .unwrap();
        // 识海承载-府：lib=shihai_fu，依赖 serde + rizhi_fu。
        let 识海 = 根.join("鸿蒙/基础设施 - 域/识海承载-府");
        std::fs::create_dir_all(&识海).unwrap();
        std::fs::write(
            识海.join("Cargo.toml"),
            "[package]\nname = \"shihai_fu\"\n[lib]\nname = \"shihai_fu\"\n[dependencies]\nserde = \"1\"\nrizhi_fu = { path = \"../日志记录-府\" }\n",
        )
        .unwrap();
        // 天庭治理-府：lib=tianting_fu，依赖 shihai_fu（府间依赖）。
        let 天庭 = 根.join("鸿蒙/基础设施 - 域/天庭治理-府");
        std::fs::create_dir_all(&天庭).unwrap();
        std::fs::write(
            天庭.join("Cargo.toml"),
            "[package]\nname = \"tianting_fu\"\n[lib]\nname = \"tianting_fu\"\n[dependencies]\nserde = \"1\"\nshihai_fu = { path = \"../识海承载-府\" }\n",
        )
        .unwrap();

        let 工作区 = 工作区::新(&根);
        let 段 = 读workspace成员在(&工作区).expect("应读出 workspace members 与府间依赖");
        // workspace members 段。
        assert!(段.contains("workspace members"), "应含段头：{段}");
        assert!(段.contains("识海承载-府"), "应含府名：{段}");
        // 府间依赖映射段。
        assert!(段.contains("府间依赖"), "应含府间依赖段头：{段}");
        assert!(段.contains("lib=shihai_fu"), "应含 shihai_fu lib 名：{段}");
        assert!(
            段.contains("lib=tianting_fu"),
            "应含 tianting_fu lib 名：{段}"
        );
        assert!(段.contains("serde"), "应含 serde 依赖：{段}");
        assert!(段.contains("rizhi_fu"), "应含 rizhi_fu 依赖：{段}");
        // 天庭治理-府 依赖 shihai_fu（府间依赖关系）。
        assert!(段.contains("shihai_fu"), "应含府间依赖 shihai_fu：{段}");
        let _ = std::fs::remove_dir_all(&根);
    }

    /// 府间依赖映射·无府成员时返回 None（零开销不注入）。
    #[test]
    fn 读workspace成员_无府成员返回空() {
        let 根 = std::env::temp_dir().join(format!(
            "三档拼装-空workspace-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();
        // 根 Cargo.toml：无 -府 结尾成员。
        std::fs::write(
            根.join("Cargo.toml"),
            "[workspace]\nmembers = [\"某-园\"]\n",
        )
        .unwrap();
        let 工作区 = 工作区::新(&根);
        assert!(读workspace成员在(&工作区).is_none(), "无府成员应返回 None");
        let _ = std::fs::remove_dir_all(&根);
    }

    /// 结构树摘要·依赖图存在时从结构树渲染（问题10，2026-08-20 入稿）：
    /// 验证依赖图结构树非空时渲染为文本摘要，含府→殿层级。
    #[test]
    fn 读结构树摘要_依赖图存在时非空() {
        let 根 = std::env::temp_dir().join(format!(
            "三档拼装-结构树-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();
        // 构造依赖图 json：结构树含 识海承载-府 → 识海-回想-殿。
        let 上下文 = 根.join(".上下文");
        std::fs::create_dir_all(&上下文).unwrap();
        let 图 = 依赖图 {
            档案们: vec![],
            结构树: 结构节点 {
                名字: "根".to_string(),
                子节点: vec![结构节点 {
                    名字: "识海承载-府".to_string(),
                    子节点: vec![结构节点::新("识海-回想-殿")],
                }],
            },
        };
        图.保存(上下文.join("依赖图.json")).unwrap();

        let 工作区 = 工作区::新(&根);
        let 段 = 读结构树摘要在(&工作区).expect("应读出结构树摘要");
        assert!(段.contains("结构树"), "应含段头：{段}");
        assert!(段.contains("识海承载-府"), "应含府名：{段}");
        assert!(段.contains("识海-回想-殿"), "应含殿名：{段}");
        let _ = std::fs::remove_dir_all(&根);
    }

    /// 结构树摘要·依赖图不存在时从目录扫描生成兜底：
    /// 验证无依赖图时扫描工作区 Cargo.toml 生成府级摘要。
    #[test]
    fn 读结构树摘要_依赖图不存在时从目录扫描() {
        let 根 = std::env::temp_dir().join(format!(
            "三档拼装-结构树兜底-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();
        // 不写依赖图，只写 Cargo.toml（模拟未扫描的项目）。
        let 府 = 根.join("鸿蒙/基础设施 - 域/识海承载-府");
        std::fs::create_dir_all(&府).unwrap();
        std::fs::write(府.join("Cargo.toml"), "[package]\nname = \"shihai_fu\"\n").unwrap();

        let 工作区 = 工作区::新(&根);
        let 段 = 读结构树摘要在(&工作区).expect("应从目录扫描生成结构摘要");
        assert!(段.contains("结构树"), "应含段头：{段}");
        assert!(段.contains("识海承载-府"), "应含府路径：{段}");
        let _ = std::fs::remove_dir_all(&根);
    }

    /// 结构树摘要·空工作区返回 None（零开销不注入）。
    #[test]
    fn 读结构树摘要_空工作区返回空() {
        let 根 = std::env::temp_dir().join(format!(
            "三档拼装-空结构-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();
        // 空工作区，无 Cargo.toml，无依赖图。
        let 工作区 = 工作区::新(&根);
        assert!(读结构树摘要在(&工作区).is_none(), "空工作区应返回 None");
        let _ = std::fs::remove_dir_all(&根);
    }
}
