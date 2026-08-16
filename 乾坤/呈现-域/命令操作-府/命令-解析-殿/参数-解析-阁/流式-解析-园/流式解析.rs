//! 参数解析：argv → 调用

use rizhi_fu::debug;

/// 一次命令调用
#[derive(Clone, Debug)]
pub struct 调用 {
    pub 域: String,
    pub 动作: String,
    pub 参数: Vec<String>,
    pub 旗标: Vec<(String, String)>,
    pub 要JSON: bool,
}

impl 调用 {
    pub fn 空() -> 调用 {
        调用 {
            域: String::new(),
            动作: String::new(),
            参数: Vec::new(),
            旗标: Vec::new(),
            要JSON: false,
        }
    }
}

/// 解析 argv（跳过程序名）→ 调用
pub fn 解析调用(输入: Vec<String>) -> 调用 {
    let mut 调用 = 调用::空();
    let mut 段位 = 0usize; // 0=域 1=动作 2+=参数
    let mut 迭代 = 输入.into_iter();
    while let Some(词) = 迭代.next() {
        match 词.as_str() {
            "--json" => 调用.要JSON = true,
            "--全文" => 调用.旗标.push(("全文".to_string(), "true".to_string())),
            "-t" | "--令牌" => 调用.旗标.push(("令牌".to_string(), 迭代.next().unwrap_or_default())),
            "-f" | "--文件" => 调用.旗标.push(("文件".to_string(), 迭代.next().unwrap_or_default())),
            "-意见" | "--意见" => 调用.旗标.push(("意见".to_string(), 迭代.next().unwrap_or_default())),
            _ => {
                if 段位 == 0 {
                    调用.域 = 词;
                    段位 = 1;
                } else if 段位 == 1 {
                    调用.动作 = 词;
                    段位 = 2;
                } else {
                    调用.参数.push(词);
                }
            }
        }
    }
    debug!(域 = %调用.域, 动作 = %调用.动作, 参数数 = 调用.参数.len(), "命令已解析");
    调用
}
