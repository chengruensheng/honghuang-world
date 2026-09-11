//! hm-symext-rust —— Rust 语言解析插件入口
//!
//! 由引擎按 `插件清单.json` 的 `入口` 启动，调用约定见设计 §4.2：
//! `<入口> --项目根 <路径> --清单 <文件列表> --输出 <json路径>`
//!
//! 本 crate 与引擎 crate 分离：契约类型（产出 / 符号 / 边…）来自 `hm-symext`，
//! 而 tree-sitter 与 rust-analyzer 等重依赖只挂在本 crate，引擎不沾。

use hm_symext::{产出, 边, 当前协议版本, 插件诊断, 符号};
use hm_symext_rust::{生成符号与边, 解析文件, 扫描单元包, 扫描调用, 单元包信息};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match 运行() {
        Ok(()) => ExitCode::SUCCESS,
        Err(原因) => {
            eprintln!("rust 插件失败：{原因}");
            ExitCode::from(1)
        }
    }
}

fn 运行() -> Result<(), String> {
    let 参数: Vec<String> = std::env::args().collect();
    let mut 项目根 = PathBuf::from(".");
    let mut 清单路径: Option<PathBuf> = None;
    let mut 输出路径: Option<PathBuf> = None;

    let mut i = 1;
    while i < 参数.len() {
        match 参数[i].as_str() {
            "--项目根" if i + 1 < 参数.len() => {
                项目根 = PathBuf::from(&参数[i + 1]);
                i += 2;
            }
            "--清单" if i + 1 < 参数.len() => {
                清单路径 = Some(PathBuf::from(&参数[i + 1]));
                i += 2;
            }
            "--输出" if i + 1 < 参数.len() => {
                输出路径 = Some(PathBuf::from(&参数[i + 1]));
                i += 2;
            }
            _ => i += 1,
        }
    }

    let 清单路径 = 清单路径.ok_or_else(|| "缺少 --清单 参数".to_string())?;
    let 输出路径 = 输出路径.ok_or_else(|| "缺少 --输出 参数".to_string())?;

    let 列表文本 = std::fs::read_to_string(&清单路径).map_err(|e| format!("读文件清单失败：{e}"))?;
    let 文件们: Vec<String> = 列表文本
        .lines()
        .map(|行| 行.trim().to_string())
        .filter(|行| !行.is_empty())
        .collect();

    // 先统一读一遍源文件：指纹与语法层共用这一份内容，避免「算完指纹后文件又变」
    // 导致产出与指纹错配。
    let mut 源们: Vec<(String, Vec<u8>)> = Vec::new();
    let mut 诊断: Vec<插件诊断> = Vec::new();
    for 相对 in &文件们 {
        match std::fs::read(项目根.join(相对)) {
            Ok(内容) => 源们.push((相对.clone(), 内容)),
            Err(错误) => 诊断.push(插件诊断 {
                级别: "警告".into(),
                文件: 相对.clone(),
                信息: format!("读取失败：{错误}"),
            }),
        }
    }

    let 单元包结果 = 扫描单元包(&项目根);
    let 指纹 = 计算指纹(&项目根, &文件们, &源们, 单元包结果.as_deref().unwrap_or(&[]));

    // 整层缓存：指纹一致即插件的全部输入未变，直接复用上次产出、跳过本次全部扫描。
    // 语义层（rust-analyzer）既不能分块也无法序列化数据库，只能整层命中或整层重扫，
    // 故这是把载入成本降到零的唯一手段。
    if let Some(缓存产出) = 命中缓存(&项目根, 指纹) {
        std::fs::copy(&缓存产出, &输出路径).map_err(|e| format!("写产出失败：{e}"))?;
        eprintln!("rust 插件：输入未变，复用缓存产出（指纹 {指纹:016x}）");
        return Ok(());
    }

    let mut 全部符号: Vec<符号> = Vec::new();
    let mut 全部边: Vec<边> = Vec::new();
    let mut 包名缓存: BTreeMap<PathBuf, String> = BTreeMap::new();

    // 单元包层：crate 符号 + 依赖边（读 Cargo 清单，无语法推断，置信度高）
    match 单元包结果 {
        Ok(单元包们) => {
            let (crate符号们, 依赖边们) = 生成符号与边(&单元包们);
            全部符号.extend(crate符号们);
            全部边.extend(依赖边们);
        }
        Err(原因) => 诊断.push(插件诊断 {
            级别: "警告".into(),
            文件: "Cargo.toml".into(),
            信息: 原因,
        }),
    }

    // 语法层：逐文件提取符号与边（复用上面已读入的内存内容）
    for (相对, 源码) in &源们 {
        let 包名 = 找包名(&项目根, 相对, &mut 包名缓存);
        match 解析文件(相对, &包名, 源码) {
            Ok((符们, 边们)) => {
                全部符号.extend(符们);
                全部边.extend(边们);
            }
            Err(原因) => {
                诊断.push(插件诊断 {
                    级别: "警告".into(),
                    文件: 相对.clone(),
                    信息: 原因,
                });
            }
        }
    }

    // 语义层：函数级调用边（载入 rust-analyzer，置信度高）。失败只降级不中断——
    // 语法层产出照常，调用边缺席由诊断说明。
    match 扫描调用(&项目根, &全部符号) {
        Ok((调用边们, 语义诊断)) => {
            全部边.extend(调用边们);
            诊断.extend(语义诊断);
        }
        Err(原因) => 诊断.push(插件诊断 {
            级别: "警告".into(),
            文件: "Cargo.toml".into(),
            信息: 原因,
        }),
    }

    let 包 = 产出 {
        协议版本: 当前协议版本.to_string(),
        语言: "rust".into(),
        符号: 全部符号,
        边: 全部边,
        悬空: Vec::new(),
        诊断,
    };

    let json = serde_json::to_string_pretty(&包).map_err(|e| format!("序列化产出失败：{e}"))?;
    std::fs::write(&输出路径, json).map_err(|e| format!("写产出失败：{e}"))?;
    写回缓存(&项目根, 指纹, &输出路径);
    Ok(())
}

// ---------- 整层缓存 ----------

/// 缓存目录：随项目根走的状态区（`.传承结构规范` §二「记忆/【状态】」）
fn 缓存目录(项目根: &Path) -> PathBuf {
    项目根.join(".传承").join("记忆").join("缓存")
}

/// 整层指纹：插件版本 + 协议版本 + 文件清单 + 全部 Cargo 清单 + 全部源文件内容
///
/// 必须覆盖插件的**全部输入**：本插件除 `.rs` 内容外还读 Cargo 清单（crate 符号与
/// `依赖` 边都从那里来），且以「清单里有哪些文件」为界（读不到的文件同样计入）。
/// 漏项即会在「只改该项」时错误复用旧产出。版本号一并入指纹，则插件逻辑升级后
/// 缓存自动失效——否则产出口径变了仍复用旧数据。
fn 计算指纹(
    项目根: &Path,
    清单文件们: &[String],
    源们: &[(String, Vec<u8>)],
    单元包们: &[单元包信息],
) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut 哈希 = std::collections::hash_map::DefaultHasher::new();
    env!("CARGO_PKG_VERSION").hash(&mut 哈希);
    当前协议版本.hash(&mut 哈希);
    清单文件们.hash(&mut 哈希);

    let mut 清单们: Vec<(String, Vec<u8>)> = Vec::new();
    if let Ok(内容) = std::fs::read(项目根.join("Cargo.toml")) {
        清单们.push(("Cargo.toml".to_string(), 内容));
    }
    for 包 in 单元包们 {
        if let Ok(内容) = std::fs::read(项目根.join(&包.清单路径)) {
            清单们.push((包.清单路径.clone(), 内容));
        }
    }
    清单们.sort();

    for (路径, 内容) in 清单们.iter().chain(源们.iter()) {
        路径.hash(&mut 哈希);
        内容.hash(&mut 哈希);
    }
    哈希.finish()
}

/// 指纹一致且缓存产出在位时，返回该产出的路径
fn 命中缓存(项目根: &Path, 指纹: u64) -> Option<PathBuf> {
    let 记号 = std::fs::read_to_string(缓存目录(项目根).join("rust解析-指纹.txt")).ok()?;
    if 记号.trim() != format!("{指纹:016x}") {
        return None;
    }
    let 产出 = 缓存目录(项目根).join("rust解析-产出.json");
    产出.is_file().then_some(产出)
}

/// 写回缓存。顺序要紧：产出先落盘、指纹后写——指纹是「此产出有效」的标记，
/// 若先写指纹而产出写失败，就会留下错配，下次把陈旧产出当成新的复用。
fn 写回缓存(项目根: &Path, 指纹: u64, 产出路径: &Path) {
    let 目录 = 缓存目录(项目根);
    if std::fs::create_dir_all(&目录).is_err() {
        return;
    }
    if std::fs::copy(产出路径, 目录.join("rust解析-产出.json")).is_err() {
        return;
    }
    let _ = std::fs::write(目录.join("rust解析-指纹.txt"), format!("{指纹:016x}"));
}

/// 沿目录向上找最近的 `Cargo.toml`，取其 `[package] name` 作为 crate 名
fn 找包名(项目根: &Path, 相对: &str, 缓存: &mut BTreeMap<PathBuf, String>) -> String {
    let mut 当前 = 项目根.join(相对);
    当前.pop();

    loop {
        if let Some(名) = 缓存.get(&当前) {
            return 名.clone();
        }
        let 清单 = 当前.join("Cargo.toml");
        if 清单.is_file() {
            if let Some(名) = 读包名(&清单) {
                缓存.insert(当前.clone(), 名.clone());
                return 名;
            }
        }
        if !当前.pop() {
            break;
        }
    }

    "(未知crate)".to_string()
}

fn 读包名(清单路径: &Path) -> Option<String> {
    let 文本 = std::fs::read_to_string(清单路径).ok()?;
    let 值: toml::Value = toml::from_str(&文本).ok()?;
    值.get("package")?
        .get("name")?
        .as_str()
        .map(|名| 名.to_string())
}
