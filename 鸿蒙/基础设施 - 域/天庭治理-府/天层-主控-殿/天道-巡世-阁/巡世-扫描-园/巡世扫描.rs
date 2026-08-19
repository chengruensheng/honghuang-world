//! 巡世 - 扫描 - 园：扫描世界，产出巡世报告与违逆清单。
//!
//! 四项检查统一入口（设计稿 §12 P2-7，Line 1114）：
//! ① 园无测试检测 ② clippy 警告检测 ③ 教训重复模式检测 ④ 规模启发。

use crate::类型_定义_殿::{优先级, 巡世候选, 巡世报告, 要求类别};
use rizhi_fu::{info, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 园无测试检测跳过项：证道 / 单元测试-府 本就是测试集合，不要求园内再嵌测试。
const 园无测试跳过: &[&str] = &["证道", "单元测试-府"];

/// 扫描世界：四项检查统一入口，汇聚候选与违逆。
pub fn 扫描世界(根目录: &Path) -> 巡世报告 {
    let 文件们 = 收集源文件(根目录);
    let mut 候选 = Vec::new();
    候选.extend(检园无测试(根目录));
    候选.extend(检clippy警告(根目录));
    候选.extend(检教训重复(根目录));
    候选.extend(检规模(&文件们));
    let 时间 = shihai_fu::当前毫秒();
    info!(
        根 = %根目录.display(),
        源文件数 = 文件们.len(),
        候选数 = 候选.len(),
        "巡世扫描完成"
    );
    巡世报告 {
        id: format!("巡世-{时间}"),
        时间,
        候选,
        违逆: Vec::new(),
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
fn 含测试标记(rs: &Path) -> bool {
    let Ok(内容) = std::fs::read_to_string(rs) else {
        return false;
    };
    内容.contains("#[test]") || 内容.contains("#[cfg(test)]") || 内容.contains("mod 测试")
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
fn 检clippy警告(根目录: &Path) -> Vec<巡世候选> {
    if !根目录.join("Cargo.toml").exists() {
        return Vec::new();
    }
    let 输出 = std::process::Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--message-format=short",
        ])
        .current_dir(根目录)
        .output();
    let Ok(输出) = 输出 else {
        warn!("clippy 调用失败，跳过 clippy 警告检测");
        return Vec::new();
    };
    let 合并 = format!(
        "{}\n{}",
        String::from_utf8_lossy(&输出.stdout),
        String::from_utf8_lossy(&输出.stderr)
    );
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
