//! 渲染组装：把叙述与校验结论拼成 Markdown，并生成索引页与校验报告。

use std::collections::BTreeMap;

use crate::{判定, 文档事实, 生成结果, 校验结论};

/// 坐标层级顺序（由粗到细）
const 层序: [&str; 6] = ["根", "域", "府", "殿", "阁", "园"];

/// 单篇文档后缀（产出契约的一部分）
const 文档后缀: &str = ".md";

/// 单篇文档的文件名：路径 → slug（`/` 换 `__`，去语言后缀，加 `.md`）
///
/// 文件名即溯源坐标：由它可反推回源文件。
pub fn 文档文件名(文件: &str) -> String {
    let 去后缀 = 文件.rsplit_once('.').map(|(前, _)| 前).unwrap_or(文件);
    format!("{}{}", 去后缀.replace('/', "__"), 文档后缀)
}

/// 渲染一篇文档
pub fn 渲染文档(事实: &文档事实, 正文: &str, 结论: &校验结论) -> String {
    let mut 文本 = String::new();
    写页首(&mut 文本, 事实);
    文本.push_str(正文);
    写页脚(&mut 文本, 结论);
    文本
}

/// 渲染索引页：按坐标根分组的篇目清单
pub fn 渲染索引(事实们: &[文档事实], 结果: &生成结果) -> String {
    let mut 文本 = String::new();
    文本.push_str("# 代码文档 · 索引\n\n");
    文本.push_str(&format!(
        "共 {} 篇 ｜ 符号 {} ｜ 边 {} ｜ 疑似幻觉 {} 篇 ｜ 自检 {}\n\n",
        结果.篇数,
        结果.符号数,
        结果.边数,
        结果.可疑篇数(),
        if 结果.自检.通过 { "通过" } else { "未通过" }
    ));

    let mut 分组: BTreeMap<String, Vec<&文档事实>> = BTreeMap::new();
    for 事实 in 事实们 {
        分组.entry(根名(事实)).or_default().push(事实);
    }
    for (根, 列表) in &分组 {
        文本.push_str(&format!("## {}（{} 篇）\n\n", 根, 列表.len()));
        for 事实 in 列表 {
            文本.push_str(&format!(
                "- [{}](篇/{})\n",
                事实.文件,
                文档文件名(&事实.文件)
            ));
        }
        文本.push('\n');
    }
    文本
}

/// 渲染校验报告
pub fn 渲染校验报告(结果: &生成结果) -> String {
    let mut 文本 = String::new();
    文本.push_str("# 代码文档 · 校验报告\n\n");
    文本.push_str("> 校验只针对**事实性**：叙述中出现的符号引用能否在事实图上溯源。\n");
    文本.push_str("> 文风与结论质量不在校验范围。\n\n");
    文本.push_str(&format!("- 生成篇数：{}\n", 结果.篇数));
    文本.push_str(&format!("- 叙述器：{}\n", 结果.叙述器));
    文本.push_str(&format!("- 叙述降级：{} 篇（叙述器失败后回落模板）\n", 结果.降级篇数));
    文本.push_str(&format!("- 校验通过：{} 篇\n", 结果.篇数 - 结果.可疑篇数()));
    文本.push_str(&format!("- 疑似幻觉：{} 篇\n\n", 结果.可疑篇数()));
    文本.push_str(&format!("## 自检\n\n{}\n\n", 结果.自检.说明));

    if 结果.可疑篇数() == 0 {
        文本.push_str("## 疑似幻觉清单\n\n无。\n");
        return 文本;
    }

    文本.push_str("## 疑似幻觉清单\n\n| 文件 | 无法溯源的引用 |\n|---|---|\n");
    for 结论 in &结果.校验 {
        if let 判定::疑似幻觉(项) = &结论.判定 {
            文本.push_str(&format!("| {} | {} |\n", 结论.文件, 项.join("、")));
        }
    }
    文本
}

fn 写页首(文本: &mut String, 事实: &文档事实) {
    文本.push_str(&format!("# {}\n\n", 事实.文件));
    文本.push_str(&format!("> **坐标**：{}\n", 坐标链(事实)));
    文本.push_str(&format!(
        "> **来源语言**：{} ｜ 定义 {} ｜ 出边 {} ｜ 入边 {} ｜ 悬空 {}\n",
        事实.来源语言,
        事实.定义.len(),
        事实.出边.len(),
        事实.入边.len(),
        事实.悬空.len()
    ));
    文本.push_str(
        "> 由「代码文档」府依符号索引生成；事实源：`.传承/图谱/知识图谱/符号索引.json`\n\n",
    );
}

fn 写页脚(文本: &mut String, 结论: &校验结论) {
    // 前置两个换行：正文（尤其模型产出）可能不以换行结尾，
    // 而「文本 + 单个换行 + ---」会被 Markdown 解析成 setext 标题。
    文本.push_str("\n\n---\n\n");
    match &结论.判定 {
        判定::通过 => {
            文本.push_str("> **事实校验**：通过（叙述中的符号引用均可溯源）\n");
        }
        判定::疑似幻觉(项) => {
            文本.push_str(&format!(
                "> **事实校验**：疑似幻觉 {} 条 —— {}\n",
                项.len(),
                项.join("、")
            ));
        }
    }
}

/// 坐标链：按 根/域/府/殿/阁/园 顺序拼接
fn 坐标链(事实: &文档事实) -> String {
    let 段: Vec<String> = 层序
        .iter()
        .filter_map(|层| 事实.坐标.get(*层))
        .filter(|值| !值.is_empty())
        .cloned()
        .collect();
    if 段.is_empty() {
        "（未 join 到坐标）".to_string()
    } else {
        段.join(" / ")
    }
}

fn 根名(事实: &文档事实) -> String {
    事实
        .坐标
        .get("根")
        .filter(|值| !值.is_empty())
        .cloned()
        .unwrap_or_else(|| "未归类".to_string())
}
