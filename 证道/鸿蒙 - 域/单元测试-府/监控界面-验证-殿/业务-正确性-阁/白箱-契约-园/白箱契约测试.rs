#![cfg(test)]
//! 业务正确性：白箱六字段契约 + 事件类型派生 + JSON 序列化兼容前端字段名。
//! 依据：融合蓝图 §9.3 白箱六字段契约 + §13.f 轨迹表格 7 种事件类型。

use jiankong_fu::{token用量, 事件类型, 影响项, 白箱事件};

/// 业务正确性验收：白箱事件核心字段齐全（ts/源/动作/影响/token/耗时ms 六字段）。
#[test]
fn 白箱六字段_齐备() {
    let 影响 = vec![影响项 {
        类型: "文件".into(),
        名: "鸿蒙/xxx.rs".into(),
        变化: "+421".into(),
        字节: Some(421),
    }];
    let 事件 = 白箱事件 {
        ts: 1700000000000,
        源: "鸿蒙/道术施展-府".into(),
        动作: "工具循环-7".into(),
        影响,
        token: token用量::default(),
        耗时ms: 0,
        证据: String::new(),
        任务线id: String::new(),
        轮次: None,
        思考链: None, // §13.f.7a
    };
    assert!(事件.ts > 0, "ts 必填非零");
    assert!(!事件.源.is_empty(), "源 必填非空");
    assert!(!事件.动作.is_empty(), "动作 必填非空");
    assert!(!事件.影响.is_empty(), "影响 必填非空（白箱核心）");
}

/// 业务正确性验收：JSON 序列化含"耗时ms"中文键名（前端按字段名解析）。
#[test]
fn 白箱事件_json_含中文键名() {
    let 事件 = 白箱事件 {
        ts: 100,
        源: "源".into(),
        动作: "动作".into(),
        影响: vec![],
        token: token用量::default(),
        耗时ms: 50,
        证据: String::new(),
        任务线id: String::new(),
        轮次: None,
        思考链: Some("这是思考文本".to_string()), // §13.f.7a
    };
    let json = serde_json::to_string(&事件).unwrap();
    assert!(
        json.contains(r#""耗时ms""#),
        "JSON 必含耗时ms 中文键：{}",
        json
    );
    assert!(json.contains(r#""源""#), "JSON 必含源 中文键：{}", json);
    assert!(json.contains(r#""动作""#), "JSON 必含动作 中文键：{}", json);
    assert!(json.contains(r#""影响""#), "JSON 必含影响 中文键：{}", json);
    assert!(
        json.contains(r#""token""#),
        "JSON 必含 token 字段：{}",
        json
    );
}

/// 业务正确性验收：token 用量六档序列化，零值不序列化（向后兼容）。
#[test]
fn token六档_零值不序列化() {
    let t = token用量 {
        提示词: 100,
        输出: 50,
        缓存: 0,
        缓存写: 0,
        推理: 0,
        总计: 150,
    };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains(r#""提示词":100"#), "提示词应保留：{}", json);
    assert!(json.contains(r#""输出":50"#), "输出应保留：{}", json);
    assert!(json.contains(r#""总计":150"#), "总计应保留：{}", json);
    // 原四档（提示词/输出/缓存/总计）即使为 0 也序列化以保兼容
    assert!(
        json.contains(r#""缓存":0"#),
        "缓存（原四档）即使为 0 也序列化以保兼容：{}",
        json
    );
    // 新两档（§13.f.7）零值不序列化
    assert!(
        !json.contains(r#""缓存写""#),
        "缓存写=0 不应序列化：{}",
        json
    );
    assert!(!json.contains(r#""推理""#), "推理=0 不应序列化：{}", json);
}

/// 业务正确性验收：影响项 4 字段（类型/名/变化/字节）。
#[test]
fn 影响项_四字段() {
    let 项 = 影响项 {
        类型: "格位".into(),
        名: "调用".into(),
        变化: "+1 条".into(),
        字节: Some(256),
    };
    assert_eq!(项.类型, "格位");
    assert_eq!(项.名, "调用");
    assert_eq!(项.字节, Some(256));
    let json = serde_json::to_string(&项).unwrap();
    assert!(json.contains(r#""类型""#), "缺类型：{}", json);
    assert!(json.contains(r#""名""#), "缺名：{}", json);
    assert!(json.contains(r#""字节""#), "缺字节：{}", json);
}

/// 业务正确性验收：事件类型枚举序列化字段名（system/user/context/...）。
#[test]
fn 事件类型_英文序列化() {
    let cases = [
        (事件类型::系统, "system"),
        (事件类型::界主, "user"),
        (事件类型::上下文, "context"),
        (事件类型::压缩, "compacted"),
    ];
    for (类型, 期望) in cases {
        let json = serde_json::to_string(&类型).unwrap();
        let 期望json = format!(r#""{}""#, 期望);
        assert_eq!(json, 期望json, "类型 {:?} 应序列化为 {}", 类型, 期望);
    }
}

/// 业务正确性验收：白箱事件反序列化前端字段名一致。
#[test]
fn 白箱事件_反序列化兼容() {
    let json = r#"{
        "ts": 1700000000000,
        "源": "鸿蒙/道术施展-府",
        "动作": "工具循环-7",
        "影响": [{"类型": "文件", "名": "test.rs", "字节": 421}],
        "token": {"提示词": 100, "输出": 50, "缓存": 0, "总计": 150},
        "耗时ms": 1234
    }"#;
    let 事件: 白箱事件 = serde_json::from_str(json).expect("反序列化应成功");
    assert_eq!(事件.ts, 1700000000000);
    assert_eq!(事件.源, "鸿蒙/道术施展-府");
    assert_eq!(事件.动作, "工具循环-7");
    assert_eq!(事件.影响.len(), 1);
    assert_eq!(事件.影响[0].类型, "文件");
    assert_eq!(事件.token.提示词, 100);
    assert_eq!(事件.耗时ms, 1234);
}
