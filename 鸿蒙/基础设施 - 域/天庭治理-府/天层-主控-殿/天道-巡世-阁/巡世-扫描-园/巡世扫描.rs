//! 巡世 - 扫描 - 园：扫描世界，产出巡世报告与违逆清单。
//!
//! 四项检查统一入口（设计稿 §12 P2-7，Line 1114）：
//! ① 园无测试检测 ② clippy 警告检测 ③ 教训重复模式检测 ④ 规模启发。

use crate::类型_定义_殿::{
    优先级, 巡世候选, 巡世报告, 本质档位, 本质类别, 法则违逆, 要求类别,
};
use rizhi_fu::{info, warn};
use shihai_fu::{工作区, 扫描违逆};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

/// 园无测试检测跳过项：证道 / 单元测试-府 本就是测试集合，不要求园内再嵌测试。
const 园无测试跳过: &[&str] = &["证道", "单元测试-府"];

/// 本质打分：把扫描函数产出的候选打上 本质类别 + 本质档位 标签。
/// 依据：多智能体架构设计.md §19.2 18 类本质 → 12 档映射。
/// 稳态映射：纯函数，候选文本/依据不再做语义分析（避免 LLM 漂移）。
pub fn 本质打分(目标: &str, 依据: &str) -> (本质类别, 本质档位) {
    // 优先依据文本特征（更稳定），其次目标关键词
    if 依据.contains("道韵违逆") {
        return (本质类别::规则违逆, 本质档位::S11);
    }
    if 目标.starts_with("补齐占位") {
        return (本质类别::占位骨架, 本质档位::S11);
    }
    if 目标.contains("clippy警告") {
        // clippy 警告：当前零警告为硬要求（AGENTS 第 15 条），故升档到 S10 资源耗尽预防（防止积累）
        return (本质类别::资源耗尽预防, 本质档位::S10);
    }
    if 目标.contains("反复出现") {
        return (本质类别::失败模式反复, 本质档位::S11);
    }
    if 目标.contains("项目规模较大") {
        return (本质类别::质量改进, 本质档位::S11);
    }
    // 兜底：补测试类
    (本质类别::覆盖率不足, 本质档位::S11)
}

/// 扫描世界：五项检查统一入口（§十三 道韵接入），汇聚候选与违逆。
///
/// 五项检查：
/// ① 园无测试检测 ② clippy 警告检测 ③ 教训重复模式检测 ④ 规模启发 ⑤ **道韵违逆**。
pub fn 扫描世界(根目录: &Path) -> 巡世报告 {
    let 文件们 = 收集源文件(根目录);
    let mut 候选 = Vec::new();
    候选.extend(检园无测试(根目录));
    候选.extend(检clippy警告(根目录));
    候选.extend(检教训重复(根目录));
    候选.extend(检规模(&文件们));
    候选.extend(检占位骨架(&文件们));
    let (道韵候选们, 法则违逆们) = 检道韵违逆(根目录);
    候选.extend(道韵候选们);
    let 时间 = shihai_fu::当前毫秒();
    info!(
        根 = %根目录.display(),
        源文件数 = 文件们.len(),
        候选数 = 候选.len(),
        法则违逆数 = 法则违逆们.len(),
        "巡世扫描完成"
    );
    巡世报告 {
        id: format!("巡世-{时间}"),
        时间,
        候选,
        违逆: 法则违逆们,
    }
}

/// ① 园无测试检测：园目录下 .rs 未含测试标记（跳过 证道/单元测试-府）→ 产候选，优先级=中。
/// 园内全无测试时再查证道域（单元测试-府）是否有按园名关键词匹配的测试文件，有则跳过。
fn 检园无测试(根目录: &Path) -> Vec<巡世候选> {
    let 排除项 = shihai_fu::扫描排除项(根目录);
    let mut 园们 = Vec::new();
    找园(根目录, &排除项, &mut 园们);
    let 证道测试们 = 收集证道测试文件(根目录);
    let mut 候选 = Vec::new();
    for 园路径 in &园们 {
        // 跳过 证道/单元测试-府 测试域。
        let 相对 = 园路径.strip_prefix(根目录).unwrap_or(园路径);
        let 相对串 = 相对.to_string_lossy();
        if 园无测试跳过.iter().any(|项| 相对串.contains(项)) {
            continue;
        }
        let rs们 = 收集园下rs(园路径);
        if rs们.is_empty() {
            continue;
        }
        let 全无测试 = rs们.iter().all(|rs| !含测试标记(rs));
        if 全无测试 {
            let 园名 = 园路径
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // 证道域已有对应测试 → 跳过（测试约定在证道域而非园内）。
            if 证道已测(&园名, &证道测试们) {
                continue;
            }
            let (本质类别, 本质档位) = 本质打分(
                &format!("为{园名}生产模块追加单元测试"),
                &format!("园路径 {}", 园路径.display()),
            );
            候选.push(巡世候选 {
                目标: format!("为{园名}生产模块追加单元测试"),
                依据: format!("园路径 {}", 园路径.display()),
                建议类别: 要求类别::维护,
                优先级: 优先级::中,
                本质类别,
                本质档位,
            });
        }
    }
    if !候选.is_empty() {
        info!(无测试园数 = 候选.len(), "园无测试检测完成");
    }
    候选
}

/// 递归找园目录（路径含「园」的目录）。
fn 找园(当前: &Path, 排除项: &[String], 结果: &mut Vec<PathBuf>) {
    let Ok(条目们) = std::fs::read_dir(当前) else {
        return;
    };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if !路径.is_dir() || 排除项.iter().any(|项| 项 == &名) {
            continue;
        }
        if 名.contains('园') {
            结果.push(路径.clone());
        }
        // 继续下探找嵌套园（园是六层最末层，但保险起见不截断递归）。
        找园(&路径, 排除项, 结果);
    }
}

/// 收集园直接子 .rs 文件（园是末层，不递归）。
fn 收集园下rs(园路径: &Path) -> Vec<PathBuf> {
    let Ok(条目们) = std::fs::read_dir(园路径) else {
        return Vec::new();
    };
    条目们
        .flatten()
        .filter(|条目| 条目.file_name().to_string_lossy().ends_with(".rs"))
        .map(|条目| 条目.path())
        .collect()
}

/// 检查 .rs 是否含测试标记：`#[test]` / `#[cfg(test)]` / `mod 测试`。
/// 逐行扫描短路：命中即返回，无需读全文；复用行缓冲避免逐行 String 分配（热路径 IO 优化）。
/// 等价于「前 N 行命中快速判断 + 否则读到尾部确认」，且无两次读。
fn 含测试标记(rs: &Path) -> bool {
    let Ok(文件) = std::fs::File::open(rs) else {
        return false;
    };
    let mut 读器 = std::io::BufReader::new(文件);
    let mut 行 = Vec::new();
    loop {
        行.clear();
        match 读器.read_until(b'\n', &mut 行) {
            Ok(0) | Err(_) => return false,
            Ok(_) => {}
        }
        if let Ok(文本) = std::str::from_utf8(&行) {
            if 文本.contains("#[test]")
                || 文本.contains("#[cfg(test)]")
                || 文本.contains("mod 测试")
            {
                return true;
            }
        }
    }
}

/// 收集证道域下所有 .rs 文件路径，用于按园名匹配已有测试。
/// 测试约定落在 `证道/` 而非园内，预收集一次供各园复用。
fn 收集证道测试文件(根目录: &Path) -> Vec<PathBuf> {
    let 证道根 = 根目录.join("证道");
    if !证道根.exists() {
        return Vec::new();
    }
    let mut 结果 = Vec::new();
    递归收集rs(&证道根, &mut 结果);
    结果
}

/// 递归收集目录下所有 .rs 文件。
fn 递归收集rs(目录: &Path, 结果: &mut Vec<PathBuf>) {
    let Ok(条目们) = std::fs::read_dir(目录) else {
        return;
    };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        if 路径.is_dir() {
            递归收集rs(&路径, 结果);
        } else if 路径
            .file_name()
            .map(|n| n.to_string_lossy().ends_with(".rs"))
            .unwrap_or(false)
        {
            结果.push(路径);
        }
    }
}

/// 证道域是否已有对应园名的测试：按园名关键词匹配证道测试文件路径。
/// 园名 `即时-应答-园` → 基名 `即时-应答` 与去连字符 `即时应答`，证道路径含任一即认为已测。
fn 证道已测(园名: &str, 证道测试们: &[PathBuf]) -> bool {
    let 基名 = 园名.trim_end_matches('园').trim_end_matches('-');
    let 去连字符 = 基名.replace('-', "");
    证道测试们.iter().any(|路径| {
        let 路径串 = 路径.to_string_lossy();
        路径串.contains(基名) || 路径串.contains(&去连字符)
    })
}

/// ② clippy 警告检测：跑 cargo clippy，有警告 → 产候选，优先级=中。
/// 仅在 cargo workspace 根（含 Cargo.toml）才跑，避免在临时目录或非 workspace 路径误调。
/// 带超时（60 秒）与 --offline，防 clippy 挂死巡世线程、减少外部依赖执行（安全报告 L5）。
fn 检clippy警告(根目录: &Path) -> Vec<巡世候选> {
    if !根目录.join("Cargo.toml").exists() {
        return Vec::new();
    }
    let 参数们 = [
        "clippy",
        "--workspace",
        "--all-targets",
        "--message-format=short",
        "--offline",
    ];
    let 工作目录 = 根目录.to_str().unwrap_or(".");
    let 输出 = daoshu_fu::运行命令超时("cargo", &参数们, Some(工作目录), 60_000, &[]);
    let Ok(结果) = 输出 else {
        warn!("clippy 调用失败或超时，跳过 clippy 警告检测");
        return Vec::new();
    };
    let 合并 = format!("{}\n{}", 结果.标准输出, 结果.标准错误);
    let 警告数 = 合并.lines().filter(|行| 行.contains("warning:")).count();
    if 警告数 > 0 {
        info!(警告数, "clippy 警告检测完成");
        let (本质类别, 本质档位) = 本质打分(
            &format!("清理clippy警告（{}条）", 警告数),
            &format!(
                "cargo clippy --workspace --all-targets 输出 {} 条 warning",
                警告数
            ),
        );
        vec![巡世候选 {
            目标: format!("清理clippy警告（{警告数}条）"),
            依据: format!("cargo clippy --workspace --all-targets 输出 {警告数} 条 warning"),
            建议类别: 要求类别::优化,
            优先级: 优先级::中,
            本质类别,
            本质档位,
        }]
    } else {
        Vec::new()
    }
}

/// ③ 教训重复模式检测：读教训格位，按内容前 40 字符分组，同组 ≥3 → 产候选，优先级=高。
fn 检教训重复(根目录: &Path) -> Vec<巡世候选> {
    let 格位目录 = 根目录.join(".上下文").join("格位");
    let 存储 = shihai_fu::模型存储::打开(格位目录);
    let 记录们 = match 存储.读格位(shihai_fu::教训格位) {
        Ok(记录们) => 记录们,
        Err(错误) => {
            warn!(错误 = %错误, "读教训格位失败，跳过教训重复检测");
            return Vec::new();
        }
    };
    let mut 分组: HashMap<String, Vec<String>> = HashMap::new();
    for 记录 in &记录们 {
        if !shihai_fu::是有效教训(记录) {
            continue;
        }
        let 前缀: String = 记录.内容.chars().take(40).collect();
        分组.entry(前缀).or_default().push(记录.内容.clone());
    }
    let mut 候选 = Vec::new();
    for (前缀, 同组) in &分组 {
        if 同组.len() >= 3 {
            let 摘要: String = 前缀.chars().take(20).collect();
            let (本质类别, 本质档位) = 本质打分(
                &format!("{摘要}反复出现（{}次），需系统性修复", 同组.len()),
                &format!("教训格位同前缀（前 40 字符）记录 {} 条", 同组.len()),
            );
            候选.push(巡世候选 {
                目标: format!("{摘要}反复出现（{}次），需系统性修复", 同组.len()),
                依据: format!("教训格位同前缀（前 40 字符）记录 {} 条", 同组.len()),
                建议类别: 要求类别::维护,
                优先级: 优先级::高,
                本质类别,
                本质档位,
            });
        }
    }
    if !候选.is_empty() {
        info!(重复模式数 = 候选.len(), "教训重复检测完成");
    }
    候选
}

/// ④ 规模启发：源文件数 > 200 → 产候选，优先级=低。
fn 检规模(文件们: &[PathBuf]) -> Vec<巡世候选> {
    if 文件们.len() > 200 {
        let (本质类别, 本质档位) = 本质打分(
            "项目规模较大，考虑按域拆分为更多府",
            &format!("源文件数 {}", 文件们.len()),
        );
        vec![巡世候选 {
            目标: "项目规模较大，考虑按域拆分为更多府".to_string(),
            依据: format!("源文件数 {}", 文件们.len()),
            建议类别: 要求类别::优化,
            优先级: 优先级::低,
            本质类别,
            本质档位,
        }]
    } else {
        Vec::new()
    }
}

/// ⑥ 占位·骨架扫描（2026-08-24 界主拍板，设计稿 §12 P2-7 第六项检查）。
/// 扫源码占位/待实装/未启用等「未完成态」标记，产「补齐占位」候选，优先级=高。
/// 给巡世补一双看半成品的眼睛：此前巡世只扫「缺测试」，乙阶段空转于补测试，
/// 本检查把自动优化引向真正未完成的工作。零占位 = 世界无半成品。
fn 检占位骨架(文件们: &[PathBuf]) -> Vec<巡世候选> {
    // 标记词收敛为「真未完成」特征，避免误伤合法业务语义：
    // - 不用宽泛「占位」（会误伤 提示词模板的「占位符 {背景}/{目标}」、识海「摘要占位/文件级占位档案」、
    //   以及「阶段一占位(已在调度要求内完成)」这类"功能已做完、注释未删"的残留）；
    // - 不用「待实现」单独词（会误伤 状态机枚举变体 要求状态::待实现）。
    // 只认「明确未实装/待办」的组合词 + Rust 标准未完成宏。
    const 占位标记: &[&str] = &[
        "服务占位",
        "占位查询",
        "待实装",
        "未启用",
        "只占位",
        "此处只占位",
        "仅占位",
        "留待",
        "TODO",
        "unimplemented!",
        "todo!",
    ];
    let mut 候选 = Vec::new();
    for 文件 in 文件们 {
        let Ok(内容) = std::fs::read_to_string(文件) else {
            continue;
        };
        for 标记 in 占位标记 {
            if 内容.contains(标记) {
                let 短路径 = 文件
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let (本质类别, 本质档位) = 本质打分(
                    &format!("补齐占位（{}）：{}", 标记, 短路径),
                    &format!("文件 {}", 文件.display()),
                );
                候选.push(巡世候选 {
                    目标: format!("补齐占位（{}）：{}", 标记, 短路径),
                    依据: format!("文件 {}", 文件.display()),
                    建议类别: 要求类别::补基础,
                    优先级: 优先级::高,
                    本质类别,
                    本质档位,
                });
                break;
            }
        }
    }
    if !候选.is_empty() {
        info!(占位数 = 候选.len(), "占位骨架检测完成");
    }
    候选
}

/// 递归收集 .rs 源文件，跳过排除项。
fn 收集源文件(根目录: &Path) -> Vec<PathBuf> {
    let 排除项 = shihai_fu::扫描排除项(根目录);
    let mut 结果 = Vec::new();
    递归(根目录, &排除项, &mut 结果);
    结果
}

fn 递归(目录: &Path, 排除项: &[String], 结果: &mut Vec<PathBuf>) {
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
            递归(&路径, 排除项, 结果);
        } else if 名.ends_with(".rs") {
            结果.push(路径);
        }
    }
}

/// ⑤ 道韵违逆检测（§十三 道韵维度启用）：
/// 调用 shihai_fu::扫描违逆 得到违逆报告，把每条违逆转为：
/// - 巡世候选（让鸿钧化为要求 → 大罗金仙自纠）
/// - 法则违逆（保留在巡世报告.违逆 字段供巡世报告库归档）
///
/// 严重度 → 优先级映射：
/// - 错误 → 高（边界违逆，深链是契约破坏，必须立即修）
/// - 警告 → 中（命名/引用止步违逆，约定级别，可批处理）
///
/// 类别映射：
/// - 边界违逆 → 补基础（跨府契约修复）
/// - 命名/引用止步 → 维护（规范梳理）
fn 检道韵违逆(根目录: &Path) -> (Vec<巡世候选>, Vec<法则违逆>) {
    // 用传入的根目录构造工作区（保证测试隔离 + 与扫描世界一致）
    let 工作区 = 工作区::新(根目录);
    let 报告 = 扫描违逆(&工作区);
    if 报告.条目们.is_empty() && 报告.警告数 == 0 && 报告.错误数 == 0 {
        return (Vec::new(), Vec::new());
    }

    let mut 候选们 = Vec::new();
    let mut 法则们 = Vec::new();

    for 条目 in &报告.条目们 {
        // 转法则违逆（写报告库）
        法则们.push(法则违逆 {
            路径: 条目.路径.clone(),
            违逆内容: 条目.描述.clone(),
            依据规则: format!(
                "道韵维度 §十二 · {:?} · 严重度 {:?}",
                条目.类型, 条目.严重度
            ),
        });

        // 转巡世候选（让世界自纠）
        let 优先级 = match 条目.严重度 {
            shihai_fu::严重度::错误 => 优先级::高,
            shihai_fu::严重度::警告 => 优先级::中,
        };
        let 建议类别 = match 条目.类型 {
            shihai_fu::违逆类型::边界 => 要求类别::补基础,
            shihai_fu::违逆类型::命名 | shihai_fu::违逆类型::引用止步 => {
                要求类别::维护
            }
            shihai_fu::违逆类型::层级 => 要求类别::补基础,
        };
        // 本质档位：违逆错误（S5 契约破坏）> 违逆警告（S11 规则违逆）
        let (本质类别, 本质档位) = match 条目.严重度 {
            shihai_fu::严重度::错误 => (本质类别::契约破坏, 本质档位::S5),
            shihai_fu::严重度::警告 => (本质类别::规则违逆, 本质档位::S11),
        };
        候选们.push(巡世候选 {
            目标: format!("修复道韵违逆：{}", 条目.描述),
            依据: format!("路径：{} · 建议：{}", 条目.路径, 条目.建议),
            建议类别,
            优先级,
            本质类别,
            本质档位,
        });
    }

    info!(
        候选数 = 候选们.len(),
        法则违逆数 = 法则们.len(),
        警告 = 报告.警告数,
        错误 = 报告.错误数,
        "道韵违逆检测完成"
    );
    (候选们, 法则们)
}

#[cfg(test)]
mod 真实任务验证 {
    //! 验证 §十三 道韵接入：扫描真实工作目录，输出候选与法则违逆。
    use super::*;
    use std::path::Path;

    /// ⑥ 占位·骨架扫描单测：临时工作区造占位文件 → 扫描 → 抓到高优先级占位候选，
    /// 干净文件不误报。不污染项目根，测试完清理临时目录。
    #[test]
    fn 占位骨架扫描_命中占位_不误报干净文件() {
        let 进程id = std::process::id();
        let 纳秒 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let 根 = std::env::temp_dir().join(format!("占位扫描-{进程id}-{纳秒}"));
        std::fs::create_dir_all(&根).unwrap();

        // 造一个含占位标记的文件 + 一个干净文件
        std::fs::write(根.join("有占位.rs"), "pub fn 服务占位() {}").unwrap();
        std::fs::write(根.join("干净.rs"), "pub fn 正常() {}").unwrap();

        // 造 2 个"不应误报"的文件：状态机枚举变体 + 提示词占位符（都是合法业务语义）
        std::fs::write(
            根.join("状态机.rs"),
            "要求状态::待实现 => vec![要求状态::实现中]",
        )
        .unwrap();
        std::fs::write(根.join("提示模板.rs"), "渲染占位符 {背景} {目标} 替换").unwrap();

        let 文件们 = 收集源文件(&根);
        let 候选 = 检占位骨架(&文件们);

        eprintln!("[占位扫描] 候选 {}", 候选.len());
        // 有占位.rs 应命中一条；干净.rs/状态机.rs/提示模板.rs 均不误报
        assert_eq!(
            候选.len(),
            1,
            "应只命中一条占位候选（不误报 状态机待实现/提示词占位符）"
        );
        assert!(候选[0].目标.contains("占位"), "候选目标应含占位标记词");
        assert!(候选[0].依据.contains("有占位.rs"), "依据应指向有占位.rs");

        let _ = std::fs::remove_dir_all(&根);
        eprintln!("[占位扫描] 通过（已清理临时工作区）");
    }

    /// 真实任务验证：扫描实际项目根，统计道韵违逆产出。
    /// 输出格式：候选数/违逆数 + 前 N 条候选与违逆详情，便于人工核对。
    #[test]
    fn 真实任务_扫描当前项目_产出道韵候选与法则违逆() {
        // 直接传项目根，绕过 cwd 锚点问题
        let 报告 = 扫描世界(Path::new(r"D:\洪荒 - 世界"));
        eprintln!("\n===== 真实任务：扫描当前项目 =====");
        eprintln!("候选数: {}", 报告.候选.len());
        eprintln!("法则违逆数: {}", 报告.违逆.len());
        eprintln!("\n--- 候选 (前 20 条) ---");
        for c in 报告.候选.iter().take(20) {
            eprintln!("[{:?}] {} -- {}", c.优先级, c.目标, c.依据);
        }
        eprintln!("\n--- 法则违逆 (前 20 条) ---");
        for v in 报告.违逆.iter().take(20) {
            eprintln!("{}: {} ({})", v.路径, v.违逆内容, v.依据规则);
        }
        eprintln!("===== END =====\n");
        // 真实任务允许 0 候选 0 违逆（项目干净），不强制断言
        assert!(报告.候选.len() < 10000, "候选数应合理");
        assert!(报告.违逆.len() < 10000, "法则违逆数应合理");
    }

    /// §十三 集成验证：临时工作区造违逆 → 扫描 → 检测到候选 + 法则违逆。
    /// 不污染项目根，测试完清理临时目录。
    #[test]
    fn 集成_临时工作区_造违逆扫描可捕() {
        let 进程id = std::process::id();
        let 纳秒 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let 根 = std::env::temp_dir().join(format!("xunshi-集成-{进程id}-{纳秒}"));
        std::fs::create_dir_all(&根).unwrap();

        // 故意造违逆：英文目录 + 园内 .rs
        std::fs::create_dir_all(根.join("bad_name_dir")).unwrap();
        std::fs::create_dir_all(根.join("示例-园")).unwrap();
        std::fs::write(根.join("示例-园").join("mod.rs"), "pub fn x() {}").unwrap();

        let 报告 = 扫描世界(&根);
        eprintln!("\n[集成] 临时工作区 {:?} 扫描结果:", 根);
        eprintln!("[集成]   候选数: {}", 报告.候选.len());
        eprintln!("[集成]   法则违逆数: {}", 报告.违逆.len());

        // 验证：bad_name_dir 应被检测为命名违逆 → 候选（警告级=中优先级）
        let 有命名候选 = 报告.候选.iter().any(|c| c.依据.contains("bad_name_dir"));
        let 有命名违逆 = 报告.违逆.iter().any(|v| v.路径.contains("bad_name_dir"));
        assert!(有命名候选, "应检测到 bad_name_dir 命名违逆候选");
        assert!(有命名违逆, "应包含 bad_name_dir 法则违逆");

        // 清理
        let _ = std::fs::remove_dir_all(&根);
        eprintln!("[集成] 测试通过（已清理临时工作区）");
    }

    /// §十三 e2e：扫描→候选→世界状态候选池 全流程（不依赖 AI token）。
    /// §十三 真实任务：扫描项目根并把候选/违写入世界状态（写入 .上下文/状态/世界状态.jsonl）。
    #[test]
    fn 真实任务_扫描项目根并写入世界状态() {
        use shihai_fu::工作区;
        let ws = 工作区::定位();
        let 报告 = 扫描世界(ws.根路径());
        eprintln!(
            "[真实任务] 扫描产出候选 {} / 违逆 {}",
            报告.候选.len(),
            报告.违逆.len()
        );

        let 路径 = ws.上下文目录().join("状态").join("世界状态.jsonl");
        std::fs::create_dir_all(路径.parent().unwrap()).unwrap();
        let mut 状态 = crate::确保世界状态初始化(&ws.上下文目录().join("状态")).unwrap_or_else(|_| {
            serde_json::from_str(r#"{"阶段":"甲","v1已存档":false,"进入路径":"从零创建","长期记忆":"","界主想法池":[],"在途要求":[],"验收历史":[],"失败模式":[],"版本历史":[],"巡世候选池":[],"项目档案":null,"天道报告库":[]}"#).unwrap()
        });

        let 入池前 = 状态.巡世候选池.len();
        for c in &报告.候选 {
            if !状态.巡世候选池.iter().any(|x| x.目标 == c.目标) {
                状态.巡世候选池.push(c.clone());
            }
        }
        let ts = shihai_fu::当前毫秒();
        状态.天道报告库.push(crate::巡世报告 {
            id: format!("巡世-{ts}"),
            时间: ts,
            候选: vec![],
            违逆: 报告.违逆.clone(),
        });

        let mut 内容 = std::fs::read_to_string(&路径).unwrap_or_default();
        let 新行 = serde_json::to_string(&状态).unwrap();
        内容.push_str(&format!("{}\n", 新行));
        std::fs::write(&路径, 内容).unwrap();

        eprintln!(
            "[真实任务] 候选池: {} → {}, 法则违逆: {}",
            入池前,
            状态.巡世候选池.len(),
            报告.违逆.len()
        );
    }

    #[test]
    fn e2e_扫描入候选池_世界状态更新() {
        let 进程id = std::process::id();
        let 纳秒 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let 临时 = std::env::temp_dir().join(format!("e2e-扫描入池-{进程id}-{纳秒}"));

        // 1. 造违逆
        std::fs::create_dir_all(&临时).unwrap();
        std::fs::create_dir_all(临时.join("bad_english_dir")).unwrap();
        std::fs::create_dir_all(临时.join("示例-园")).unwrap();
        std::fs::write(临时.join("示例-园").join("mod.rs"), "pub fn x() {}").unwrap();

        // 2. 扫描
        let 报告 = 扫描世界(&临时);
        eprintln!(
            "[e2e] 扫描完成：候选 {} 条 / 法则违逆 {} 条",
            报告.候选.len(),
            报告.违逆.len()
        );
        assert!(!报告.候选.is_empty() && 报告.候选.len() > 1);
        assert!(!报告.违逆.is_empty());

        // 3. 直接构造世界状态（避免依赖 确保世界状态初始化）
        let mut 状态 = match crate::确保世界状态初始化(&临时.join(".上下文")) {
            Ok(s) => s,
            Err(_) => serde_json::from_str(r#"{"阶段":"甲","v1已存档":false,"进入路径":"从零创建","长期记忆":"","界主想法池":[],"在途要求":[],"验收历史":[],"失败模式":[],"版本历史":[],"巡世候选池":[],"项目档案":null,"天道报告库":[]}"#).unwrap(),
        };
        let 入池前 = 状态.巡世候选池.len();
        for 候选 in &报告.候选 {
            if !状态.巡世候选池.iter().any(|c| c.目标 == 候选.目标) {
                状态.巡世候选池.push(候选.clone());
            }
        }
        状态.天道报告库.push(crate::巡世报告 {
            id: format!("巡世-{纳秒}"),
            时间: 纳秒 as u64,
            候选: vec![],
            违逆: 报告.违逆.clone(),
        });

        // 4. 验证候选池增长
        assert!(状态.巡世候选池.len() > 入池前, "候选池应增长");
        assert!(!状态.天道报告库.last().unwrap().违逆.is_empty());

        eprintln!("[e2e] 候选池：{} → {}", 入池前, 状态.巡世候选池.len());
        eprintln!(
            "[e2e] 法则违逆数：{}",
            状态.天道报告库.last().unwrap().违逆.len()
        );
        eprintln!("[e2e] 通过（不依赖 AI token，端到端验证 数据流）");

        // 清理
        let _ = std::fs::remove_dir_all(&临时);
    }
}
