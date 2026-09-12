//! 接待模块拆分后的独立测试模块（原 接待模块.rs 内 mod tests 搬移）

use std::sync::Arc;

use hm_contract::Component;
use hm_content_contract::{对话消息, 工具对话器, 工具调用, 模型响应};
use hm_error::Result;
use serde_json::json;

use super::接待模块::{
    保留最近, 会话历史上限, 会话消息, 键_优先级, 键_回复, 键_场景, 键_描述, 键_标题, 工具_闲聊,
    工具_对齐总结, 解析意图, 角色_道祖, 需求已明确, 道祖接待,
};

/// mock 对话器：仅返回纯文本响应（无工具调用），供 道祖接待 集成测试构造
struct Mock对话器;
impl Component for Mock对话器 {
    fn name(&self) -> &'static str {
        "mock"
    }
}
impl 工具对话器 for Mock对话器 {
    fn 对话(&self, _: Vec<对话消息>, _: Vec<serde_json::Value>) -> Result<模型响应> {
        Ok(模型响应 { 内容: Some("好的".into()), 工具调用: vec![], 思考: None })
    }
}

fn 构造工具调用(名称: &str, 参数: serde_json::Value) -> 工具调用 {
    工具调用 { id: "t1".into(), 名称: 名称.into(), 参数: 参数.to_string() }
}

/// 对齐总结 tool_call → 需求摘要 + 阶段=待确认
#[test]
fn 对齐总结工具调用_产出需求摘要() {
    let 响应 = 模型响应 {
        内容: None,
        工具调用: vec![构造工具调用(
            工具_对齐总结,
            json!({ 键_标题: "新建 jia-shang crate", 键_描述: "在 crates 目录新建纯函数 crate，暴露两数相加", 键_场景: "设计", 键_优先级: "P1" }),
        )],
        思考: None,
    };
    let (回复, 需求) = 解析意图(&响应).unwrap();
    assert!(需求.is_some(), "对齐总结应产出需求摘要");
    let 摘要 = 需求.unwrap();
    assert_eq!(摘要.标题, "新建 jia-shang crate");
    assert_eq!(摘要.描述, "在 crates 目录新建纯函数 crate，暴露两数相加");
    assert_eq!(摘要.场景.as_deref(), Some("设计"));
    assert_eq!(摘要.优先级.as_deref(), Some("P1"));
    assert!(回复.contains("新建 jia-shang crate"));
}

/// 闲聊 工具 → 回复文本，无需求（阶段=接待中）
#[test]
fn 闲聊工具调用_无需求() {
    let 响应 = 模型响应 {
        内容: None,
        工具调用: vec![构造工具调用(工具_闲聊, json!({ 键_回复: "善。" }))],
        思考: None,
    };
    let (回复, 需求) = 解析意图(&响应).unwrap();
    assert_eq!(回复, "善。");
    assert!(需求.is_none());
}

/// 无工具调用（纯文本答复）→ 无需求
#[test]
fn 无工具调用_纯文本_无需求() {
    let 响应 = 模型响应 { 内容: Some("好的".into()), 工具调用: vec![], 思考: None };
    let (回复, 需求) = 解析意图(&响应).unwrap();
    assert_eq!(回复, "好的");
    assert!(需求.is_none());
}

/// 对齐总结缺标题/描述 → 报错（防脏数据入库）
#[test]
fn 对齐总结缺字段_报错() {
    let 响应 = 模型响应 {
        内容: None,
        工具调用: vec![构造工具调用(工具_对齐总结, json!({ 键_标题: "", 键_描述: "" }))],
        思考: None,
    };
    assert!(解析意图(&响应).is_err());
}

/// 含任务特征词 → 需求已明确（触发对齐总结引导）
#[test]
fn 明确任务判为已明确() {
    assert!(需求已明确("请在 crates 目录新建 jia-shang crate，暴露 pub fn 两数相加"));
    assert!(需求已明确("请实现一个两数相加的函数，Rust，优先级 P1，场景设计"));
}

/// 纯闲聊/问候 → 不判定为明确（不干扰澄清/闲聊）
#[test]
fn 闲聊不判为明确() {
    assert!(!需求已明确("你好"));
    assert!(!需求已明确("随便聊聊今天天气"));
}

/// 长会话下 更新会话 把历史截断到上限（有界环形，最旧被丢弃）
#[test]
fn 更新会话_历史保持有界() {
    let 接待 = 道祖接待::新(Arc::new(Mock对话器));
    for i in 0..(会话历史上限 + 6) {
        接待
            .更新会话(&format!("消息{i}"), &模型响应 { 内容: Some(format!("回复{i}")), 工具调用: vec![], 思考: None })
            .unwrap();
    }
    let 会话 = 接待.会话.lock().expect("道祖接待锁中毒");
    assert!(会话.历史.len() <= 会话历史上限, "历史应被截断到上限，实际 {}", 会话.历史.len());
    assert_eq!(会话.历史.last().unwrap().角色, 角色_道祖);
    assert!(会话.历史.iter().all(|m| !m.内容.contains("消息0")), "最旧消息应被丢弃");
}

/// 历史未超上限 → 不截断（避免误丢早期澄清）
#[test]
fn 保留最近_未超上限_不截断() {
    let mut 历史 = vec![会话消息 { 角色: "用户".into(), 内容: "你好".into() }];
    保留最近(&mut 历史);
    assert_eq!(历史.len(), 1);
}
