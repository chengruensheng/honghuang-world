use std::path::Path;

use crate::{依赖边, 符号, 符号种类, 模块, 图谱};
use hm_error::{Error, Result};

/// 应跳过的目录：构建产物、版本控制、依赖缓存、IDE
const 忽略目录: &[&str] = &[
    "target", ".git", ".hg", ".svn", "node_modules", "dist", "build", ".next",
    "__pycache__", ".venv", "venv", ".idea", ".vscode",
];

/// crate 清单文件名（扫描器唯一需要的硬编码路径，收敛为常量便于门禁与维护）
const 清单文件: &str = "Cargo.toml";

/// Rust workspace 扫描器：启动装配期全量扫描项目根，构建「世界态」图谱。
///
/// 对齐 Python 原型 `F:\临时工作区\项目认知底座\rust_扫描器.py` 的通用化策略：
/// - 不硬编码目录命名约定（「-域/府/殿」、src/、crates/ 均可）
/// - 优先读顶层 Cargo.toml `[workspace] members`；回退 rglob 找子 Cargo.toml；再回退单 crate 模式
/// - 只提取**顶层 pub 符号**（fn/struct/enum/trait/const/static），impl 方法与私有项不提取
/// - 图谱 模块集 只收 crate 级（包名），避免目录/文件节点膨胀污染检索
pub struct Rust扫描器;

impl Rust扫描器 {
    pub fn 新() -> Self {
        Rust扫描器
    }

    /// 扫描项目根，构建图谱；根不存在/非目录返回 Err
    pub fn 扫描(&self, 根路径: &Path) -> Result<图谱> {
        if !根路径.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("扫描根路径不存在: {}", 根路径.display()),
            )));
        }
        if !根路径.is_dir() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("扫描根路径不是目录: {}", 根路径.display()),
            )));
        }

        let mut 图谱 = 图谱::新();

        // 1. 优先：顶层 Cargo.toml 的 [workspace] members
        let mut members = 读workspace成员(根路径);
        if members.is_empty() {
            // 2. 回退：rglob 找子 Cargo.toml（排除构建/版本控制目录）
            members = rglob成员(根路径);
        }
        if members.is_empty() {
            // 3. 单 crate 模式：根即 package
            members = vec![根路径.to_path_buf()];
        }

        // 两阶段：先全量建 crate 模块（依赖判定需要完整模块集），再解析依赖/技术栈/符号
        for member in &members {
            self.建crate模块(&mut 图谱, 根路径, member);
        }
        for member in &members {
            self.建crate依赖与符号(&mut 图谱, member);
        }
        Ok(图谱)
    }

    /// 阶段一：crate → 模块节点（去重：workspace 别名/重复成员不重复建）
    fn 建crate模块(&self, 图谱: &mut 图谱, 根路径: &Path, member: &Path) {
        let 相对路径 = match member.strip_prefix(根路径) {
            Ok(相对) => 相对.to_string_lossy().replace('\\', "/"),
            Err(_) => member.to_string_lossy().replace('\\', "/"),
        };
        let 包名 = 读包名(member)
            .filter(|名| !名.is_empty())
            .unwrap_or_else(|| {
                member
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
        if 包名.is_empty() {
            return;
        }
        if !图谱.模块集.iter().any(|模块| 模块.名称 == 包名) {
            图谱.添加模块(模块 { 名称: 包名.clone(), 路径: 相对路径.clone() });
        }
    }

    /// 阶段二：依赖边（workspace 内）/ 技术栈（外部）+ 顶层 pub 符号提取
    fn 建crate依赖与符号(&self, 图谱: &mut 图谱, member: &Path) {
        let 包名 = 读包名(member)
            .filter(|名| !名.is_empty())
            .unwrap_or_else(|| {
                member
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
        if 包名.is_empty() {
            return;
        }
        let toml路径 = member.join(清单文件);
        if let Ok(文本) = std::fs::read_to_string(&toml路径) {
            for 依赖名 in 读依赖(&文本) {
                if 图谱.模块集.iter().any(|模块| 模块.名称 == 依赖名) {
                    图谱.添加依赖(依赖边 { 源: 包名.clone(), 目标: 依赖名 });
                } else {
                    图谱.添加技术栈(依赖名);
                }
            }
        }

        // 递归收集 .rs 叶子文件，提取顶层 pub 符号
        let mut 文件集: Vec<std::path::PathBuf> = Vec::new();
        收集rs文件(member, &mut 文件集);
        for 文件 in 文件集 {
            提取符号(图谱, &包名, &文件);
        }
    }
}

/// 读顶层 Cargo.toml 的 [workspace] members 列表
fn 读workspace成员(根路径: &Path) -> Vec<std::path::PathBuf> {
    let toml路径 = 根路径.join(清单文件);
    let Ok(文本) = std::fs::read_to_string(&toml路径) else {
        return vec![];
    };
    let mut 在workspace = false;
    let mut 在members = false;
    let mut 成员: Vec<String> = Vec::new();
    for 行 in 文本.lines() {
        let 行 = 行.split('#').next().unwrap_or("").trim(); // 整行为注释时视为空行
        if 行.is_empty() {
            continue;
        }
        if 行.starts_with('[') {
            在workspace = 行 == "[workspace]";
            在members = false;
            continue;
        }
        if !在workspace {
            continue;
        }
        if 行.starts_with("members") {
            if let Some(等号) = 行.find('=') {
                let 余 = 行[等号 + 1..].trim();
                if 余.starts_with('[') {
                    // 内联数组 ["a", "b"]
                    for 段 in 余.trim_matches(|c| c == '[' || c == ']').split(',') {
                        let 段 = 段.trim().trim_matches('"').trim_matches('\'').trim();
                        if !段.is_empty() {
                            成员.push(段.to_string());
                        }
                    }
                    在members = false;
                } else {
                    在members = true;
                }
            }
            continue;
        }
        if 在members {
            if 行.starts_with(']') {
                在members = false;
                continue;
            }
            let 段 = 行.trim_matches(',').trim().trim_matches('"').trim_matches('\'').trim();
            if !段.is_empty() {
                成员.push(段.to_string());
            }
        }
    }
    成员.into_iter().map(|m| 根路径.join(m)).collect()
}

/// 回退：rglob 找子 Cargo.toml（排除忽略目录与顶层）
fn rglob成员(根路径: &Path) -> Vec<std::path::PathBuf> {
    let mut 结果: Vec<std::path::PathBuf> = Vec::new();
    let mut 待扫: Vec<std::path::PathBuf> = vec![根路径.to_path_buf()];
    while let Some(目录) = 待扫.pop() {
        let Ok(条目) = std::fs::read_dir(&目录) else { continue };
        for 条目 in 条目.flatten() {
            let 路径 = 条目.path();
            if 路径.is_dir() {
                let 名 = 路径.file_name().and_then(|n| n.to_str()).unwrap_or(""); // 非 UTF-8 文件名降级为空串
                if 名.starts_with('.') || 忽略目录.contains(&名) {
                    continue;
                }
                if 路径.join(清单文件).exists() {
                    结果.push(路径);
                } else {
                    待扫.push(路径);
                }
            }
        }
    }
    结果.sort();
    结果
}

/// 读 Cargo.toml [package] name
fn 读包名(member: &Path) -> Option<String> {
    let toml路径 = member.join(清单文件);
    let 文本 = std::fs::read_to_string(&toml路径).ok()?;
    let mut 在包 = false;
    for 行 in 文本.lines() {
        let 行 = 行.split('#').next().unwrap_or("").trim(); // 整行为注释时视为空行
        if 行.starts_with('[') {
            在包 = 行 == "[package]";
            continue;
        }
        if 在包 && 行.starts_with("name") {
            if let Some(等号) = 行.find('=') {
                let 值 = 行[等号 + 1..].trim().trim_matches('"').trim_matches('\'').trim();
                if !值.is_empty() {
                    return Some(值.to_string());
                }
            }
        }
    }
    None
}

/// 极简 Cargo.toml 依赖解析：只取包名（dependencies / dev-dependencies / build-dependencies 段）
fn 读依赖(文本: &str) -> Vec<String> {
    let mut 结果: Vec<String> = Vec::new();
    let mut 在依赖 = false;
    let mut 段依赖名: Option<String> = None; // [dependencies.tokio] 段的 "tokio"
    for 行 in 文本.lines() {
        let 行 = 行.split('#').next().unwrap_or("").trim(); // 整行为注释时视为空行
        if 行.is_empty() {
            continue;
        }
        if 行.starts_with('[') {
            let 段 = 行.trim_matches('[').trim_matches(']').trim();
            在依赖 = 段 == "dependencies"
                || 段 == "dev-dependencies"
                || 段 == "build-dependencies"
                || 段.starts_with("dependencies.");
            段依赖名 = 段.strip_prefix("dependencies.").map(|s| s.to_string());
            continue;
        }
        if !在依赖 {
            continue;
        }
        if let Some(等号) = 行.find('=') {
            let 行名 = 行[..等号].trim().trim_matches('"').trim_matches('\'').trim();
            // [dependencies.tokio] 段内是 version/path 等键，依赖名取段后缀
            let 名 = 段依赖名.clone().unwrap_or_else(|| 行名.to_string());
            if 名.is_empty() || 名.starts_with('[') {
                continue;
            }
            if 段依赖名.is_none()
                && matches!(行名, "version" | "path" | "git" | "features" | "default-features" | "optional")
            {
                continue; // 普通段内非依赖键
            }
            if !结果.iter().any(|已有| 已有 == &名) {
                结果.push(名);
            }
        }
    }
    结果
}

/// 递归收集目录下全部 .rs 文件（排除忽略目录/隐藏目录）
fn 收集rs文件(目录: &Path, 输出: &mut Vec<std::path::PathBuf>) {
    let Ok(条目) = std::fs::read_dir(目录) else { return };
    for 条目 in 条目.flatten() {
        let 路径 = 条目.path();
        if 路径.is_dir() {
            let 名 = 路径.file_name().and_then(|n| n.to_str()).unwrap_or(""); // 非 UTF-8 文件名降级为空串
            if 名.starts_with('.') || 忽略目录.contains(&名) {
                continue;
            }
            收集rs文件(&路径, 输出);
        } else if 路径.extension().and_then(|e| e.to_str()) == Some("rs") {
            输出.push(路径);
        }
    }
}

/// 提取文件内顶层 pub 符号（行级状态机：跳过注释/字符串内误匹配）
fn 提取符号(图谱: &mut 图谱, 包名: &str, 文件: &Path) {
    let Ok(文本) = std::fs::read_to_string(文件) else { return };
    for 行 in 文本.lines() {
        let 去注释 = 去行注释(行).trim();
        // 顶层 pub 符号：pub 开头（含 pub(crate)/pub(super) 等可见性形式）
        if !去注释.starts_with("pub ") && !去注释.starts_with("pub(") {
            continue;
        }
        let 余 = if let Some(后) = 去注释.strip_prefix("pub ") {
            后.trim_start()
        } else if let Some(后) = 去注释.strip_prefix("pub(") {
            // pub(crate) fn ...：吃掉括号内可见性
            match 后.find(')') {
                Some(闭) => 后[闭 + 1..].trim_start(),
                None => 后,
            }
        } else {
            continue;
        };
        if 余.is_empty() {
            continue;
        }
        let Some((种类, 名称, 签名)) = 解析符号(余) else {
            continue;
        };
        图谱.添加符号(符号 {
            名称,
            种类,
            所属模块: 包名.to_string(),
            签名,
        });
    }
}

/// 解析 `[修饰] fn 名称...` / `struct 名称` / `const 名称` ...；返回 (种类, 名称, 签名)
fn 解析符号(余: &str) -> Option<(符号种类, String, Option<String>)> {
    let 词集: Vec<&str> = 余.split_whitespace().collect();
    let mut i = 0;
    let mut 是常量 = false;
    while i < 词集.len() {
        match 词集[i] {
            "async" | "unsafe" | "default" => i += 1,
            "extern" => i += 2, // extern "C" fn
            "const" | "static" if 词集.get(i + 1).copied() == Some("fn") => i += 1, // const fn
            "const" | "static" => {
                是常量 = true;
                i += 1;
            }
            _ => break,
        }
    }
    // 常量：const 名 = 值 / static 名: 类型（i 已停在名称词）
    if 是常量 {
        let 名称 = 词集
            .get(i)
            .map(|名| {
                名.trim_start_matches("r#")
                    .split(|c| c == '(' || c == '<' || c == ':' || c == ';' || c == '{')
                    .next()
                    .unwrap_or("") // 名称取词失败降级为空串，由下方 is_empty 守卫
                    .trim()
                    .to_string()
            })
            .unwrap_or_default();
        if 名称.is_empty() {
            return None;
        }
        return Some((符号种类::常量, 名称, Some(余.to_string())));
    }
    let Some(&种类词) = 词集.get(i) else { return None };
    // 名称截断到 `(`/`<`/`:`/`;`/`{`：`运行(任务: &str)` → `运行`；`阈值: usize` → `阈值`
    let 名称 = 词集
        .get(i + 1)
        .map(|名| {
            名.trim_start_matches("r#")
                .split(|c| c == '(' || c == '<' || c == ':' || c == ';' || c == '{')
                .next()
                .unwrap_or("") // 名称取词失败降级为空串，由下方 is_empty 守卫
                .trim()
                .to_string()
        })?;
    if 名称.is_empty() {
        return None;
    }
    let 种类 = match 种类词 {
        "fn" | "trait" => 符号种类::函数,
        "struct" | "enum" | "type" => 符号种类::类型,
        "const" | "static" => 符号种类::常量,
        _ => return None, // pub mod / pub use / pub(crate) use 等不提取
    };
    // 签名：函数取 名称 后到行尾（截断 120 字符，去尾分号），保留参数与返回
    let 签名 = if 种类词 == "fn" {
        let 头部 = 余.replace("pub ", "").replace("pub(crate) ", "");
        let 截断: String = 头部.chars().take(120).collect();
        Some(截断.trim_end_matches(';').trim().to_string())
    } else {
        None
    };
    Some((种类, 名称, 签名))
}

/// 去行注释：不在字符串字面量内的 `//` 之后全部去掉（块注释内部不处理，避免过度复杂）
fn 去行注释(行: &str) -> &str {
    let 字节 = 行.as_bytes();
    let mut 在字符串 = false;
    let mut 转义 = false;
    for (i, &b) in 字节.iter().enumerate() {
        if 在字符串 {
            if 转义 {
                转义 = false;
            } else if b == b'\\' {
                转义 = true;
            } else if b == b'"' {
                在字符串 = false;
            }
            continue;
        }
        match b {
            b'"' => 在字符串 = true,
            b'/' if i + 1 < 字节.len() && 字节[i + 1] == b'/' => return &行[..i],
            _ => {}
        }
    }
    行
}
