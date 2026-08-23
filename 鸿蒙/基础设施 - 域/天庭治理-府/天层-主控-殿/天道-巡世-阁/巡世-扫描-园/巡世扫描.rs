//! 巡世 - 扫描 - 园：扫描世界，产出巡世报告与违逆清单。
//!
//! 四项检查统一入口（设计稿 §12 P2-7，Line 1114）：
//! ① 园无测试检测 ② clippy 警告检测 ③ 教训重复模式检测 ④ 规模启发。

use crate::类型_定义_殿::{优先级, 巡世候选, 巡世报告, 法则违逆, 要求类别};
use rizhi_fu::{info, warn};
use shihai_fu::{工作区, 扫描违逆};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

/// 园无测试检测跳过项：证道 / 单元测试-府 本就是测试集合，不要求园内再嵌测试。
const 园无测试跳过: &[&str] = &["证道", "单元测试-府"];

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
            候选.push(巡世候选 {
                目标: format!("为{园名}生产模块追加单元测试"),
                依据: format!("园路径 {}", 园路径.display()),
                建议类别: 要求类别::维护,
                优先级: 优先级::中,
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
        vec![巡世候选 {
            目标: format!("清理clippy警告（{警告数}条）"),
            依据: format!("cargo clippy --workspace --all-targets 输出 {警告数} 条 warning"),
            建议类别: 要求类别::优化,
            优先级: 优先级::中,
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
            候选.push(巡世候选 {
                目标: format!("{摘要}反复出现（{}次），需系统性修复", 同组.len()),
                依据: format!("教训格位同前缀（前 40 字符）记录 {} 条", 同组.len()),
                建议类别: 要求类别::维护,
                优先级: 优先级::高,
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
        vec![巡世候选 {
            目标: "项目规模较大，考虑按域拆分为更多府".to_string(),
            依据: format!("源文件数 {}", 文件们.len()),
            建议类别: 要求类别::优化,
            优先级: 优先级::低,
        }]
    } else {
        Vec::new()
    }
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
        候选们.push(巡世候选 {
            目标: format!("修复道韵违逆：{}", 条目.描述),
            依据: format!("路径：{} · 建议：{}", 条目.路径, 条目.建议),
            建议类别,
            优先级,
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
}
