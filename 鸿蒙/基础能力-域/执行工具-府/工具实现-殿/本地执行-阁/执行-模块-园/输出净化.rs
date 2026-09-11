//! 终端输出净化：剥离子进程命令输出中的 ANSI/VT 转义序列。
//!
//! 为什么需要：cargo/rustc 等工具会输出颜色与光标控制序列（如 `ESC[1m`、`ESC[8;..H`），
//! 这些序列进入天机流后显示为 `□[1m□[92m` 乱码（2026-09-11 真机截图实证）。
//! 治本在执行器启动命令时注入 `CARGO_TERM_COLOR=never`/`NO_COLOR`，本文件是输出边界兜底，
//! 双保险，且对不遵守禁色变量的工具有效。

/// 剥离终端转义序列（ANSI/VT），返回纯净文本。
///
/// 字节级扫描：转义序列全部由 ASCII（`<0x80`）构成，而 UTF-8 多字节字符首字节 `≥0xC0`、
/// 续字节 `≥0x80`，不会落入任何序列字节区间，故按字节跳过绝不会切断多字节字符。
pub fn 剥终端转义(输入: &str) -> String {
    let 字节 = 输入.as_bytes();
    let mut 出: Vec<u8> = Vec::with_capacity(字节.len());
    let mut i = 0;
    while i < 字节.len() {
        if 字节[i] != 0x1B {
            出.push(字节[i]);
            i += 1;
            continue;
        }
        // 末尾孤立 ESC：直接丢弃
        if i + 1 >= 字节.len() {
            break;
        }
        i = 跳过转义(字节, i);
    }
    // 非序列字节（含合法 UTF-8）原样保留，序列字节被剔除，故结果必为合法 UTF-8
    String::from_utf8(出).unwrap_or_default()
}

/// 从 ESC（位置 `起`）起，返回该转义序列之后的下一字节位置。
fn 跳过转义(字节: &[u8], 起: usize) -> usize {
    match 字节[起 + 1] {
        // CSI：ESC [ 参数(0x30-0x3F)* 中间(0x20-0x2F)* 终止(0x40-0x7E)
        b'[' => {
            let mut j = 起 + 2;
            while j < 字节.len() && (0x30..=0x3F).contains(&字节[j]) {
                j += 1;
            }
            while j < 字节.len() && (0x20..=0x2F).contains(&字节[j]) {
                j += 1;
            }
            if j < 字节.len() && (0x40..=0x7E).contains(&字节[j]) {
                j += 1;
            }
            j
        }
        // OSC：ESC ] …直到 BEL(0x07) 或 ST(ESC \)
        b']' => {
            let mut j = 起 + 2;
            while j < 字节.len() {
                if 字节[j] == 0x07 {
                    return j + 1;
                }
                if 字节[j] == 0x1B && j + 1 < 字节.len() && 字节[j + 1] == b'\\' {
                    return j + 2;
                }
                j += 1;
            }
            j
        }
        // 其它：ESC [中间(0x20-0x2F)*]? 终止(0x30-0x7E)，兼容单字节序列（ESC 7 / ESC c 等）
        引导 => {
            let mut j = 起 + 1;
            if (0x20..=0x2F).contains(&引导) {
                j += 1;
                while j < 字节.len() && (0x20..=0x2F).contains(&字节[j]) {
                    j += 1;
                }
            }
            if j < 字节.len() && (0x30..=0x7E).contains(&字节[j]) {
                j + 1
            } else {
                // 无法识别：至少跳过 ESC 与引导两字节，避免死循环
                起 + 2
            }
        }
    }
}

#[cfg(test)]
mod 测试 {
    use super::剥终端转义;

    #[test]
    fn 纯文本保持不变() {
        assert_eq!(剥终端转义("Finished dev profile"), "Finished dev profile");
    }

    #[test]
    fn 剥离颜色与样式序列() {
        // 真机样本：ESC[1mESC[92m FinishedESC[0m
        let 输入 = "\u{1b}[1m\u{1b}[92m Finished\u{1b}[0m";
        assert_eq!(剥终端转义(输入), " Finished");
    }

    #[test]
    fn 剥离光标定位序列() {
        // 真机样本：Running unittests 后用 ESC[8;42H 移动光标
        let 输入 = "Running\u{1b}[0m\u{1b}[8;42H ";
        assert_eq!(剥终端转义(输入), "Running ");
    }

    #[test]
    fn 剥离超链接osc序列() {
        let 输入 = "\u{1b}]8;;http://example.com\u{1b}\\链接\u{1b}]8;;\u{1b}\\";
        assert_eq!(剥终端转义(输入), "链接");
    }

    #[test]
    fn 中文与转义混合不切断多字节() {
        let 输入 = "编译\u{1b}[31m失败\u{1b}[0m了";
        assert_eq!(剥终端转义(输入), "编译失败了");
    }

    #[test]
    fn 结尾孤立转义被丢弃() {
        assert_eq!(剥终端转义("abc\u{1b}"), "abc");
    }

    #[test]
    fn 回车符予以保留() {
        // 本函数只剥转义序列，不处理 \r（行界拆分归展示侧摘要负责）
        assert_eq!(剥终端转义("a\rb"), "a\rb");
    }
}
