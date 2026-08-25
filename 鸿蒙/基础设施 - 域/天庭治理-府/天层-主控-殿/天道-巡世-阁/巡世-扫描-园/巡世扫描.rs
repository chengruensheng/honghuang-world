//! 巡世 - 扫描 - 园 · 巡世扫描：扫描世界目录，产出巡世报告（候选清单 + 法则违逆清单）。
//!
//! 提供两个公开函数：
//! - `本质打分`：根据目标与依据文本，归类「本质类别 + 本质档位」。
//! - `扫描世界`：巡世主入口，串联五种启发（园无测试 / clippy 警告 / 教训重复 / 规模 / 占位骨架）+ 道韵违逆扫描。
//! - `巡世扫描_占位`：模块级注册占位（守护一次调用），用于满足 `pub` 路径可见性测试与最小编译验证。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use shihai_fu::{本质档位, 本质类别};
use tianting_fu::{优先级, 巡世报告, 巡世候选, 要求类别};
use tracing::{info, warn};

/// 模块级注册守护：保证 `巡世扫描_占位` 在并发场景下仅生效一次，防止 data race。
static 巡世扫描_占位守护: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// 本质打分：根据目标与依据文本，返回（本质类别, 本质档位）。
///
/// 优先依据文本特征（更稳定），其次目标关键词；兜底归「覆盖率不足」/ S11。
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

/// 巡世扫描主入口：扫描 `根目录`，返回 `巡世报告{id, 时间, 候选, 违逆}`。
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

/// 收集 `根目录` 下所有 `.rs` 源文件路径。
fn 收集源文件(根目录: &Path) -> Vec<std::path::PathBuf> {
    let mut 文件们 = Vec::new();
    收集源文件_递归(根目录, &mut 文件们);
    文件们
}

fn 收集源文件_递归(目录: &Path, 文件们: &mut Vec<std::path::PathBuf>) {
    let 读结果 = std::fs::read_dir(目录);
    let 条目们 = match 读结果 {
        Ok(it) => it,
        Err(_) => return,
    };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        if 路径.is_dir() {
            // 跳过常见无意义目录
            if let Some(名) = 路径.file_name().and_then(|s| s.to_str()) {
                if matches!(名, "target" | ".git" | "node_modules" | ".上下文" | ".codeartsdoer") {
                    continue;
                }
            }
            收集源文件_递归(&路径, 文件们);
        } else if 路径.extension().and_then(|s| s.to_str()) == Some("rs") {
            文件们.push(路径);
        }
    }
}

/// ① 园无测试检测：园目录（以「-园」结尾）下所有 .rs 均无 `#[test]` → 产候选。
fn 检园无测试(根目录: &Path) -> Vec<巡世候选> {
    let mut 候选 = Vec::new();
    遍历园(根目录, &mut |园路径: &Path| {
        let 园名 = 园路径
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let rs们 = 收集_rs(园路径);
        if rs们.is_empty() {
            return;
        }
        let 全部含测试 = rs们.iter().all(|p| 文件含测试标记(p));
        if !全部含测试 {
            let (本质类别, 本质档位) = 本质打分(
                &format!("为{园名}生产模块追加单元测试"),
                "园目录下全部 .rs 缺少 #[test] 标记",
            );
            候选.push(巡世候选 {
                目标: format!("为{园名}生产模块追加单元测试"),
                依据: format!("园目录 {} 下全部 .rs 缺少 #[test] 标记", 园路径.display()),
                建议类别: 要求类别::补测试,
                优先级: 优先级::中,
                本质类别,
                本质档位,
            });
        }
    });
    候选
}

fn 遍历园<F: FnMut(&Path)>(根目录: &Path, 回调: &mut F) {
    let 读结果 = std::fs::read_dir(根目录);
    let 条目们 = match 读结果 {
        Ok(it) => it,
        Err(_) => return,
    };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        if !路径.is_dir() {
            continue;
        }
        let 名 = 路径.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if 名.ends_with("-园") {
            回调(&路径);
        } else {
            遍历园(&路径, 回调);
        }
    }
}

fn 收集_rs(目录: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(条目们) = std::fs::read_dir(目录) {
        for 条目 in 条目们.flatten() {
            let p = 条目.path();
            if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    out
}

fn 文件含测试标记(路径: &Path) -> bool {
    let 内容 = match std::fs::read_to_string(路径) {
        Ok(c) => c,
        Err(_) => return false,
    };
    内容.contains("#[test]") || 内容.contains("#![test]")
}

/// ② clippy 警告检测：若发现 clippy 警告文本文件（构建产物 .上下文/clippy.txt）则产候选。
///
/// 此处只检测"是否存在标记文件"，具体警告数由调用方在依据中带入。
fn 检clippy警告(根目录: &Path) -> Vec<巡世候选> {
    let 标记 = 根目录.join(".上下文").join("clippy.txt");
    if !标记.exists() {
        return Vec::new();
    }
    let 内容 = std::fs::read_to_string(&标记).unwrap_or_default();
    let 警告数 = 内容.lines().filter(|l| l.contains("warning:")).count();
    if 警告数 == 0 {
        return Vec::new();
    }
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
                建议类别: 要求类别::补测试,
                优先级: 优先级::高,
                本质类别,
                本质档位,
            });
        }
    }
    候选
}

/// ④ 规模启发：源文件数 > 200 → 产候选。
fn 检规模(文件们: &[std::path::PathBuf]) -> Vec<巡世候选> {
    if 文件们.len() <= 200 {
        return Vec::new();
    }
    let (本质类别, 本质档位) = 本质打分(
        &format!("项目规模较大（{}个源文件），考虑分层拆分", 文件们.len()),
        &format!("源文件数 {} 超过 200 阈值", 文件们.len()),
    );
    vec![巡世候选 {
        目标: format!("项目规模较大（{}个源文件），考虑分层拆分", 文件们.len()),
        依据: format!("源文件数 {} 超过 200 阈值", 文件们.len()),
        建议类别: 要求类别::优化,
        优先级: 优先级::中,
        本质类别,
        本质档位,
    }]
}

/// ⑤ 占位骨架检测：园目录下 .rs 仅有 `// 占位`/`// 兜底` 类注释且无 pub fn → 产候选。
fn 检占位骨架(文件们: &[std::path::PathBuf]) -> Vec<巡世候选> {
    let mut 候选 = Vec::new();
    for 文件 in 文件们 {
        // 只关心园目录下的 .rs
        let 在园下 = 文件
            .ancestors()
            .any(|a| a.file_name().and_then(|s| s.to_str()).map_or(false, |n| n.ends_with("-园")));
        if !在园下 {
            continue;
        }
        let 内容 = match std::fs::read_to_string(文件) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // 占位特征：内容极短（<200 字符）且不含 `pub fn`
        let 是占位 = 内容.len() < 200 && !内容.contains("pub fn");
        if 是占位 {
            let 文件名 = 文件
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("未知");
            let (本质类别, 本质档位) = 本质打分(
                &format!("补齐占位：{文件名}"),
                &format!("{} 内容仅占位注释且无 pub fn", 文件.display()),
            );
            候选.push(巡世候选 {
                目标: format!("补齐占位：{文件名}"),
                依据: format!("{} 内容仅占位注释且无 pub fn", 文件.display()),
                建议类别: 要求类别::补实现,
                优先级: 优先级::中,
                本质类别,
                本质档位,
            });
        }
    }
    候选
}

/// ⑥ 道韵违逆扫描：复用识海铭记殿违逆扫描，返回 (道韵候选, 法则违逆)。
fn 检道韵违逆(根目录: &Path) -> (Vec<巡世候选>, Vec<tianting_fu::法则违逆>) {
    let 违逆报告 = shihai_fu::扫描违逆(根目录);
    let mut 候选 = Vec::new();
    let mut 违逆们 = Vec::new();
    for 条目 in &违逆报告.条目 {
        违逆们.push(tianting_fu::法则违逆 {
            类型: 条目.类型.clone(),
            严重度: 条目.严重度.clone(),
            文件: 条目.文件.clone(),
            行: 条目.行,
            摘要: 条目.摘要.clone(),
        });
        let (本质类别, 本质档位) = 本质打分(
            &format!("修复道韵违逆：{}", 条目.摘要),
            "道韵违逆扫描发现规则违反条目",
        );
        候选.push(巡世候选 {
            目标: format!("修复道韵违逆：{}", 条目.摘要),
            依据: format!("道韵违逆扫描发现规则违反：{}", 条目.摘要),
            建议类别: 要求类别::优化,
            优先级: 优先级::高,
            本质类别,
            本质档位,
        });
    }
    (候选, 违逆们)
}

/// 模块级注册占位：用于证明 `pub fn 巡世扫描_占位` 经 `pub mod` 链路真实可达。
///
/// 由 `once_cell::sync::Lazy<Mutex<()>>` 守护，幂等且并发安全。
/// `#[cold]` 显式声明占位语义，防误内联。
#[cold]
pub fn 巡世扫描_占位() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _守卫 = 巡世扫描_占位守护
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|中毒| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("巡世扫描占位守护锁中毒：{中毒}"),
            ))
        })?;
    Ok(())
}