//! 符号 - 解析：从源码行提取符号签名 / 上方注释 / 定义体 / use 引用。

/// 提取符号签名：pub fn / pub struct / pub trait / pub enum / pub type。
/// 返回（符号名, 签名）。
pub(crate) fn 提取符号签名(行: &str) -> Option<(String, String)> {
    let 行 = 行.trim();
    let 余 = 行.strip_prefix("pub ")?;
    let 余 = 余.strip_prefix("async ").unwrap_or(余);
    if let Some(余) = 余.strip_prefix("fn ") {
        let 名: String = 余
            .chars()
            .take_while(|字符| *字符 != '(' && !字符.is_whitespace())
            .collect();
        if 名.is_empty() {
            return None;
        }
        return Some((名.clone(), format!("fn {名}")));
    }
    for 关键字 in ["struct", "trait", "enum", "type"] {
        if let Some(余) = 余.strip_prefix(关键字) {
            if 余.starts_with(' ') {
                let 名: String = 余
                    .trim_start()
                    .chars()
                    .take_while(|字符| {
                        !字符.is_whitespace() && *字符 != '{' && *字符 != ';' && *字符 != '<'
                    })
                    .collect();
                if !名.is_empty() {
                    return Some((名.clone(), format!("{关键字} {名}")));
                }
            }
        }
    }
    None
}

/// 提取符号定义上方的 /// 注释（向上最多 8 行，跳过 // 注释与 #[属性] 行，连续）。
pub(crate) fn 提取上方注释(行们: &[&str], 序号: usize) -> String {
    let mut 注释们 = Vec::new();
    let 下界 = 序号.saturating_sub(8);
    let mut 游标 = 序号;
    while 游标 > 下界 {
        游标 -= 1;
        let 行 = 行们[游标].trim();
        if let Some(注释) = 行.strip_prefix("///") {
            注释们.push(注释.trim().to_string());
        } else if 行.starts_with("//") || 行.starts_with("#[") {
            // 跳过普通注释与属性行（#[derive]/#[serde] 等），继续向上找 /// 注释
            continue;
        } else {
            break;
        }
    }
    注释们.reverse();
    注释们.join(" ")
}

/// 提取符号的完整定义体：从符号起始行起，花括号配平取整块（fn / trait / enum / struct）；
/// 无花括号的类型别名 / 单元结构取到行尾分号。跳过字符串字面量与 // 注释，防误配平。
pub(crate) fn 提取定义块(行们: &[&str], 起始行: usize) -> String {
    let mut 深度 = 0i32;
    let mut 找到花括号 = false;
    let mut 结束行 = 起始行;
    for (偏移, 行) in 行们.iter().enumerate().skip(起始行) {
        let mut 在字符串 = false;
        let mut 转义 = false;
        let mut 字符们 = 行.chars().peekable();
        while let Some(字符) = 字符们.next() {
            if 在字符串 {
                if 转义 {
                    转义 = false;
                } else if 字符 == '\\' {
                    转义 = true;
                } else if 字符 == '"' {
                    在字符串 = false;
                }
                continue;
            }
            if 字符 == '"' {
                在字符串 = true;
                continue;
            }
            if 字符 == '/' && 字符们.peek() == Some(&'/') {
                break; // 注释，本行代码到此为止
            }
            match 字符 {
                '{' => {
                    深度 += 1;
                    找到花括号 = true;
                }
                '}' => {
                    深度 -= 1;
                    if 找到花括号 && 深度 <= 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        if 找到花括号 && 深度 <= 0 {
            结束行 = 偏移;
            break;
        }
        if !找到花括号 && 行.trim_end().ends_with(';') {
            结束行 = 偏移;
            break;
        }
        结束行 = 偏移;
    }
    行们[起始行..=结束行].join("\n")
}

/// 提取 use / pub use 引用的符号名（含花括号批量、别名、单路径）。
pub(crate) fn 提取use引用(行: &str) -> Vec<String> {
    let 行 = 行.trim();
    let 内容 = if let Some(余) = 行.strip_prefix("pub use ") {
        余
    } else if let Some(余) = 行.strip_prefix("use ") {
        余
    } else {
        return Vec::new();
    };
    let 内容 = 内容.trim_end_matches(';').trim();
    let mut 引用们 = Vec::new();
    if let (Some(开), Some(闭)) = (内容.find('{'), 内容.find('}')) {
        let 内部 = &内容[开 + 1..闭];
        for 项 in 内部.split(',') {
            let 名 = 项.trim().split(" as ").next().unwrap_or("").trim();
            if let Some(末段) = 名.split("::").last() {
                if !末段.is_empty() && 末段 != "*" {
                    引用们.push(末段.to_string());
                }
            }
        }
    } else {
        let 名 = 内容.split(" as ").next().unwrap_or("").trim();
        if let Some(末段) = 名.split("::").last() {
            if !末段.is_empty() && 末段 != "*" {
                引用们.push(末段.to_string());
            }
        }
    }
    引用们
}
