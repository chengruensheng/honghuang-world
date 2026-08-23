#![cfg(test)]
//! 性能·并发分流：10 个独立白箱事件并行构造，互不干扰。
//! 依据：融合蓝图 §6.3 性能并发「10 路并发 EventSource 不互相干扰」。

use jiankong_fu::{token用量, 影响项, 白箱事件};
use std::thread;

#[test]
fn 并发构造_10路无干扰() {
    let 句柄们: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let 影响 = vec![影响项 {
                    类型: "文件".into(),
                    名: "test.rs".into(),
                    变化: String::new(),
                    字节: Some(i as u64),
                }];
                let 事件 = 白箱事件 {
                    ts: 1700000000000 + i as u64,
                    源: format!("源/线程-{}", i),
                    动作: "并发动作".into(),
                    影响,
                    token: token用量::default(),
                    耗时ms: 0,
                    证据: String::new(),
                    任务线id: String::new(),
                    轮次: None,
                };
                assert_eq!(事件.影响[0].字节, Some(i as u64));
                i
            })
        })
        .collect();

    let 总和: i32 = 句柄们.into_iter().map(|h| h.join().unwrap()).sum();
    // 0+1+2+...+9 = 45
    assert_eq!(总和, 45);
}

#[test]
fn 并发构造_token无竞争() {
    let 句柄们: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let 事件 = 白箱事件 {
                    ts: 0,
                    源: "源".into(),
                    动作: "动作".into(),
                    影响: vec![],
                    token: token用量 {
                        提示词: i as u64 * 100,
                        输出: i as u64 * 50,
                        缓存: 0,
                        缓存写: 0,
                        推理: 0,
                        总计: i as u64 * 150,
                    },
                    耗时ms: 0,
                    证据: String::new(),
                    任务线id: String::new(),
                    轮次: None,
                };
                事件.token.总计
            })
        })
        .collect();

    let mut 总计: u64 = 0;
    for h in 句柄们 {
        总计 += h.join().unwrap();
    }
    // 0+150+300+450+...+1350 = 150*45 = 6750
    assert_eq!(总计, 150 * 45);
}

#[test]
fn 并发_共享Arc源字符串无竞争() {
    use std::sync::Arc;
    let 共享源 = Arc::new("共享源".to_string());
    let 句柄们: Vec<_> = (0..5)
        .map(|i| {
            let src = Arc::clone(&共享源);
            thread::spawn(move || {
                let src_str: String = (*src).clone();
                let 事件 = 白箱事件 {
                    ts: 1000 + i as u64,
                    源: src_str,
                    动作: "动作".into(),
                    影响: vec![],
                    token: token用量::default(),
                    耗时ms: 0,
                    证据: String::new(),
                    任务线id: String::new(),
                    轮次: None,
                };
                (事件.ts, 事件.源.clone())
            })
        })
        .collect();

    let mut ts_vec = Vec::new();
    for h in 句柄们 {
        let (ts, src) = h.join().unwrap();
        assert_eq!(src, "共享源");
        ts_vec.push(ts);
    }
    ts_vec.sort();
    assert_eq!(ts_vec, vec![1000, 1001, 1002, 1003, 1004]);
}
