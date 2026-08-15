//! 播种 - 归纳 - 园：按种子降级链归纳语义记录。

use crate::{格位, 记录, 模型存储, 扫描排除项};
use moxing_fu::{调用模型, 对话消息, 模型配置};
use std::path::Path;

/// 播种结果：已归纳（返回内容）或 需人类（返回原因）。
pub enum 播种结果 {
    已归纳(String),
    需人类(String),
}

/// 直调模型归纳一条记录（来源 = LLM，必带证据）。
pub fn 播种归纳(存储: &模型存储, 格位: &格位, 配置: &模型配置, 背景: &str) -> Result<String, String> {
    let 提示 = format!(
        "你按这句本质去项目里找内容，归纳成一条简短记录，并给出证据（从哪个文件或符号推断）：{}\n背景：{}",
        格位.种子提示词, 背景
    );
    let 回复 = 调用模型(配置, &[对话消息::用户(提示)])?;
    let 记录 = 记录::新(&格位.名字, &回复, &format!("按种子「{}」归纳", 格位.种子提示词), "LLM");
    存储.写记录(&记录)?;
    Ok(回复)
}

/// 按种子降级链播种：推荐位置文件 → 代码兜底 → 人类。
/// 有推荐位置文件则读其内容交模型归纳；无推荐位置或文件不存在则回落人类。
pub fn 播种降级(存储: &模型存储, 格位: &格位, 配置: &模型配置, 根: &Path) -> Result<播种结果, String> {
    if !格位.推荐位置.is_empty() {
        let 候选 = 根.join(&格位.推荐位置);
        if 候选.is_file() {
            let 素材 = std::fs::read_to_string(&候选).map_err(|错误| format!("读推荐位置失败: {错误}"))?;
            let 提示 = format!(
                "你按这句本质去项目里找内容，归纳成一条简短记录，并给出证据（从哪个文件或符号推断）：{}\n素材：{}",
                格位.种子提示词, 素材
            );
            let 回复 = 调用模型(配置, &[对话消息::用户(提示)])?;
            let 摘要: String = 回复.lines().take(3).collect::<Vec<_>>().join("\n");
            let 记录 = 记录::新(&格位.名字, &摘要, &format!("从「{}」归纳", 格位.推荐位置), "LLM");
            存储.写记录(&记录)?;
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

/// 纯LLM播种：先搜集文档 + 代码印证，交模型归纳，齐全则审阅录入；
/// 无文档则代码为主 + 问界主，直到拿到内容。
pub fn 纯LLM播种(
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
        let 提示 = format!(
            "按这句本质归纳成一条简短记录，并给出证据：{}\n文档素材：\n{}\n代码印证（文件清单）：{}",
            格位.种子提示词, 素材, 印证
        );
        let 回复 = 调用模型(配置, &[对话消息::用户(提示)])?;
        let 摘要: String = 回复.lines().take(3).collect::<Vec<_>>().join("\n");
        let 证据 = format!("文档 + 代码印证（{} 个文档）", 文档们.len());
        if 界主.审阅(&格位.名字, &摘要, &证据) {
            let 记录 = 记录::新(&格位.名字, &摘要, &证据, "LLM");
            存储.写记录(&记录)?;
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

/// 纯LLM播种全部 36 格位，返回已归纳条数。
pub fn 纯LLM播种全部(
    存储: &模型存储,
    配置: &模型配置,
    根: &Path,
    界主: &dyn 界主交互,
) -> Result<usize, String> {
    let mut 条数 = 0;
    for 格位 in crate::全部格位() {
        if matches!(纯LLM播种(存储, &格位, 配置, 根, 界主)?, 播种结果::已归纳(_)) {
            条数 += 1;
        }
    }
    Ok(条数)
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::{固化度, 共享度, 顺序档位, 范畴};

    fn 造格位(推荐位置: &str) -> 格位 {
        let mut 格位 = 格位::新("架构", 范畴::规则, "技术架构原则", "人类", 固化度::经, 共享度::共享, 顺序档位::最前);
        格位.推荐位置 = 推荐位置.to_string();
        格位
    }

    #[test]
    fn 无推荐位置且无源文件则回落人类() {
        let 根 = std::env::temp_dir().join("识海测试-播种-空目录");
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();

        let 格位 = 造格位("");
        let 配置 = 模型配置 { 密钥: "k".into(), 地址: "未用".into(), 模型: "m".into() };
        let 存储 = 模型存储::打开(&根);
        match 播种降级(&存储, &格位, &配置, &根).unwrap() {
            播种结果::需人类(_) => {}
            播种结果::已归纳(_) => assert!(false),
        }

        std::fs::remove_dir_all(&根).ok();
    }

    struct 假界主;
    impl 界主交互 for 假界主 {
        fn 审阅(&self, _格位名: &str, _归纳: &str, _证据: &str) -> bool {
            true
        }
        fn 询问(&self, 格位名: &str, _问题: &str) -> String {
            format!("{格位名} 的回答")
        }
    }

    #[test]
    fn 纯LLM播种无文档则代码为主() {
        let 根 = std::env::temp_dir().join("识海测试-纯LLM-无文档");
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();
        std::fs::write(根.join("样例.rs"), "fn 主函数() {}\n").unwrap();

        let 格位 = 造格位("");
        let 配置 = 模型配置 { 密钥: "k".into(), 地址: "未用".into(), 模型: "m".into() };
        let 存储 = 模型存储::打开(&根);
        match 纯LLM播种(&存储, &格位, &配置, &根, &假界主).unwrap() {
            播种结果::已归纳(摘要) => {
                assert!(摘要.contains("fn 主函数"));
                assert_eq!(存储.读格位(&格位.名字).unwrap().len(), 1);
            }
            播种结果::需人类(_) => assert!(false),
        }

        std::fs::remove_dir_all(&根).ok();
    }
}
