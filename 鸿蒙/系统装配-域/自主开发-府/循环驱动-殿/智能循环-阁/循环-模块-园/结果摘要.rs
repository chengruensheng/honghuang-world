//! 命令类工具结果的事件摘要：结论在尾，保尾呈现。
//!
//! 通用截断只留头部，会把 CLI 的验收结论切掉——真机实测 `cargo test` 的
//! `test result: ok. N passed; 0 failed` 在最尾，被 200 字符帽整段切掉，
//! 界面无法自证验收是否通过。命令输出的关键结论几乎总在尾部
//! （`test result:` / `Finished` / `error[E..]` / `warning:`），故超长时
//! 「保首行动作 + 省略中段 + 保尾结论」。

/// 省略中段的连接标记
const 中段标记: &str = "…（中段省略）…";

/// 结论行强度：0=非结论；1=弱结论（成功/提示，仅兜底）；2=强结论（成败真因，必抓）。
///
/// 分级动机（2026-09-11 真机）：`cargo build` 失败时，真正的 `error[E0583]` 在前，
/// 而输出最尾一行是另一 crate 的 `warning`——若遇任意结论即止，会停在 warning，
/// 把失败真因 error 省略掉。故从尾向头扫描时，弱结论不停、继续找强结论。
fn 结论强度(行: &str) -> u8 {
    let 小写 = 行.to_lowercase();
    const 强: &[&str] = &[
        "test result",
        "error",
        "could not compile",
        "aborting",
        "cannot find",
        "failed to",
        "compilation failed",
    ];
    const 弱: &[&str] = &["finished", "warning", "passed", "failed"];
    if 强.iter().any(|k| 小写.contains(k)) {
        2
    } else if 弱.iter().any(|k| 小写.contains(k)) {
        1
    } else {
        0
    }
}

/// 为命令输出生成有界摘要：未超上限原样返回；超长则保首行动作、保尾结论行。
///
/// - 行界同时认 `\n` 与 `\r`（cargo 进度条用 `\r` 刷新，剥离 ANSI 后可能残留）；
/// - 长度一律按 `char` 计，中文不被截半；
/// - 尾段优先锁定强结论行（error/test result/…）；尾行是弱结论（warning）时不停、
///   继续向头抓强结论；无任何结论时退化为取尾部若干行；
/// - 返回长度为软上限并经最终硬上限兜底，用于事件展示，非硬协议边界。
pub fn 摘要命令输出(文本: &str, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        return 文本.to_string();
    }
    let 行: Vec<&str> = 文本
        .split(|c| c == '\n' || c == '\r')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if 行.is_empty() {
        return 文本.chars().take(上限).collect::<String>() + "…";
    }

    let 可用 = 上限.saturating_sub(中段标记.chars().count()).max(1);
    // 首部「在做什么」给 1/4，尾部「结论」给 3/4
    let 首预算 = 可用 / 4;
    let 尾预算 = 可用 - 首预算;

    // 从尾向前选行（受尾预算约束）：
    // - 遇强结论（error/test result/…成败真因）立即停；
    // - 遇弱结论（warning/finished/…）纳入但不停，继续向头找强结论（真机：尾行 warning 会掩盖真因 error）；
    // - 预算用尽则停（物理上限，全文仍在后端日志可查）。
    let mut 尾选: Vec<usize> = Vec::new();
    let mut 已用 = 0usize;
    for 索引 in (0..行.len()).rev() {
        let 行长 = 行[索引].chars().count() + 1;
        if !尾选.is_empty() && 已用 + 行长 > 尾预算 {
            break;
        }
        尾选.push(索引);
        已用 += 行长;
        if 结论强度(行[索引]) == 2 {
            break;
        }
        if 已用 >= 尾预算 {
            break;
        }
    }
    尾选.reverse();

    // 每行硬截断到尾预算，防单行超长撑爆
    let 尾段 = 尾选
        .iter()
        .map(|&i| 截字符(行[i], 尾预算))
        .collect::<Vec<_>>()
        .join("\n");

    let mut 结果 = if 尾选.contains(&0) {
        尾段
    } else {
        format!("{}\n{中段标记}\n{尾段}", 截字符(行[0], 首预算))
    };

    // 最终硬上限兜底：仍超则保尾截断（结论在尾），前缀省略号
    if 结果.chars().count() > 上限 {
        let 取 = 上限.saturating_sub(1).max(1);
        let 尾: String = 结果.chars().rev().take(取).collect::<Vec<_>>().into_iter().rev().collect();
        结果 = format!("…{尾}");
    }
    结果
}

/// 按字符数截断单行，超长以省略号结尾
fn 截字符(行: &str, 上限: usize) -> String {
    if 行.chars().count() <= 上限 {
        行.to_string()
    } else {
        行.chars().take(上限).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod 测试 {
    use super::摘要命令输出;

    #[test]
    fn 未超上限原样返回() {
        assert_eq!(摘要命令输出("hello\nworld", 200), "hello\nworld");
    }

    #[test]
    fn 超长时只保首行动作与尾部结论() {
        let mut 文本 = String::new();
        for i in 0..50 {
            文本.push_str(&format!("Compiling crate_{i}\n"));
        }
        文本.push_str("test result: ok. 29 passed; 0 failed; 0 ignored\n");
        let 摘要 = 摘要命令输出(&文本, 200);
        assert!(摘要.contains("test result: ok. 29 passed"), "摘要须含尾部结论: {摘要}");
        assert!(摘要.contains("Compiling crate_0"), "摘要须含首行动作: {摘要}");
        assert!(摘要.contains("中段省略"), "摘要须含省略标记: {摘要}");
        assert!(!摘要.contains("Compiling crate_49"), "结论行已拿到，中段噪声须省略: {摘要}");
    }

    #[test]
    fn 单行超长也能在预算内返回() {
        let 文本 = "a".repeat(1000);
        let 摘要 = 摘要命令输出(&文本, 100);
        assert!(摘要.chars().count() <= 101, "单行超长须有界: {}", 摘要.chars().count());
        assert!(摘要.ends_with('…'));
    }

    #[test]
    fn 回车换行都认作行界且锁定结论() {
        let mut 文本 = String::from("首行动作\r");
        for _ in 0..60 {
            文本.push_str("噪声\r");
        }
        文本.push_str("test result: ok. 3 passed\r");
        let 摘要 = 摘要命令输出(&文本, 200);
        assert!(摘要.contains("test result: ok. 3 passed"), "回车分隔的尾结论须保留: {摘要}");
        assert!(!摘要.contains("噪声"), "结论行已拿到，噪声须省略: {摘要}");
    }

    #[test]
    fn 短空白原样返回不截断() {
        assert_eq!(摘要命令输出("\n\r  \n", 50), "\n\r  \n");
    }

    #[test]
    fn 中文按字符不截半() {
        let 文本 = "编译".repeat(200);
        let 摘要 = 摘要命令输出(&文本, 40);
        assert!(摘要.contains("编译"));
        assert!(摘要.ends_with('…'));
        assert!(摘要.chars().count() <= 41);
    }

    #[test]
    fn 构建失败尾行error被锁定() {
        let 文本 = format!("{}\nerror[E0425]: cannot find value `x`\n", "Compiling a\n".repeat(40));
        let 摘要 = 摘要命令输出(&文本, 200);
        assert!(摘要.contains("error[E0425]"), "错误结论须保留: {摘要}");
    }

    #[test]
    fn 尾行是warning时仍向头抓到error真因() {
        // 真机场景（2026-09-11 #14）：cargo build 失败，真因 error[E0583] 在前，
        // 输出最尾一行是另一 crate 的 warning——弱结论不得掩盖强结论。
        let mut 文本 = String::new();
        for _ in 0..40 {
            文本.push_str("Compiling a\n");
        }
        文本.push_str("error[E0583]: file not found for module `table`\n");
        for _ in 0..5 {
            文本.push_str("zz\n");
        }
        文本.push_str("warning: `session-summary` (lib) generated 1 warning");
        let 摘要 = 摘要命令输出(&文本, 200);
        assert!(摘要.contains("error[E0583]"), "必须跨 warning 抓到失败真因 error: {摘要}");
        assert!(摘要.contains("warning"), "尾行弱结论也应保留: {摘要}");
    }
}
