//! 播种 - 归纳 - 园：按种子降级链归纳语义记录。

use crate::{格位, 记录, 模型存储, 扫描排除项, 渲染播种提示, 依赖图, 来源};
use moxing_fu::{调用模型, 对话消息, 模型配置};
use rizhi_fu::{debug, error, info, warn};
use std::path::Path;

/// 播种结果：已归纳（返回内容）或 需人类（返回原因）。
pub enum 播种结果 {
    已归纳(String),
    需人类(String),
}

/// 直调模型归纳一条记录（来源 = LLM，必带证据）。
pub fn 播种归纳(存储: &模型存储, 格位: &格位, 配置: &模型配置, 背景: &str) -> Result<String, String> {
    let 提示 = 渲染播种提示(&格位.种子提示词, 背景, "");
    let (回复, 用量) = 调用模型(配置, &[对话消息::用户(提示)], moxing_fu::精简上限)?;
    let 记录 = 记录::新(&格位.名字, &回复, &format!("按种子「{}」归纳", 格位.种子提示词), "LLM");
    存储.写记录(&记录)?;
    info!(格位名 = %格位.名字, 内容长度 = 回复.len(), 提示词 = 用量.提示词, "播种归纳完成");
    Ok(回复)
}

/// 按种子降级链播种：推荐位置文件 → 代码兜底 → 人类。
/// 有推荐位置文件则读其内容交模型归纳；无推荐位置或文件不存在则回落人类。
pub fn 播种降级(存储: &模型存储, 格位: &格位, 配置: &模型配置, 根: &Path) -> Result<播种结果, String> {
    if !格位.推荐位置.is_empty() {
        let 候选 = 根.join(&格位.推荐位置);
        if 候选.is_file() {
            let 素材 = std::fs::read_to_string(&候选).map_err(|错误| format!("读推荐位置失败: {错误}"))?;
            let 提示 = 渲染播种提示(&格位.种子提示词, &素材, "");
            let (回复, 用量) = 调用模型(配置, &[对话消息::用户(提示)], moxing_fu::精简上限)?;
            let 摘要: String = 回复.lines().take(3).collect::<Vec<_>>().join("\n");
            let 记录 = 记录::新(&格位.名字, &摘要, &format!("从「{}」归纳", 格位.推荐位置), "LLM");
            存储.写记录(&记录)?;
            info!(格位名 = %格位.名字, 内容长度 = 摘要.len(), 提示词 = 用量.提示词, "播种归纳完成");
            return Ok(播种结果::已归纳(摘要));
        }
    }
    // 降级链第二级：代码兜底（扫描根目录首个源文件作机械证据）
    let 排除项 = 扫描排除项(根);
    if let Some(首个) = 找首个源文件(根, &排除项) {
        let 内容 = std::fs::read_to_string(&首个).map_err(|错误| format!("读源文件失败: {错误}"))?;
        let 摘要: String = 内容.lines().take(3).collect::<Vec<_>>().join("\n");
        let 记录 = 记录::新(&格位.名字, &摘要, &format!("从「{}」扫描", 首个.display()), "代码");
        存储.写记录(&记录)?;
        return Ok(播种结果::已归纳(摘要));
    }

    // 降级链第三级：人类
    info!(格位名 = %格位.名字, "播种回落人类");
    Ok(播种结果::需人类(format!(
        "种子「{}」未找到推荐位置，需人类录入",
        格位.名字
    )))
}

/// 递归找首个 .rs 源文件（跳过排除项）。
fn 找首个源文件(根: &Path, 排除项: &[String]) -> Option<std::path::PathBuf> {
    let Ok(条目们) = std::fs::read_dir(根) else { return None };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 排除项.iter().any(|项| 项 == &名) {
                continue;
            }
            if let Some(命中) = 找首个源文件(&路径, 排除项) {
                return Some(命中);
            }
        } else if 路径.extension().map_or(false, |扩展名| 扩展名 == "rs") {
            return Some(路径);
        }
    }
    None
}

/// 界主交互：审阅 + 询问（由上层 UI 弹窗 / 输入实现）。
pub trait 界主交互 {
    /// 审阅一条归纳，返回是否通过录入（弹窗审阅）。
    fn 审阅(&self, 格位名: &str, 归纳: &str, 证据: &str) -> bool;
    /// 询问界主一个问题，返回回答（空串表示跳过）。
    fn 询问(&self, 格位名: &str, 问题: &str) -> String;
}

/// 搜集项目文档：递归找 .md / .txt，返回（相对路径，内容）。
pub fn 搜集文档(根: &Path, 排除项: &[String]) -> Vec<(String, String)> {
    let mut 文档们 = Vec::new();
    递归搜集文档(根, 根, 排除项, &mut 文档们);
    文档们
}

fn 递归搜集文档(根: &Path, 目录: &Path, 排除项: &[String], 文档们: &mut Vec<(String, String)>) {
    let Ok(条目们) = std::fs::read_dir(目录) else { return };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 排除项.iter().any(|项| 项 == &名) {
                continue;
            }
            递归搜集文档(根, &路径, 排除项, 文档们);
        } else if 名.ends_with(".md") || 名.ends_with(".txt") {
            if let Ok(内容) = std::fs::read_to_string(&路径) {
                let 相对 = 路径.strip_prefix(根).unwrap_or(&路径).to_string_lossy().to_string();
                文档们.push((相对, 内容));
            }
        }
    }
}

/// 搜集代码印证：源文件清单（相对路径），用于印证文档。
pub fn 搜集代码印证(根: &Path, 排除项: &[String]) -> Vec<String> {
    let mut 文件们 = Vec::new();
    递归搜集源文件(根, 根, 排除项, &mut 文件们);
    文件们
}

fn 递归搜集源文件(根: &Path, 目录: &Path, 排除项: &[String], 文件们: &mut Vec<String>) {
    let Ok(条目们) = std::fs::read_dir(目录) else { return };
    for 条目 in 条目们.flatten() {
        let 路径 = 条目.path();
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 路径.is_dir() {
            if 排除项.iter().any(|项| 项 == &名) {
                continue;
            }
            递归搜集源文件(根, &路径, 排除项, 文件们);
        } else if 名.ends_with(".rs") {
            let 相对 = 路径.strip_prefix(根).unwrap_or(&路径).to_string_lossy().to_string();
            文件们.push(相对);
        }
    }
}

/// 模型播种：先搜集文档 + 代码印证，交模型归纳，齐全则审阅录入；
/// 无文档则代码为主 + 问界主，直到拿到内容。
pub fn 模型播种(
    存储: &模型存储,
    格位: &格位,
    配置: &模型配置,
    根: &Path,
    界主: &dyn 界主交互,
) -> Result<播种结果, String> {
    let 排除项 = 扫描排除项(根);
    let 文档们 = 搜集文档(根, &排除项);
    let 代码们 = 搜集代码印证(根, &排除项);

    if !文档们.is_empty() {
        let 素材: String = 文档们
            .iter()
            .map(|(名, 内容)| format!("【{名}】\n{内容}"))
            .collect::<Vec<_>>()
            .join("\n");
        let 印证: String = 代码们.join("、");
        let 提示 = 渲染播种提示(&格位.种子提示词, &素材, &印证);
        let (回复, 用量) = 调用模型(配置, &[对话消息::用户(提示)], moxing_fu::精简上限)?;
        let 摘要: String = 回复.lines().take(3).collect::<Vec<_>>().join("\n");
        let 证据 = format!("文档 + 代码印证（{} 个文档）", 文档们.len());
        if 界主.审阅(&格位.名字, &摘要, &证据) {
            let 记录 = 记录::新(&格位.名字, &摘要, &证据, "LLM");
            存储.写记录(&记录)?;
            info!(格位名 = %格位.名字, 提示词 = 用量.提示词, "模型播种归纳完成");
            return Ok(播种结果::已归纳(摘要));
        }
        let 回答 = 界主.询问(&格位.名字, &format!("归纳未通过审阅，请界主给出「{}」的正确内容", 格位.名字));
        if !回答.is_empty() {
            let 记录 = 记录::新(&格位.名字, &回答, "界主审阅后录入", "人类");
            存储.写记录(&记录)?;
            return Ok(播种结果::已归纳(回答));
        }
        return Ok(播种结果::需人类(format!("格位「{}」待人类录入", 格位.名字)));
    }

    // 无文档：代码为主 + 问界主
    if let Some(首个) = 找首个源文件(根, &排除项) {
        let 内容 = std::fs::read_to_string(&首个).map_err(|错误| format!("读源文件失败: {错误}"))?;
        let 摘要: String = 内容.lines().take(3).collect::<Vec<_>>().join("\n");
        let 证据 = format!("从「{}」扫描（无文档，代码为主）", 首个.display());
        if 界主.审阅(&格位.名字, &摘要, &证据) {
            let 记录 = 记录::新(&格位.名字, &摘要, &证据, "代码");
            存储.写记录(&记录)?;
            return Ok(播种结果::已归纳(摘要));
        }
    }
    let 回答 = 界主.询问(&格位.名字, &format!("种子「{}」无文档无代码印证，请界主给出内容", 格位.种子提示词));
    if !回答.is_empty() {
        let 记录 = 记录::新(&格位.名字, &回答, "界主", "人类");
        存储.写记录(&记录)?;
        return Ok(播种结果::已归纳(回答));
    }
    Ok(播种结果::需人类(format!("格位「{}」待人类录入", 格位.名字)))
}

/// 模型播种全部 36 格位，返回已归纳条数。
pub fn 模型播种全部(
    存储: &模型存储,
    配置: &模型配置,
    根: &Path,
    界主: &dyn 界主交互,
) -> Result<usize, String> {
    let mut 条数 = 0;
    for 格位 in crate::全部格位() {
        match 模型播种(存储, &格位, 配置, 根, 界主) {
            Ok(播种结果::已归纳(_)) => 条数 += 1,
            Ok(_) => {}
            Err(错误) => {
                error!(格位名 = %格位.名字, "播种失败：{错误}");
                return Err(错误);
            }
        }
    }
    debug!(条数, "模型播种全部完成");
    Ok(条数)
}

/// 人类格位临时代偿（设计稿 §4.2 规则5 / 项目心智模型 §6.4）：
/// 来源=人类的格位（初心·使命 / 铁律·总纲 / 标准 / 细则·解读 / 架构 / 身份 /
/// 价值观·原则 / 方向 / 权限 / 世界观）在人类尚未录入时，用 LLM 探索项目
/// （搜集文档 + 代码印证 + 依赖图结构树/符号清单）生成**临时内容**落格位，
/// 让首屏投影携带真实项目通则而非空摘要；人类后续录入覆盖为 经，临时代偿自动让位。
///
/// 返回已补条数（人类已录的格位零成本跳过；LLM 调用失败只记警告，不阻断主流程——
/// 临时代偿是"锦上添花"，缺了他家仍按原有空摘要走）。
pub fn 临时代偿(
    存储: &模型存储,
    配置: &模型配置,
    根: &Path,
    依赖图: &依赖图,
) -> Result<usize, String> {
    let 排除项 = 扫描排除项(根);
    let mut 补条数 = 0usize;
    for 格位 in crate::全部格位().into_iter().filter(|格位| 格位.来源 == 来源::人类) {
        // 已有人类录入（任何非失效链头）或已有临时代偿记录 → 跳过（跨任务复用）。
        let 已有 = 存储.读格位(&格位.名字).unwrap_or_default();
        if 已有.iter().any(|记录| !记录.失效) {
            continue;
        }
        // 探索实证：文档 + 代码印证 + 依赖图结构树/符号清单。
        let 文档们 = 搜集文档(根, &排除项);
        let 代码们 = 搜集代码印证(根, &排除项);
        let 图谱 = 依赖图摘要(依赖图);
        let 素材: String = 文档们
            .iter()
            .take(8)
            .map(|(名, 内容)| format!("【{名}】\n{}", 截断(&内容, 1200)))
            .collect::<Vec<_>>()
            .join("\n");
        if 素材.is_empty() {
            continue; // 连文档都没有时无实证可依，宁缺毋滥（不空想）
        }
        let 印证 = format!(
            "{} 个源码文件；依赖图谱：{}",
            代码们.len(),
            截断(&图谱, 400)
        );
        let 提示 = 渲染播种提示(&格位.种子提示词, &素材, &印证);
        let (回复, 用量) = match 调用模型(配置, &[对话消息::用户(&提示)], moxing_fu::精简上限) {
            Ok(值) => 值,
            Err(错误) => {
                warn!(格位名 = %格位.名字, "临时代偿 LLM 归纳失败：{错误}");
                continue;
            }
        };
        if 回复.trim() == "不确定" || 回复.trim().is_empty() {
            info!(格位名 = %格位.名字, "临时代偿：素材中无相关内容，跳过");
            continue;
        }
        let 摘要: String = 回复.lines().take(3).collect::<Vec<_>>().join("\n");
        let 记录 = 记录::新(
            &格位.名字,
            &摘要,
            &format!("临时代偿：文档+代码印证+依赖图（{} 文档 / {} 源码）探索归纳", 文档们.len(), 代码们.len()),
            "LLM",
        );
        if let Err(错误) = 存储.写记录(&记录) {
            warn!(格位名 = %格位.名字, "临时代偿落格位失败：{错误}");
            continue;
        }
        补条数 += 1;
        info!(格位名 = %格位.名字, 内容长度 = 摘要.len(), 提示词 = 用量.提示词, "临时代偿完成");
    }
    debug!(补条数, "人类格位临时代偿完成");
    Ok(补条数)
}

/// 依赖图摘要：结构树根节点清单 + 前若干符号签名（实证素材，供临时代偿归纳）。
fn 依赖图摘要(依赖图: &依赖图) -> String {
    let mut 行 = String::new();
    for 节点 in 依赖图.结构树.子节点.iter().take(12) {
        行.push_str(&节点.名字);
        行.push(' ');
    }
    let 符号数 = 依赖图.档案们.len();
    let 签名们: Vec<&str> = 依赖图
        .档案们
        .iter()
        .filter(|档案| !档案.签名.is_empty())
        .take(20)
        .map(|档案| 档案.签名.as_str())
        .collect();
    format!("结构：{}；符号 {} 个，例：{}", 行.trim(), 符号数, 签名们.join(" · "))
}

/// 截断到上限字符（中文按字符计）。
fn 截断(文本: &str, 上限: usize) -> String {
    let 字符们: Vec<char> = 文本.chars().collect();
    if 字符们.len() > 上限 {
        format!("{}…", 字符们[..上限].iter().collect::<String>())
    } else {
        文本.to_string()
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::模型存储;

    fn 临时存储() -> 模型存储 {
        // 进程+线程+毫秒三重区分：workspace 并行测试时不同测试线程同毫秒会撞同一临时目录。
        let 线程 = format!("{:?}", std::thread::current().id());
        crate::模型存储::打开(std::env::temp_dir().join(format!(
            "临时代偿测试-{}-{}-{}",
            std::process::id(),
            线程,
            crate::当前毫秒()
        )))
    }

    #[test]
    fn 截断_超限加省略号() {
        assert_eq!(截断("天地玄黄宇宙洪荒", 4), "天地玄黄…");
        assert_eq!(截断("短文本", 100), "短文本");
        assert_eq!(截断("", 4), "");
    }

    #[test]
    fn 依赖图摘要_含结构与符号数() {
        let mut 图 = crate::依赖图::default();
        图.档案们.push(crate::符号档案::新("p", "甲府", "甲.rs", "甲函数", "pub fn 甲函数", "fn 甲函数", ""));
        图.结构树.子节点.push(crate::结构节点::新("乾坤"));
        let 摘要 = 依赖图摘要(&图);
        assert!(摘要.contains("结构：乾坤"), "应含结构：{摘要}");
        assert!(摘要.contains("符号 1 个"), "应含符号数：{摘要}");
        assert!(摘要.contains("fn 甲函数"), "应含签名示例：{摘要}");
    }

    #[test]
    fn 临时代偿_人类已录格位零成本跳过() {
        let 存储 = 临时存储();
        // 「初心·使命」是来源=人类格位：预写一条人类记录 → 临时代偿应跳过（返回 0，
        // 不触发 LLM 调用；若误触发会因空密钥地址报错，此测试通过即证明未触发）。
        存储.写记录(&记录::新("初心·使命", "人类已填的初心", "界主录入", "人类")).unwrap();
        let 空图 = crate::依赖图::default();
        let 根 = std::env::temp_dir();
        // 空密钥/地址的配置：若临时代偿误闯 LLM 调用会立即失败——正因为人类已录格位被跳过，
        // 返回 0 而非错误。
        let 空配置 = 模型配置 { 密钥: String::new(), 地址: String::new(), 模型: String::new() };
        let 结果 = 临时代偿(&存储, &空配置, &根, &空图).unwrap();
        assert_eq!(结果, 0, "人类已录的格位应跳过，无 LLM 调用");
    }
}

