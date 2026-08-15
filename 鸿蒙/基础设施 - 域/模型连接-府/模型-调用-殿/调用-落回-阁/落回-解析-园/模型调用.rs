//! 调用 - 落回 - 园：调用模型，解析落回内容。

use crate::类型_定义_殿::对话消息;
use peizhi_fu::模型配置;

/// 一次模型调用：发送消息，取回文本内容。
pub fn 调用模型(配置: &模型配置, 消息们: &[对话消息]) -> Result<String, String> {
    let 请求体 = 构造请求体(配置, 消息们);

    let 响应 = ureq::post(&配置.地址)
        .set("Authorization", &format!("Bearer {}", 配置.密钥))
        .set("Content-Type", "application/json")
        .send_string(&请求体)
        .map_err(|错误| format!("模型请求失败: {错误}"))?;

    let 文本 = 响应
        .into_string()
        .map_err(|错误| format!("读取响应失败: {错误}"))?;

    解析回复(&文本)
}

/// 构造 OpenAI 兼容请求体（独立出便于测试）。
pub fn 构造请求体(配置: &模型配置, 消息们: &[对话消息]) -> String {
    serde_json::json!({
        "model": 配置.模型,
        "messages": 消息们.iter().map(|消息| serde_json::json!({
            "role": 消息.角色,
            "content": 消息.内容,
        })).collect::<Vec<_>>(),
    })
    .to_string()
}

/// 从 OpenAI 兼容响应解析 choices[0].message.content。
pub fn 解析回复(文本: &str) -> Result<String, String> {
    let 解析: serde_json::Value =
        serde_json::from_str(文本).map_err(|错误| format!("解析响应失败: {错误}"))?;
    解析["choices"][0]["message"]["content"]
        .as_str()
        .map(|段| 段.to_string())
        .ok_or_else(|| "响应缺少 choices[0].message.content".to_string())
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::类型_定义_殿::对话消息;

    #[test]
    fn 构造请求体含模型与消息() {
        let 配置 = 模型配置 {
            密钥: "k".to_string(),
            地址: "https://example.com/v1/chat/completions".to_string(),
            模型: "MiniMax-M3".to_string(),
        };
        let 体 = 构造请求体(&配置, &[对话消息::用户("你好")]);
        assert!(体.contains("MiniMax-M3"));
        assert!(体.contains("你好"));
    }

    #[test]
    fn 解析回复取内容() {
        let 文本 = r#"{"choices":[{"message":{"content":"回答"}}]}"#;
        assert_eq!(解析回复(文本).unwrap(), "回答");
    }
}
