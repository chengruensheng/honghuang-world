//! 读取 - 落配 - 园：从环境文件读取密钥，落成模型配置。

use crate::类型_定义_殿::模型配置;
use rizhi_fu::{debug, info, warn};
use std::collections::HashMap;

/// 从 .env 环境文件读取模型配置。
pub fn 读模型配置(环境文件路径: &str) -> 模型配置 {
    let 映射 = 解析环境文件(环境文件路径);
    let 配置 = 模型配置 {
        密钥: 映射.get("LLM_API_KEY").cloned().unwrap_or_default(),
        地址: 映射.get("LLM_BASE_URL").cloned().unwrap_or_default(),
        模型: 映射.get("LLM_MODEL").cloned().unwrap_or_default(),
    };
    debug!(环境文件路径, "模型配置已读取");
    if 配置.密钥.is_empty() || 配置.地址.is_empty() || 配置.模型.is_empty() {
        warn!(
            环境文件路径,
            "模型配置字段缺失（密钥/地址/模型），后续模型调用会失败"
        );
    } else {
        info!(模型 = %配置.模型, "模型配置就绪");
    }
    配置
}

/// 解析 .env 的 `键=值` 行，跳过空行与注释。
pub fn 解析环境文件(路径: &str) -> HashMap<String, String> {
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
