//! workspace 成员缓存——读根 Cargo.toml 的 workspace members + 各府 [lib] name/[dependencies]，
//! 带文件指纹缓存，避免三档拼装与模板生成重复读盘解析。
//!
//! 落本园因三档拼装首用此能力，园内多文件允许（层级结构-设计 §8.6）；
//! 经 shihai_fu 入口透出后，天庭治理-府 模板生成亦调用，止步 lib 根。
//!
//! 缓存失效：以根 Cargo.toml 的 (mtime, size) 为指纹，根 Cargo.toml 变化即重算。
//! 各府 Cargo.toml 的依赖段变化不触发失效——开发期改了重新编译运行即重建；
//! 运行时 Cargo.toml 不变，缓存命中后零读盘零解析。

use crate::工作区;
use rizhi_fu::debug;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::SystemTime;

/// workspace 成员摘要——members 列表 + 各府依赖信息，供调用方各自格式化。
#[derive(Clone)]
pub struct 工作区成员摘要 {
    /// workspace members 中以 `-府` 结尾的成员相对路径列表。
    pub 成员们: Vec<String>,
    /// 各府的依赖信息（府名 / lib 名 / 依赖列表）。
    pub 府间依赖: Vec<府依赖>,
}

/// 单个府的依赖信息。
#[derive(Clone)]
pub struct 府依赖 {
    /// 府名（member 路径末段）。
    pub 府名: String,
    /// [lib] name，未声明时为 None。
    pub lib名: Option<String>,
    /// [dependencies] 段的依赖名列表。
    pub 依赖们: Vec<String>,
}

/// 缓存项——记录指纹与摘要，指纹匹配则复用摘要。
struct 缓存项 {
    根路径: PathBuf,
    指纹: (SystemTime, u64),
    摘要: 工作区成员摘要,
}

static 缓存: OnceLock<RwLock<Option<缓存项>>> = OnceLock::new();

/// 取缓存静态引用（OnceLock 延迟初始化，避开 RwLock::new 的 const fn 版本要求）。
fn 取缓存() -> &'static RwLock<Option<缓存项>> {
    缓存.get_or_init(|| RwLock::new(None))
}

/// 读 workspace 成员摘要（带缓存）——用当前工作区定位。
pub fn 读workspace成员缓存() -> Option<工作区成员摘要> {
    读workspace成员缓存在(&工作区::定位())
}

/// 读 workspace 成员摘要（带缓存）——在指定工作区读，供测试注入临时工作区。
///
/// 流程：读根 Cargo.toml 的 metadata 算指纹 → 命中缓存则返回 → 否则解析后存缓存返回。
/// members 为空返回 None（零开销不注入）。
pub fn 读workspace成员缓存在(工作区: &工作区) -> Option<工作区成员摘要> {
    let 根 = 工作区.根路径();
    let cargo路径 = 根.join("Cargo.toml");
    let metadata = std::fs::metadata(&cargo路径).ok()?;
    let mtime = metadata.modified().ok()?;
    let 指纹 = (mtime, metadata.len());
    // 先读锁查缓存：根路径相同且指纹匹配则复用。
    {
        let 锁 = 取缓存().read().unwrap_or_else(|毒| 毒.into_inner());
        if let Some(项) = 锁.as_ref() {
            if 项.根路径 == 根 && 项.指纹 == 指纹 {
                return Some(项.摘要.clone());
            }
        }
    }
    // 未命中：解析根 + 各府 Cargo.toml。
    let 摘要 = 解析工作区成员摘要(根)?;
    let 新项 = 缓存项 {
        根路径: 根.to_path_buf(),
        指纹,
        摘要: 摘要.clone(),
    };
    {
        let mut 锁 = 取缓存().write().unwrap_or_else(|毒| 毒.into_inner());
        *锁 = Some(新项);
    }
    Some(摘要)
}

/// 解析 workspace 成员摘要——读根 Cargo.toml 取 members，再读各府 Cargo.toml 取 lib 名与依赖。
fn 解析工作区成员摘要(根: &Path) -> Option<工作区成员摘要> {
    let 内容 = std::fs::read_to_string(根.join("Cargo.toml")).ok()?;
    let 成员们: Vec<String> = 内容
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
    if 成员们.is_empty() {
        return None;
    }
    let mut 府间依赖 = Vec::with_capacity(成员们.len());
    for member in &成员们 {
        let 府cargo = 根.join(member).join("Cargo.toml");
        let Ok(府内容) = std::fs::read_to_string(&府cargo) else {
            continue;
        };
        let 府名 = Path::new(member)
            .file_name()
            .map(|名| 名.to_string_lossy().to_string())
            .unwrap_or_else(|| member.clone());
        let lib名 = 解析lib名(&府内容);
        let 依赖们 = 解析依赖段(&府内容);
        府间依赖.push(府依赖 {
            府名,
            lib名,
            依赖们,
        });
    }
    debug!(府数 = 府间依赖.len(), "workspace 成员摘要已解析");
    Some(工作区成员摘要 {
        成员们, 府间依赖
    })
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
