use hm_content_contract::模型响应;
use hm_error::{Error, Result};

use super::super::解析工具调用;

/// 响应解析防线：在接收层把「无产出包裹」就地判败。
///
/// 背景：限流/故障/截断中的供应商可能返回 HTTP 200 + 无产出内容。若放行，
/// 下游（阶段产出解析等）会把「答复为空」误判为「模型答非所问」，
/// 触发回喂重试白烧一轮；在此判败 Err 后，池的故障转移
/// 会自动把请求导向下一家供应商。生成与对话两条路径的判空
/// 口径不同，故拆为两个函数，避免一处放宽两处失守。

/// 生成路径：`content` 缺失**或 trim 后为空**都判败。
///
/// 生成没有工具调用语义，空文本即无产出，无合法空场景。
pub(super) fn 提取生成文本(值: &serde_json::Value) -> Result<String> {
    let 文本 = 值["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| Error::模型("模型响应缺少 choices[0].message.content".into()))?;
    if 文本.trim().is_empty() {
        return Err(Error::模型(
            "模型响应内容为空（疑似限流或故障），按失败转移下一供应商".into(),
        ));
    }
    Ok(文本.to_string())
}

/// 对话路径：**无正文且无工具调用**即判败（思考不算产出）。
///
/// 只调用工具的轮次 `content` 可为空，但此时必有 `tool_calls`，属合法响应。
/// 反过来「思考满、正文空、无工具调用」不是合法轮次而是**截断**：
/// 推理模型思考链会吃光 `max_tokens`（实测 a6api 默认 8192 全被 reasoning 占满），
/// 供应商仍回 HTTP 200——若放行，驱动只会拿到空答复去解析 JSON 并白烧回喂重试。
/// 判败后由池故障转移下一家，或至少让失败在接收层显形（不伪装成「模型答非所问」）。
pub(super) fn 提取对话响应(值: &serde_json::Value) -> Result<模型响应> {
    let 消息体 = &值["choices"][0]["message"];
    let 响应 = 模型响应 {
        内容: 消息体["content"].as_str().map(|s| s.to_string()),
        工具调用: 解析工具调用(消息体),
        思考: 消息体["reasoning_content"]
            .as_str()
            .or_else(|| 消息体["reasoning"].as_str())
            .map(|s| s.to_string()),
    };
    let 内容空 = 响应.内容.as_ref().map_or(true, |s| s.trim().is_empty());
    if 内容空 && 响应.工具调用.is_empty() {
        let 被截断 = 值["choices"][0]["finish_reason"].as_str() == Some("length");
        let 说明 = if 被截断 {
            "输出被截断（finish_reason=length：思考链耗尽 max_tokens），请调大供应商 max_tokens"
        } else {
            "模型响应无正文且无工具调用（疑似限流或故障）"
        };
        return Err(Error::模型(format!("{说明}，按失败转移下一供应商")));
    }
    Ok(响应)
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use serde_json::json;

    #[test]
    fn 生成_正常文本放行() {
        let v = json!({"choices":[{"message":{"content":"你好"}}]});
        assert_eq!(提取生成文本(&v).expect("应放行"), "你好");
    }

    #[test]
    fn 生成_空白内容判败() {
        let v = json!({"choices":[{"message":{"content":"   \n "}}]});
        assert!(提取生成文本(&v).is_err(), "空白 content 应按空包裹判败");
    }

    #[test]
    fn 生成_缺content判败() {
        let v = json!({"choices":[{"message":{}}]});
        assert!(提取生成文本(&v).is_err());
    }

    #[test]
    fn 对话_纯空包裹判败() {
        let v = json!({"choices":[{"message":{"content":""}}]});
        assert!(提取对话响应(&v).is_err(), "无正文且无工具调用应判败");
    }

    #[test]
    fn 对话_缺content且有工具调用放行() {
        let v = json!({"choices":[{"message":{"tool_calls":[
            {"id":"1","type":"function","function":{"name":"读文件","arguments":"{}"}}
        ]}}]});
        let r = 提取对话响应(&v).expect("工具调用轮次应放行");
        assert_eq!(r.工具调用.len(), 1);
    }

    #[test]
    fn 对话_思考满正文空且无工具判败() {
        // 推理模型思考链吃光 max_tokens 的截断形态：思考非空但无产出，必须判败而非放行
        let v = json!({"choices":[{"finish_reason":"length","message":{
            "content":"", "reasoning_content":"长长长思考"
        }}]});
        let 错误 = format!("{}", 提取对话响应(&v).unwrap_err());
        assert!(错误.contains("截断"), "应指明截断并提示调大 max_tokens：{错误}");
    }

    #[test]
    fn 对话_思考非空正文空且无截断说明判败() {
        let v = json!({"choices":[{"finish_reason":"stop","message":{
            "content":null, "reasoning_content":"只思考不产出"
        }}]});
        let 错误 = format!("{}", 提取对话响应(&v).unwrap_err());
        assert!(错误.contains("无正文且无工具调用"), "非截断情形应给出通用判败说明：{错误}");
    }

    #[test]
    fn 对话_正常内容放行() {
        let v = json!({"choices":[{"message":{"content":"答案"}}]});
        let r = 提取对话响应(&v).expect("应放行");
        assert_eq!(r.内容.as_deref(), Some("答案"));
    }
}
