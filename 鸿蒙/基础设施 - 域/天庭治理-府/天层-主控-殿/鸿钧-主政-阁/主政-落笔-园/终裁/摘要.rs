//! 终裁·摘要：产物内容摘要、原文摘录、项目结构摘要。
//!
//! 给准圣/鸿钧提示词注入机械提取的真实文件事实（pub 符号、测试函数名、文档骨架、
//! 原文摘录、依赖图结构树、workspace members），治「凭字节数/符号摘要推断实现细节」。

use crate::类型_定义_殿::*;

/// 产物内容摘要：对 .rs 产物机械提取 pub 符号签名与测试函数名；对 .md/.json 等非 .rs 文本产物
/// 提取结构骨架（§14.16 修复：治「准圣凭字节数审验」——文档/配置类任务验收无内容可核对）。
/// 单文件符号上限 20 条、总摘要上限 2000 字符，防提示词膨胀。
pub(super) fn 产物内容摘要(产物们: &[产物条目]) -> String {
    const 单文件符号上限: usize = 20;
    const 总摘要上限: usize = 2_000;
    // §14.15 配置化：合法产物扩展名从装配配置读（数据驱动）；.rs 走源码提取，其余走骨架提取。
    let 文本扩展名们 = peizhi_fu::读装配().合法产物扩展名;
    let 根 = shihai_fu::工作区::定位();
    let mut 摘要们 = Vec::new();
    for 产物 in 产物们 {
        let Ok(内容) = std::fs::read_to_string(根.根路径().join(&产物.路径)) else {
            continue;
        };
        let 总行数 = 内容.lines().count();
        // 非 .rs 文本产物：结构骨架提取（.md 标题 / .json 顶层键 / 其余截断首 500）。
        if !产物.路径.ends_with(".rs") {
            if !文本扩展名们
                .iter()
                .any(|扩展名| 产物.路径.ends_with(扩展名))
            {
                continue; // 未知扩展名（二进制等）：跳过
            }
            let 骨架 = 文本骨架(&产物.路径, &内容);
            if 骨架.is_empty() {
                continue;
            }
            摘要们.push(format!(
                "- {}（{}行）\n    骨架：{}",
                产物.路径, 总行数, 骨架
            ));
            continue;
        }
        let 行们: Vec<&str> = 内容.lines().collect();
        let mut 符号们 = Vec::new();
        let mut 测试们 = Vec::new();
        let mut 索引 = 0usize;
        while 索引 < 行们.len() {
            let 行 = 行们[索引].trim();
            if 行.starts_with("#[test]") {
                // 下一行取 fn 名
                if let Some(下一行) = 行们.get(索引 + 1) {
                    let 名 = 下一行
                        .trim()
                        .trim_start_matches("fn ")
                        .split('(')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !名.is_empty() {
                        测试们.push(名);
                    }
                }
                索引 += 1;
                continue;
            }
            if 行.starts_with("pub fn")
                || 行.starts_with("pub struct")
                || 行.starts_with("pub enum")
                || 行.starts_with("pub trait")
                || 行.starts_with("pub const")
                || 行.starts_with("pub type")
                || 行.starts_with("pub use")
            {
                let 签名 = 行.split(" {").next().unwrap_or(行).to_string();
                符号们.push(签名);
            }
            索引 += 1;
        }
        if 符号们.is_empty() && 测试们.is_empty() {
            摘要们.push(format!(
                "- {}（{}行，无 pub 符号与测试）",
                产物.路径, 总行数
            ));
            continue;
        }
        let 符号段 = 符号们
            .iter()
            .take(单文件符号上限)
            .cloned()
            .collect::<Vec<_>>()
            .join("；");
        let 测试段 = 测试们
            .iter()
            .take(单文件符号上限)
            .cloned()
            .collect::<Vec<_>>()
            .join("、");
        let mut 条目 = format!("- {}（{}行）\n    符号：{}", 产物.路径, 总行数, 符号段);
        if !测试段.is_empty() {
            条目.push_str(&format!("\n    测试：{}", 测试段));
        }
        摘要们.push(条目);
    }
    if 摘要们.is_empty() {
        return "（无内容可提取的产物）".to_string();
    }
    let mut 合并 = 摘要们.join("\n");
    if 合并.chars().count() > 总摘要上限 {
        合并 = 合并.chars().take(总摘要上限).collect::<String>();
        合并.push_str("…（摘要截断，按需 读文件 工具回读）");
    }
    合并
}

/// 非 .rs 文本产物的结构骨架（§14.16）：
/// - .md：标题行（`#` 开头章节名，上限 10 条）；
/// - .json：顶层键名（上限 10 个）；
/// - 其余文本（.toml/.yaml/.yml/.txt）：截断首 500 字符。
fn 文本骨架(路径: &str, 内容: &str) -> String {
    if 路径.ends_with(".md") {
        // §14.16 修正：取所有 `#` 开头行（主标题+##章节+###小节，上限 10 条）——
        // 初版过滤 ## 导致骨架只剩主标题，准圣仍"凭字节猜"（实测装配-配置-说明.md）。
        let 标题们: Vec<&str> = 内容
            .lines()
            .map(|行| 行.trim())
            .filter(|行| 行.starts_with('#'))
            .take(10)
            .collect();
        if !标题们.is_empty() {
            return format!("章节：{}", 标题们.join(" / "));
        }
    } else if 路径.ends_with(".json") {
        if let Ok(值) = serde_json::from_str::<serde_json::Value>(内容) {
            if let Some(对象) = 值.as_object() {
                let 键们: Vec<String> = 对象.keys().take(10).cloned().collect();
                if !键们.is_empty() {
                    return format!("顶层键：{}", 键们.join("、"));
                }
            }
        }
    }
    // 其余文本或 md/json 无结构 → 截断首 500 字符。
    内容.chars().take(500).collect::<String>()
}

/// 产物原文摘录（§14.19 复验缺陷 12 修复）：把产物真实文件内容（预算内截断）注入准圣提示词，
/// 治「准圣凭字节数/符号摘要推断实现细节」——准圣没有读文件工具，此前只能看
/// "187 字节增量与一行注释体量吻合" 这类统计推断（实测准圣 think 原文实证）。
/// 单文件上限 1200 字符、总上限 3000 字符；只摘录文本产物（.rs/.md/.json/.toml/.yaml/.yml/.txt）。
pub(super) fn 产物原文摘录(产物们: &[产物条目]) -> String {
    const 单文件上限: usize = 1_200;
    const 总上限: usize = 3_000;
    let 文本扩展名们 = peizhi_fu::读装配().合法产物扩展名;
    let 根 = shihai_fu::工作区::定位();
    let mut 摘录们 = Vec::new();
    let mut 已用 = 0usize;
    for 产物 in 产物们 {
        if 已用 >= 总上限 {
            break;
        }
        // 只摘录文本产物（与 产物内容摘要 同口径）；未知扩展名（二进制等）跳过。
        let 是文本 = 产物.路径.ends_with(".rs")
            || 文本扩展名们
                .iter()
                .any(|扩展名| 产物.路径.ends_with(扩展名));
        if !是文本 {
            continue;
        }
        let Ok(内容) = std::fs::read_to_string(根.根路径().join(&产物.路径)) else {
            continue;
        };
        let 总行数 = 内容.lines().count();
        // 预算内截断：首 800 字符 + 尾部 300 字符（改动通常在文件任一位置，头尾覆盖率高；
        // 中间省略标注行数，准圣按需在产物摘要与清单间交叉核对）。
        let 字符们: Vec<char> = 内容.chars().collect();
        let 摘录 = if 字符们.len() <= 单文件上限 {
            内容.clone()
        } else {
            let 头: String = 字符们[..800].iter().collect();
            let 尾: String = 字符们[字符们.len() - 300..].iter().collect();
            format!(
                "{头}\n……（中间省略 {} 字符，共 {} 行）\n……尾部：{尾}",
                字符们.len() - 1_100,
                总行数
            )
        };
        let 条目 = format!("【{p}】（{行}行）\n{摘录}", p = 产物.路径, 行 = 总行数);
        已用 += 条目.chars().count();
        摘录们.push(条目);
    }
    if 摘录们.is_empty() {
        return "（无文本产物可摘录）".to_string();
    }
    let mut 合并 = 摘录们.join("\n\n");
    if 合并.chars().count() > 总上限 {
        合并 = 合并.chars().take(总上限).collect::<String>();
        合并.push_str("…（原文摘录截断）");
    }
    合并
}

/// 项目结构摘要（问题15）：读依赖图结构树 + workspace members，渲染为文本注入准圣/鸿钧提示词。
///
/// 治「准圣/鸿钧不知项目全貌、凭局部产物推断架构」——注入项目结构树与 workspace 成员清单，
/// 让审验方从整体结构判断产物归属合理性、模块接入是否符合六层落点（维度/域/府/殿/阁/园）。
///
/// 两段：
/// - 【结构树】：从 `shihai_fu::依赖图::加载自工作区` 读结构树，递归渲染为缩进文本（每层 2 空格）。
/// - 【workspace members】：从根 Cargo.toml 读 workspace members（含 `-府"` 的行即成员，
///   适配单行/多行 members，与 建档.rs 同口径）。
///
/// 预算：总上限 2000 字符，超则截断标注。读失败/无结构返回占位（不阻断审验）。
pub(super) fn 项目结构摘要() -> String {
    const 总上限: usize = 2_000;
    let 工作区 = shihai_fu::工作区::定位();

    // 结构树：从依赖图读，递归渲染为缩进文本。
    let 结构树段 = match shihai_fu::依赖图::加载自工作区(&工作区) {
        Ok(图) => 渲染结构树(&图.结构树),
        Err(_) => "（无结构树）".to_string(),
    };

    // workspace members：经识海承载-府 workspace 成员缓存读取（含 `-府` 的成员，与 建档.rs 同口径）。
    let members段 = match shihai_fu::读workspace成员缓存在(&工作区) {
        Some(摘要) if !摘要.成员们.is_empty() => 摘要.成员们.join("、"),
        Some(_) => "（无 workspace members）".to_string(),
        None => "（读 Cargo.toml 失败）".to_string(),
    };

    let 合并 = format!(
        "【结构树】\n{树}\n【workspace members】\n{members}",
        树 = 结构树段,
        members = members段
    );
    if 合并.chars().count() > 总上限 {
        let mut 截断: String = 合并.chars().take(总上限).collect();
        截断.push_str("…（项目结构摘要截断）");
        return 截断;
    }
    合并
}

/// 递归渲染结构节点为缩进文本（每层 2 空格，根节点不缩进）。
pub(super) fn 渲染结构树(节点: &shihai_fu::结构节点) -> String {
    let mut 输出 = String::new();
    渲染结构树_递归(节点, 0, &mut 输出);
    if 输出.is_empty() {
        return "（空结构树）".to_string();
    }
    输出
}

/// 结构树递归渲染内部函数：深度控制缩进，子节点逐级下探。
fn 渲染结构树_递归(节点: &shihai_fu::结构节点, 深度: usize, 输出: &mut String) {
    if !节点.名字.is_empty() {
        for _ in 0..深度 {
            输出.push_str("  ");
        }
        输出.push_str(&节点.名字);
        输出.push('\n');
    }
    for 子 in &节点.子节点 {
        渲染结构树_递归(子, 深度 + 1, 输出);
    }
}
