//! 文本输出：人类可读

use rizhi_fu::debug;

pub fn 呈现文本(内容: &str) -> String {
    debug!(长度 = 内容.len(), "文本已呈现");
    内容.to_string()
}
