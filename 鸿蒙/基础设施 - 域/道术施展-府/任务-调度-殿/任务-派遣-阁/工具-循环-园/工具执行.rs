//! 工具 - 执行：执行单个工具调用 + 落盘护栏 + 参数摘要。
//! 工具流水线四段（对齐 dsh tool-execution-pipeline）：预执行(护栏) → 守卫(涉及路径) → 执行(映射) → 后执行。

use crate::{
    写文件, 列举目录, 删文件, 寻找文件, 搜索内容, 改文件, 校验命令护栏, 沙箱视图, 读文件
};
use moxing_fu::工具调用;
use shihai_fu::{工作区, 模型存储};
use shijian_fu::{守卫, 工具流水线, 工具结果, 工具请求, 裁决};
use std::path::{Path, PathBuf};

/// 单文件落盘内容上限（字节）：超限拒写，防一次性灌爆盘面。
pub(crate) const 落盘内容上限: usize = 512 * 1024;

/// 源码维度缓存：目录绝对路径 → 该目录（含子孙）是否含源码（.rs / Cargo.toml）。
/// 写文件次次校验，递归扫盘太贵；维度归属基本不变，进程内缓存一次即可。
static 源码维度缓存: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<PathBuf, bool>>,
> = std::sync::OnceLock::new();

/// 目录（含子孙）是否含源码：递归找 `.rs` 或 `Cargo.toml`，跳过构建/记忆等非源码目录。
/// 命中缓存直接返回；未命中则实扫一次并落缓存。
fn 目录含源码(目录: &Path) -> bool {
    let 缓存 = 源码维度缓存.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    if let Some(&命中) = 缓存.lock().unwrap().get(目录) {
        return 命中;
    }
    fn 递归找(目录: &std::path::Path) -> bool {
        let Ok(条目们) = std::fs::read_dir(目录) else {
            return false;
        };
        for 条目 in 条目们.flatten() {
            let 路径 = 条目.path();
            let 名 = 条目.file_name().to_string_lossy().to_string();
            if 路径.is_dir() {
                // 构建产物/记忆/版本库目录不视为源码维度，跳过防误判与耗时
                if matches!(名.as_str(), "target" | ".git" | ".上下文" | "道果树") {
                    continue;
                }
                if 递归找(&路径) {
                    return true;
                }
            } else if 名.ends_with(".rs") || 名 == "Cargo.toml" {
                return true;
            }
        }
        false
    }
    let 有 = 递归找(目录);
    缓存.lock().unwrap().insert(目录.to_path_buf(), 有);
    有
}

/// 根内越界校验（源码维度白名单）：目标路径必须落在「含源码的维度目录」内。
/// - 根级文件（Cargo.toml/AGENTS.md/设计稿 .md）一律拒写；
/// - 点开头隐藏目录（.上下文/.git）拒写（记忆/版本库/依赖图等非源码资产，执行者不得写）；
/// - 首段目录已存在但无源码（太初等空壳维度）拒写；
/// - 首段目录尚不存在（新园/新阁首次落盘）放行，由调用方后续 canonicalize 根内校验把关。
///   写/改/删共用同一把尺；路径须已用 `/` 归一、去首尾空白。
pub fn 校验路径范围(根: &Path, 路径: &str) -> Result<(), String> {
    let 首段 = 路径.split('/').next().unwrap_or("");
    if 路径.contains('/') {
        if 首段.starts_with('.') {
            return Err(format!(
                "根内越界拒绝：{路径}（隐藏目录 {首段} 非源码资产，拒写；执行者只能写源码维度目录）"
            ));
        }
        let 维度目录 = 根.join(首段);
        if 维度目录.is_dir() && !目录含源码(&维度目录) {
            return Err(format!(
                "根内越界拒绝：{路径}（{首段} 为非源码维度，拒写；新文件只能建在含源码的维度目录内）"
            ));
        }
    } else if !路径.contains('/') && !首段.is_empty() && !首段.ends_with(".rs") {
        return Err(format!(
            "根内越界拒绝：{路径}（根级非源码文件拒写；源码 .rs 可建在根级，其余文件须在维度目录内）"
        ));
    }
    Ok(())
}

/// 落盘护栏：不依赖模型自觉，系统侧强制。
/// 1) 内容为空拒写——防空文件静默破坏（空文件编译通过但内容全丢）；
/// 2) 内容超长拒写——防一次性灌爆盘面/上下文；
/// 3) 根内越界拒绝（源码维度白名单）——见 校验路径范围；
/// 4) 路径越界拒绝——从目标路径逐级上溯最近已存在祖先，规范化后必须位于工作区根内（防 ../ 逃逸）。
///    工具模式与纯文本回退共用，保证两条落盘路径同等受约束。
pub fn 校验落盘(根: &Path, 路径: &str, 内容: &str) -> Result<(), String> {
    let 路径 = 路径.trim().replace('\\', "/");
    if 路径.is_empty() {
        return Err("拒写：路径为空".to_string());
    }
    if 内容.trim().is_empty() {
        return Err(format!("拒写空文件：{路径}（内容为空）"));
    }
    if 内容.len() > 落盘内容上限 {
        return Err(format!(
            "拒写超长文件：{路径}（{} 字节，超上限 {落盘内容上限}）",
            内容.len()
        ));
    }
    校验路径范围(根, &路径)?;
    校验路径越界(根, &路径)
}

/// 路径越界校验（canonicalize）：从目标路径逐级上溯最近已存在祖先，
/// 规范化后必须位于工作区根内（防 `../` 逃逸）。
/// 写/改/删共用同一把尺；路径须已用 `/` 归一、去首尾空白、经 校验路径范围 白名单把关。
/// 提取自 校验落盘 第 4 段，供 删文件 对齐写/改的 canonicalize 越界校验（安全报告 L8）。
fn 校验路径越界(根: &Path, 路径: &str) -> Result<(), String> {
    let 根规范 = 根
        .canonicalize()
        .map_err(|错误| format!("工作区根无法解析：{错误}"))?;
    let 绝对 = 根.join(路径);
    let mut 检查 = 绝对.as_path();
    loop {
        if let Ok(规范) = 检查.canonicalize() {
            if !规范.starts_with(&根规范) {
                return Err(format!(
                    "路径越界拒绝：{路径}（已解析到 {}，超出工作区根 {}）",
                    规范.display(),
                    根规范.display()
                ));
            }
            return Ok(());
        }
        let Some(父) = 检查.parent() else { break };
        if 父 == 检查 {
            break;
        }
        检查 = 父;
    }
    Err(format!(
        "路径无法解析：{路径}（最近已存在祖先不在工作区根 {} 内）",
        根规范.display()
    ))
}

/// 涉及路径护栏：目标路径必须落在要求书涉及路径内——
/// ① 目标 == 涉及路径；② 目标在涉及路径（目录）树下；③ 目标与涉及路径（文件）同目录
/// （同园新建/修改，如为被测文件补内联测试或同园新文件）。
/// 涉及路径为空（审验/核查类）→ 放行（纪律靠执行提示）。机制性约束，不硬编码路径。
pub fn 校验涉及路径(原始: &str, 涉及路径: &[String]) -> Result<(), String> {
    if 涉及路径.is_empty() {
        return Ok(());
    }
    let 目标 = 原始.trim().replace('\\', "/");
    // 预先把涉及路径全部 normalize 一次成 Vec<(String, String)>：
    // (归一化路径, 归一化前缀含结尾斜杠)。
    // 循环内直接 .starts_with(归一化前缀)，避免 iter().any() 每次都 replace('\\',"/") 分配新 String、
    // 也避免 &format!("{涉及}/") 每轮分配新 String（要求-40 性能优化，2026-08-25 落地）。
    let 涉及规范: Vec<(String, String)> = 涉及路径
        .iter()
        .map(|涉及| {
            let 归一 = 涉及.replace('\\', "/");
            let 前缀 = format!("{归一}/");
            (归一, 前缀)
        })
        .collect();
    let 在内 = 涉及规范.iter().any(|(涉及, 前缀)| {
        if 目标 == 涉及.as_str() || 目标.starts_with(前缀.as_str()) {
            return true;
        }
        // 同目录：仅当涉及路径形态为文件（末段含扩展名）时，允许改/建同目录文件（补测试/同园场景）；
        // 目录型涉及路径走 ② 目录树前缀，不适用同目录（防兄弟目录误放行）。
        let 涉及是文件 = 涉及
            .rsplit('/')
            .next()
            .map(|末| 末.contains('.'))
            .unwrap_or(false);
        match 涉及.rsplit_once('/') {
            Some((涉及父, _)) if 涉及是文件 => 目标
                .rsplit_once('/')
                .map(|(目标父, _)| 目标父 == 涉及父)
                .unwrap_or(false),
            _ => false,
        }
    });
    if 在内 {
        Ok(())
    } else {
        Err(format!(
            "涉及路径外拒写：{目标}（只允许落在要求书涉及路径内或同目录：{}）——如需改其他文件，请界主把目标路径写进涉及路径",
            涉及路径.join("、")
        ))
    }
}

/// 工作目录根内校验：join 后 canonicalize 必须仍落在工作区根内，防 `../` 逃逸。
/// 返回规范化后的绝对路径字符串（供沙箱运行直接用）。
/// 校验对齐 校验落盘 的路径越界段：从目标逐级上溯最近已存在祖先，规范化后必须位于根内。
fn 校验工作目录(根: &Path, 相对: &str) -> Result<String, String> {
    let 相对 = 相对.trim().replace('\\', "/");
    if 相对.is_empty() {
        return Ok(根
            .canonicalize()
            .map_err(|错误| format!("工作区根无法解析：{错误}"))?
            .to_string_lossy()
            .into_owned());
    }
    let 根规范 = 根
        .canonicalize()
        .map_err(|错误| format!("工作区根无法解析：{错误}"))?;
    let 绝对 = 根.join(&相对);
    let mut 检查 = 绝对.as_path();
    loop {
        if let Ok(规范) = 检查.canonicalize() {
            if !规范.starts_with(&根规范) {
                return Err(format!(
                    "工作目录越界拒绝：{相对}（已解析到 {}，超出工作区根 {}）",
                    规范.display(),
                    根规范.display()
                ));
            }
            return Ok(规范.to_string_lossy().into_owned());
        }
        let Some(父) = 检查.parent() else { break };
        if 父 == 检查 {
            break;
        }
        检查 = 父;
    }
    Err(format!(
        "工作目录无法解析：{相对}（最近已存在祖先不在工作区根 {} 内）",
        根规范.display()
    ))
}

/// 格位名校验：只允许中文+字母+数字+下划线，拒路径分隔符（/和\）和 `..`。
/// 读格位/查格位历史直接把格位名 join 到格位目录（识海承载-府 模型落盘-园 格位文件路径），
/// 格位名未经校验可含 `../` 逃逸到格位目录之外，故在工具侧 join 前先校验。
fn 校验格位名(格位名: &str) -> Result<(), String> {
    if 格位名.is_empty() {
        return Err("格位名为空".to_string());
    }
    // 字符白名单：中文（CJK 基本区）+ ASCII 字母数字 + 下划线。
    // 此白名单已拒 `/`、`\`、`.`，故 `..` 与 `../` 自然被拒，错误信息一并提示。
    for 字符 in 格位名.chars() {
        let 合法 = 字符.is_ascii_alphanumeric()
            || 字符 == '_'
            || ('\u{4e00}'..='\u{9fff}').contains(&字符);
        if !合法 {
            return Err(format!(
                "格位名含非法字符 {字符} 拒绝：{格位名}（只允许中文/字母/数字/下划线，拒路径分隔符和 ..）"
            ));
        }
    }
    Ok(())
}

/// 工具护栏（guard 阶段）：执行前统一护栏，与 execute 解耦（对齐 DeepSeek 工具流水线）。
/// 本质：任何项目的通用安全护栏——落盘非空/非超长/路径不越界，命令不越权。
/// 读类工具（读文件/列举/寻找/搜索/读格位/查格位历史）天然只读，无护栏。
pub(crate) fn 工具护栏(
    根: &Path,
    调用名: &str,
    参数: &serde_json::Value,
) -> Result<(), String> {
    match 调用名 {
        "写文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 内容 = 参数["内容"].as_str().ok_or("缺参数 内容")?;
            校验落盘(根, 路径, 内容)
        }
        "改文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 新文 = 参数["新文"].as_str().ok_or("缺参数 新文")?;
            校验落盘(根, 路径, 新文)
        }
        "运行命令" => {
            let 命令 = 参数["命令"].as_str().ok_or("缺参数 命令")?;
            let 参数们 = 参数["参数们"]
                .as_array()
                .map(|数组| 数组.iter().filter_map(|值| 值.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let 超时毫秒 = 参数.get("超时毫秒").and_then(|值| 值.as_u64());
            校验命令护栏(命令, &参数们, 超时毫秒)
        }
        _ => Ok(()),
    }
}

/// 涉及路径守卫（guards 段，单调 deny-or-abstain）：
/// 写/改/删文件的目标路径必须落在要求书涉及路径内（目标 == 涉及路径，或位于涉及路径目录树下）。
/// 防模型越界改无关文件（2026-08-17 实测：模型改 证道/Cargo.toml 致 cargo 连锁污染根 Cargo.lock）。
/// 涉及路径为空（审验类）时弃权放行（纪律靠执行提示）。identity = 路径，只裁决落盘类工具。
struct 涉及路径守卫;

impl 守卫 for 涉及路径守卫 {
    fn 裁决(&self, 请求: &工具请求) -> 裁决 {
        if !matches!(请求.调用名.as_str(), "写文件" | "改文件" | "删文件") {
            return 裁决::弃权;
        }
        let 路径们: Vec<String> = {
            let mut 全部 = Vec::new();
            if let Some(路径) = 请求.参数["路径"].as_str() {
                全部.push(路径.to_string());
            }
            if let Some(数组) = 请求.参数["路径们"].as_array() {
                for 值 in 数组 {
                    if let Some(路径) = 值.as_str() {
                        全部.push(路径.to_string());
                    }
                }
            }
            全部
        };
        for 路径 in &路径们 {
            if let Err(说明) = 校验涉及路径(路径, &请求.涉及路径) {
                return 裁决::拒绝(说明);
            }
        }
        裁决::弃权
    }
}

/// 执行器（execute 段）：映射 手脚-施展-殿 的真实函数，产出 工具结果（文本 + 写入文件）。
///
/// # 读类工具安全边界
///
/// 读文件/列举目录/寻找文件/搜索内容 为读类工具，设计上开放读取全工作区文件（模型需读源码上下文）。
/// 已知边界：可读取 .env（含 API 密钥）、.上下文/观测（含提示词）。
/// 缓解：.gitignore 已排除敏感数据不入库；观测探针正文封顶 200K 字符。
/// 若需限制，可在此处加敏感目录拒绝（与 校验路径范围 的隐藏目录拒绝复用）。
fn 执行器(请求: &工具请求) -> Result<工具结果, String> {
    let 根 = &请求.工作区根;
    let 参数 = &请求.参数;
    let mut 写入文件们: Vec<(String, u64)> = Vec::new();
    let mut 尝试写入们: Vec<String> = Vec::new();
    let 文本 = match 请求.调用名.as_str() {
        "写文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 内容 = 参数["内容"].as_str().ok_or("缺参数 内容")?;
            let 绝对 = 根.join(路径);
            尝试写入们.push(路径.to_string());
            let 已写入 = 写文件(绝对.to_str().ok_or("路径含非 UTF-8 字符")?, 内容)?;
            if 已写入 {
                写入文件们.push((路径.to_string(), 内容.len() as u64));
                Ok(format!("已写入 {路径}（{} 字节）", 内容.len()))
            } else {
                // 空操作：内容与现状相同，未写盘（不入产物清单）。
                Ok(format!(
                    "跳过写入 {路径}：内容与现状相同（空操作，未改盘面）"
                ))
            }
        }
        // 读类工具：开放读取全工作区，安全边界见 执行器 文档注释。
        "读文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 绝对 = 根.join(路径);
            let 内容 = 读文件(绝对.to_str().ok_or("路径含非 UTF-8 字符")?)?;
            Ok(format!("【文件：{路径}】\n{内容}"))
        }
        "改文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 旧文 = 参数["旧文"].as_str().ok_or("缺参数 旧文")?;
            let 新文 = 参数["新文"].as_str().ok_or("缺参数 新文")?;
            let 绝对 = 根.join(路径);
            尝试写入们.push(路径.to_string());
            let 已改写 = 改文件(绝对.to_str().ok_or("路径含非 UTF-8 字符")?, 旧文, 新文)?;
            if 已改写 {
                写入文件们.push((路径.to_string(), 新文.len() as u64));
                Ok(format!("已改写 {路径}"))
            } else {
                // 空操作：替换结果与原文相同，未写盘（不入产物清单）。
                Ok(format!(
                    "跳过改写 {路径}：替换结果与现状相同（空操作，未改盘面）"
                ))
            }
        }
        "删文件" => {
            let 路径们 = 参数["路径们"].as_array().ok_or("缺参数 路径们")?;
            let 文本们 = 路径们
                .iter()
                .filter_map(|值| 值.as_str())
                .collect::<Vec<_>>();
            if 文本们.is_empty() {
                return Err("路径们 为空".to_string());
            }
            // 删文件同走根内越界校验（源码维度白名单）+ canonicalize 路径越界校验，
            // 防执行者删根级/记忆/空壳维度资产，并防 `../` 逃逸（与写/改对齐，安全报告 L8）。
            for 路径 in &文本们 {
                let 归一 = 路径.trim().replace('\\', "/");
                if 归一.is_empty() {
                    return Err("拒删：路径为空".to_string());
                }
                校验路径范围(根, &归一)?;
                校验路径越界(根, &归一)?;
            }
            let 绝对的 = 文本们.iter().map(|路径| 根.join(路径)).collect::<Vec<_>>();
            let 字符串们 = 绝对的
                .iter()
                .map(|路径| 路径.to_string_lossy().into_owned())
                .collect::<Vec<String>>();
            let 参数们 = 字符串们
                .iter()
                .map(|路径| 路径.as_str())
                .collect::<Vec<_>>();
            删文件(&参数们)?;
            Ok(format!("已删除 {} 个文件", 参数们.len()))
        }
        // 读类工具：开放列举全工作区目录，安全边界见 执行器 文档注释。
        "列举目录" => {
            let 路径 = 参数["路径"].as_str().unwrap_or("");
            let 绝对 = 根.join(路径);
            let 条目们 = 列举目录(绝对.to_str().ok_or("路径含非 UTF-8 字符")?)?;
            let mut 行 = String::new();
            for 条目 in 条目们 {
                行.push_str(&format!(
                    "{}{}（{} 字节）\n",
                    条目.名称,
                    if 条目.是目录 { "/" } else { "" },
                    条目.字节数
                ));
            }
            Ok(行)
        }
        // 读类工具：开放全工作区文件查找，安全边界见 执行器 文档注释。
        "寻找文件" => {
            let 根参 = 参数["根"].as_str().unwrap_or("");
            let 模式 = 参数["模式"].as_str().ok_or("缺参数 模式")?;
            let 绝对 = 根.join(根参);
            let 文件们 = 寻找文件(绝对.to_str().ok_or("路径含非 UTF-8 字符")?, 模式)?;
            if 文件们.is_empty() {
                return Ok(工具结果::新("（未找到匹配文件）"));
            }
            Ok(文件们
                .iter()
                .take(100)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"))
        }
        // 读类工具：开放全工作区内容搜索，安全边界见 执行器 文档注释。
        "搜索内容" => {
            let 根参 = 参数["根"].as_str().unwrap_or("");
            let 字面串 = 参数["字面串"].as_str().ok_or("缺参数 字面串")?;
            let 绝对 = 根.join(根参);
            let 命中们 = 搜索内容(绝对.to_str().ok_or("路径含非 UTF-8 字符")?, 字面串)?;
            if 命中们.is_empty() {
                return Ok(工具结果::新("（未检索到命中）"));
            }
            let mut 行 = String::new();
            for 命中 in 命中们.iter().take(60) {
                行.push_str(&format!(
                    "{}:{} {}\n",
                    命中.路径,
                    命中.行号,
                    命中.行内容.trim()
                ));
            }
            Ok(行)
        }
        "运行命令" => {
            let 命令 = 参数["命令"].as_str().ok_or("缺参数 命令")?;
            let 参数们 = 参数["参数们"]
                .as_array()
                .map(|数组| 数组.iter().filter_map(|值| 值.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let 超时毫秒 = 参数.get("超时毫秒").and_then(|值| 值.as_u64());
            let 工作目录 = 参数["工作目录"]
                .as_str()
                .map(|相对| 校验工作目录(根, 相对))
                .transpose()?;
            // 沙箱执行：命令在隔离视图内跑，构建物落视图内，越界写入自动回滚，真实盘面零污染。
            let 沙箱 = 沙箱视图::打开当前(根);
            let 回执 = 沙箱.运行(命令, &参数们, 工作目录.as_deref(), 超时毫秒)?;
            let 结果 = &回执.结果;
            let mut 输出 = format!(
                "退出码：{:?}\n标准输出：\n{}\n标准错误：\n{}",
                结果.退出码, 结果.标准输出, 结果.标准错误
            );
            if 回执.越界数 > 0 {
                输出.push_str(&format!(
                    "\n【沙箱已拦截并回滚 {} 处越界写入，真实工作区未被改动】\n{}",
                    回执.越界数, 回执.越界详情
                ));
            }
            if let Some(超时) = 超时毫秒 {
                输出.push_str(&format!("\n【超时上限：{} 毫秒】\n", 超时));
            }
            Ok(输出)
        }
        "读格位" => {
            let 格位名 = 参数["格位名"].as_str().ok_or("缺参数 格位名")?;
            校验格位名(格位名)?;
            let 上限 = 参数["上限"].as_u64().unwrap_or(20).min(200) as usize;
            let 工作区 = 工作区::定位();
            let 存储 = 模型存储::在工作区(&工作区);
            let 链头 = 存储.读链头集(格位名).map_err(|e| e.to_string())?;
            let 取条 = 链头.len().saturating_sub(上限);
            let 窗口 = &链头[取条..];
            let mut 输出 = format!(
                "【格位：{格位名}】链头 {} 条（返回 {} 条）\n",
                链头.len(),
                窗口.len()
            );
            for (序号, 记录) in 窗口.iter().enumerate() {
                输出.push_str(&format!(
                    "#{} [ts={} 来源={:?}] {}\n",
                    序号 + 1 + 取条,
                    记录.时间戳,
                    记录.来源,
                    记录.内容
                ));
                if !记录.证据.is_empty() {
                    输出.push_str(&format!("    证据：{}\n", 记录.证据));
                }
            }
            Ok(输出)
        }
        "查格位历史" => {
            let 格位名 = 参数["格位名"].as_str().ok_or("缺参数 格位名")?;
            校验格位名(格位名)?;
            let 起始 = 参数["起始"].as_u64().unwrap_or(0) as usize;
            let 上限 = 参数["上限"].as_u64().unwrap_or(50).min(500) as usize;
            let 工作区 = 工作区::定位();
            let 存储 = 模型存储::在工作区(&工作区);
            let 全部 = 存储.读格位(格位名).map_err(|e| e.to_string())?;
            let 终 = (起始 + 上限).min(全部.len());
            if 起始 >= 全部.len() {
                return Ok(工具结果::新(format!(
                    "【格位：{格位名}】共 {} 条，偏移 {起始} 越界",
                    全部.len()
                )));
            }
            let 窗口 = &全部[起始..终];
            let mut 输出 = format!(
                "【格位：{格位名}】共 {} 条，返回第 {}..{} 条（{} 条）\n",
                全部.len(),
                起始,
                终,
                窗口.len()
            );
            for (序号, 记录) in 窗口.iter().enumerate() {
                let 失效 = if 记录.失效 { " [失效]" } else { "" };
                输出.push_str(&format!(
                    "#{} [ts={} 来源={:?}]{} {}\n",
                    起始 + 序号 + 1,
                    记录.时间戳,
                    记录.来源,
                    失效,
                    记录.内容
                ));
                if !记录.证据.is_empty() {
                    输出.push_str(&format!("    证据：{}\n", 记录.证据));
                }
            }
            Ok(输出)
        }
        _ => Err(format!("未知工具：{}", 请求.调用名)),
    }?;
    Ok(工具结果 {
        文本,
        写入文件们,
        尝试写入们,
    })
}

/// 工具流水线实例：预执行(护栏) → 守卫(涉及路径) → 执行(映射) → 后执行（当前留空，后续挂观测留痕）。
/// OnceLock 静态构造一次，注册项全局唯一（对齐 dsh 工具注册表单一实例）。
static 工具流水线实例: std::sync::OnceLock<工具流水线> = std::sync::OnceLock::new();

/// 预执行监听（pre-execute）：统一护栏——落盘非空/非超长/路径不越界，命令不越权。
/// 本质：任何项目的通用安全护栏（对齐 dsh guard 阶段，与 execute 解耦）。
fn 工具护栏监听(请求: &mut 工具请求) -> Result<(), String> {
    工具护栏(&请求.工作区根, &请求.调用名, &请求.参数)
}

/// 取全局工具流水线（首次构造并注册护栏/守卫；后执行段由后续阶段挂观测留痕）。
/// 注册的注销句柄用 Box::leak 永久持有——静态流水线生命周期与进程一致，句柄不得 drop（drop 即注销监听）。
pub(crate) fn 流水线() -> &'static 工具流水线 {
    工具流水线实例.get_or_init(|| {
        let mut 流水线 = 工具流水线::新(执行器);
        // pre-execute：统一护栏（落盘校验/命令护栏）。句柄永久持有，防注册即注销。
        let 护栏句柄 = 流水线.预执行注册(工具护栏监听);
        let _永久持有 = Box::leak(Box::new(护栏句柄));
        // guards：涉及路径守卫（单调 deny-or-abstain）。
        流水线.加守卫(std::sync::Arc::new(涉及路径守卫));
        流水线
    })
}

/// 执行单个工具调用（对外签名不变，内部走工具流水线四段）。
/// 涉及路径护栏、落盘护栏均由流水线 预执行/守卫 段承载（对齐 dsh tool-execution-pipeline）。
pub(crate) fn 执行工具(
    调用: &工具调用,
    根: &Path,
    写入文件们: &mut Vec<(String, u64)>,
    涉及路径: &[String],
) -> Result<String, String> {
    let 参数: serde_json::Value = serde_json::from_str(&调用.参数)
        .map_err(|错误| format!("参数解析失败: {错误}；原文：{}", 调用.参数))?;
    let 请求 = 工具请求::新(调用.名字.clone(), 参数, 根.to_path_buf(), 涉及路径.to_vec());
    let 结果 = 流水线().执行(&请求)?;
    写入文件们.extend(结果.写入文件们);
    Ok(结果.文本)
}

/// 工具参数摘要：只取轻量字段（路径/命令等），不打印大内容，供日志定位是哪一步卡住。
pub(crate) fn 参数摘要(调用: &工具调用) -> String {
    let Ok(参数) = serde_json::from_str::<serde_json::Value>(&调用.参数) else {
        return 调用.参数.chars().take(80).collect();
    };
    match 调用.名字.as_str() {
        "写文件" | "读文件" | "改文件" => 参数["路径"].as_str().unwrap_or("?").to_string(),
        "删文件" => serde_json::to_string(&参数["路径们"]).unwrap_or_default(),
        "列举目录" => 参数["路径"].as_str().unwrap_or("（根）").to_string(),
        "寻找文件" => format!(
            "根={} 模式={}",
            参数["根"].as_str().unwrap_or("（根）"),
            参数["模式"].as_str().unwrap_or("?")
        ),
        "搜索内容" => format!(
            "根={} 字面串={}",
            参数["根"].as_str().unwrap_or("（根）"),
            参数["字面串"].as_str().unwrap_or("?")
        ),
        "运行命令" => format!(
            "{} {:?}",
            参数["命令"].as_str().unwrap_or("?"),
            参数["参数们"]
                .as_array()
                .map(|数组| 数组.iter().filter_map(|项| 项.as_str()).collect::<Vec<_>>())
                .unwrap_or_default()
        ),
        "读格位" => format!(
            "格位={} 上限={}",
            参数["格位名"].as_str().unwrap_or("?"),
            参数["上限"].as_u64().unwrap_or(20)
        ),
        "查格位历史" => format!(
            "格位={} 起始={} 上限={}",
            参数["格位名"].as_str().unwrap_or("?"),
            参数["起始"].as_u64().unwrap_or(0),
            参数["上限"].as_u64().unwrap_or(50)
        ),
        _ => 调用.参数.chars().take(80).collect(),
    }
}

#[cfg(test)]
mod 测试 {
    use super::{参数摘要, 校验格位名, 校验涉及路径};
    use moxing_fu::工具调用;

    #[test]
    fn 涉及路径_文件精确与目录树内放行() {
        let 涉及 = vec![
            "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-回放-园/流式回放.rs"
                .to_string(),
            "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-直播-园".to_string(),
        ];
        assert!(
            校验涉及路径(
                "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-回放-园/流式回放.rs",
                &涉及
            )
            .is_ok(),
            "涉及文件本身可写"
        );
        assert!(
            校验涉及路径(
                "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-回放-园/流式回放测试.rs",
                &涉及
            )
            .is_ok(),
            "涉及文件同目录可新建（补测试场景）"
        );
        assert!(
            校验涉及路径(
                "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-直播-园/流式直播.rs",
                &涉及
            )
            .is_ok(),
            "涉及目录树下可写"
        );
        assert!(
            校验涉及路径(
                "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-回放-园",
                &涉及
            )
            .is_err(),
            "涉及文件的上级目录本身不可写"
        );
    }

    #[test]
    fn 涉及路径_越界拒绝() {
        let 涉及 = vec![
            "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-回放-园/流式回放.rs"
                .to_string(),
        ];
        let 错 = 校验涉及路径("证道/鸿蒙 - 域/单元测试-府/Cargo.toml", &涉及).unwrap_err();
        assert!(错.contains("涉及路径外拒写"), "涉及路径外应拒写：{错}");
        assert!(
            校验涉及路径("鸿蒙/基础设施 - 域/天庭治理-府/Cargo.toml", &涉及).is_err(),
            "其他府文件拒写"
        );
        // 相似前缀不可误放行：涉及 甲-园，不能写 甲-园X。
        let 涉及目录 = vec!["乾坤/甲-园".to_string()];
        assert!(
            校验涉及路径("乾坤/甲-园X/乙.rs", &涉及目录).is_err(),
            "前缀相似不应放行"
        );
        // 同目录放行不跨目录：涉及 流式-回放-园 下的文件，不能写 流式-直播-园。
        assert!(
            校验涉及路径(
                "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-直播-园/流式直播.rs",
                &涉及
            )
            .is_err(),
            "同目录规则不跨目录"
        );
    }

    #[test]
    fn 涉及路径_空放行() {
        assert!(
            校验涉及路径("任何/路径.rs", &[]).is_ok(),
            "审验类无涉及路径应放行"
        );
    }

    /// 参数摘要 · 写文件：摘要返回 路径 字段值，不含 内容 字段。
    /// 写文件调用的 JSON 参数键为 路径/内容（与 crate 根 写文件 工具签名对齐），
    /// 摘要仅取 路径，避免 内容 灌入日志导致上下文爆掉。
    /// 对应工具执行.rs 顶部 `pub(crate) fn 参数摘要` 的「写文件|读文件|改文件」分支。
    #[test]
    fn 参数摘要_写文件返回路径且不泄露内容() {
        let 调用 = 工具调用 {
            标识: "t1".to_string(),
            名字: "写文件".to_string(),
            参数:
                r#"{"路径":"鸿蒙/基础设施 - 域/甲-府/乙-阁/丙-园/甲.rs","内容":"pub fn 甲() {}"}"#
                    .to_string(),
        };
        let 摘要 = 参数摘要(&调用);
        assert!(
            摘要.contains("鸿蒙/基础设施 - 域/甲-府/乙-阁/丙-园/甲.rs"),
            "写文件摘要应完整包含路径：{摘要}"
        );
        assert!(
            !摘要.contains("pub fn"),
            "摘要不应包含 内容 字段（避免日志爆掉）：{摘要}"
        );
    }

    /// 参数摘要 · 写文件（项目真实中文路径）：除现有 `参数摘要_写文件返回路径且不泄露内容`
    /// 的占位命名外，再用项目真实路径（含 中文 + 空格 + 连字符）覆盖 contains 断言，
    /// 并断言 摘要 字符数等于 路径 字符数——与现有「不含 内容」断言互补，锁住「摘要 ≡ 路径」不变量。
    #[test]
    fn 参数摘要_写文件_项目中文路径完全等于路径() {
        let 路径 =
            "鸿蒙/基础设施 - 域/道术施展-府/任务-调度-殿/任务-派遣-阁/工具-循环-园/工具执行.rs";
        let 调用 = 工具调用 {
            标识: "t_项目路径".to_string(),
            名字: "写文件".to_string(),
            参数: format!(r#"{{"路径":"{路径}","内容":"fn 占位() {{}}\n"}}"#),
        };
        let 摘要 = 参数摘要(&调用);
        assert!(
            摘要.contains(路径),
            "项目中文路径应原样出现在摘要中：{摘要}"
        );
        assert_eq!(
            摘要.chars().count(),
            路径.chars().count(),
            "写文件摘要应与路径等长（无多余字符）：{摘要}"
        );
    }

    /// 参数摘要 · 非法 JSON 鲁棒性：调用.参数 不是合法 JSON 时函数不得 panic，
    /// 而应回退到「取前 80 字符」的截断行为（崩溃安全 + 截断行为正确）。
    /// 对应 `pub(crate) fn 参数摘要` 中 `serde_json::from_str` 失败时的 `else` 分支。
    #[test]
    #[allow(non_snake_case)]
    fn 参数摘要_非法JSON_不崩溃且截断到八十字符() {
        // 1) 非法 JSON + 长度未超 80 → 原样返回（不截断）。
        let 短乱码 = "hello world this is not json {{".to_string();
        let 调用 = 工具调用 {
            标识: "t".to_string(),
            名字: "写文件".to_string(),
            参数: 短乱码.clone(),
        };
        assert_eq!(
            参数摘要(&调用),
            短乱码,
            "非法 JSON 短串应原样返回（不触发截断）"
        );

        // 2) 非法 JSON + 长度 > 80 → 截断到前 80 字符（按 char 计数，非字节）。
        let 长乱码: String = "x".repeat(200);
        let 调用 = 工具调用 {
            标识: "t".to_string(),
            名字: "写文件".to_string(),
            参数: 长乱码.clone(),
        };
        let 摘要 = 参数摘要(&调用);
        assert_eq!(
            摘要.chars().count(),
            80,
            "非法 JSON 长串应按 char 截断到 80 字符：{摘要}"
        );
        assert_eq!(
            摘要,
            长乱码.chars().take(80).collect::<String>(),
            "截断后内容应等于原文前 80 字符"
        );

        // 3) 半截 JSON（未闭合括号）：典型模型抽风产物，应原样返回。
        let 半截 = r#"{"路径":"a.rs","内容":"半"#;
        let 调用 = 工具调用 {
            标识: "t".to_string(),
            名字: "读文件".to_string(),
            参数: 半截.to_string(),
        };
        let 摘要 = 参数摘要(&调用);
        assert_eq!(摘要, 半截, "未闭合 JSON 应回退到原文截断");

        // 4) 未知工具名 + 非法 JSON：走 match 默认分支（截断到 80 字符）。
        let 调用 = 工具调用 {
            标识: "t".to_string(),
            名字: "不存在的工具".to_string(),
            参数: "{garbage".to_string(),
        };
        let 摘要 = 参数摘要(&调用);
        assert_eq!(
            摘要, "{garbage",
            "未知工具 + 非法 JSON 应走兜底截断（< 80 字符原样返回）"
        );
    }

    /// 参数摘要 · 空字符串参数：空参数 不应 panic；返回空字符串（崩溃安全）。
    /// 覆盖 写文件 / 未知工具 两条回退路径的空串边界。
    #[test]
    fn 参数摘要_空字符串参数不崩溃() {
        let 调用 = 工具调用 {
            标识: "t".to_string(),
            名字: "写文件".to_string(),
            参数: "".to_string(),
        };
        assert_eq!(参数摘要(&调用), "", "空字符串参数应返回空摘要而非崩溃");

        let 调用 = 工具调用 {
            标识: "t".to_string(),
            名字: "未知工具".to_string(),
            参数: "".to_string(),
        };
        assert_eq!(参数摘要(&调用), "", "未知工具 + 空参数应返回空摘要");
    }

    /// 参数摘要 · 中文字符按 char 截断（不按字节）：防 UTF-8 切到字节中间产生乱码/panic。
    /// 100 个「字」= 300 字节 UTF-8，按 char 截断应得 80 个完整「字」。
    #[test]
    #[allow(non_snake_case)]
    fn 参数摘要_非法JSON_中文字符按char截断() {
        let 中长: String = "字".repeat(100);
        let 字节长 = 中长.len();
        assert!(
            字节长 > 80,
            "构造前提：100 字 应 > 80 字节（{字节长}），否则用例无意义"
        );
        let 调用 = 工具调用 {
            标识: "t".to_string(),
            名字: "写文件".to_string(),
            参数: 中长.clone(),
        };
        let 摘要 = 参数摘要(&调用);
        assert_eq!(
            摘要.chars().count(),
            80,
            "中文应按 char 截断到 80 字符（非 80 字节）：{摘要}"
        );
        assert_eq!(
            摘要,
            "字".repeat(80),
            "中文截断后应是 80 个完整「字」（无切字节乱码）"
        );
    }

    /// 格位名校验 · 合法格位名放行：中文+字母+数字+下划线组合均合法。
    #[test]
    fn 校验格位名_合法名放行() {
        assert!(校验格位名("结构").is_ok(), "纯中文合法");
        assert!(校验格位名("理解_记忆").is_ok(), "中文+下划线合法");
        assert!(校验格位名("chain_001").is_ok(), "字母+下划线+数字合法");
        assert!(校验格位名("理解记忆_2").is_ok(), "中文+下划线+数字合法");
    }

    /// 格位名校验 · 路径分隔符与 .. 拒绝：防 ../ 逃逸格位目录。
    #[test]
    fn 校验格位名_路径逃逸拒绝() {
        assert!(校验格位名("").is_err(), "空名拒绝");
        assert!(校验格位名("../逃逸").is_err(), "含 / 拒绝（路径分隔符）");
        assert!(校验格位名("..\\逃逸").is_err(), "含 \\ 拒绝（路径分隔符）");
        assert!(校验格位名("..").is_err(), ".. 拒绝（含非法字符 .）");
        assert!(校验格位名("a/../b").is_err(), "嵌套 ../ 拒绝");
        assert!(
            校验格位名("格位/子").is_err(),
            "看似无害的 / 也拒（格位名不分层）"
        );
    }

    /// 格位名校验 · 非法字符拒绝：白名单外的字符一律拒。
    #[test]
    fn 校验格位名_非法字符拒绝() {
        assert!(校验格位名("a b").is_err(), "空格非法");
        assert!(校验格位名("a.b").is_err(), "点号非法");
        assert!(校验格位名("要求-1").is_err(), "连字符非法");
        assert!(校验格位名("a:b").is_err(), "冒号非法");
        assert!(校验格位名("a*b").is_err(), "星号非法");
        assert!(校验格位名("a?b").is_err(), "问号非法");
        assert!(校验格位名("a|b").is_err(), "竖线非法");
        assert!(校验格位名("a<b").is_err(), "尖括号非法");
        assert!(校验格位名("a>b").is_err(), "尖括号非法");
        assert!(校验格位名("a\"b").is_err(), "引号非法");
    }
}
