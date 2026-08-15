//! 读取 - 落配 - 园：从环境文件读取密钥，落成模型配置。

use crate::类型_定义_殿::模型配置;
use std::collections::HashMap;

/// 从 .env 环境文件读取模型配置。
pub fn 读模型配置(环境文件路径: &str) -> 模型配置 {
    let 映射 = 解析环境文件(环境文件路径);
    模型配置 {
        密钥: 映射.get("LLM_API_KEY").cloned().unwrap_or_default(),
        地址: 映射.get("LLM_BASE_URL").cloned().unwrap_or_default(),
        模型: 映射.get("LLM_MODEL").cloned().unwrap_or_default(),
    }
}

/// 解析 .env 的 `键=值` 行，跳过空行与注释。
fn 解析环境文件(路径: &str) -> HashMap<String, String> {
    let mut 映射 = HashMap::new();
    if let Ok(内容) = std::fs::read_to_string(路径) {
        for 行 in 内容.lines() {
            let 行 = 行.trim();
            if 行.is_empty() || 行.starts_with('#') {
                continue;
            }
            if let Some((键, 值)) = 行.split_once('=') {
                映射.insert(键.trim().to_string(), 值.trim().to_string());
            }
        }
    }
    映射
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 解析键值行() {
        let 临时 = std::env::temp_dir().join("识海测试.env");
        std::fs::write(&临时, "LLM_API_KEY=abc\nLLM_MODEL=MiniMax-M3\n# 注释\n").unwrap();
        let 映射 = 解析环境文件(临时.to_str().unwrap());
        assert_eq!(映射.get("LLM_API_KEY").unwrap(), "abc");
        assert_eq!(映射.get("LLM_MODEL").unwrap(), "MiniMax-M3");
        let _ = std::fs::remove_file(&临时);
    }
}
