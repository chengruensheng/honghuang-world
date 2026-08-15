//! JSON 输出：AI 机器解析（骨架用最小转义，生产换 serde_json）

fn 转义(源: &str) -> String {
    let mut 出 = String::new();
    for 字符 in 源.chars() {
        match 字符 {
            '"' => 出.push_str("\\\""),
            '\\' => 出.push_str("\\\\"),
            '\n' => 出.push_str("\\n"),
            '\r' => 出.push_str("\\r"),
            '\t' => 出.push_str("\\t"),
            _ => 出.push(字符),
        }
    }
    出
}

pub fn 呈现JSON(内容: &str) -> String {
    format!("{{\"ok\":true,\"data\":\"{}\"}}", 转义(内容))
}
