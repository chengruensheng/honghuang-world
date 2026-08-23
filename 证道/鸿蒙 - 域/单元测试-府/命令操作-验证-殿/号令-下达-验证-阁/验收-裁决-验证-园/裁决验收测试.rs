//! 裁决验收函数单元测试
//! 覆盖：正向用例（通过/打回/意见特殊字符/空意见）、负向用例（非法结论）、
//! 幂等用例（相同要求id 重复入队）、并发隔离用例（多线程独立要求id 互不污染）
//! 隔离：每测试用 WORLD_WORKSPACE_ROOT 指向独立临时目录，teardown 清理

#![allow(non_snake_case)]

use mingling_fu::裁决验收;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;

fn 临时工作区() -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let 路径 = env::temp_dir().join(format!("裁决验收测试_{pid}_{nanos}"));
    fs::create_dir_all(&路径).expect("创建临时工作区");
    路径
}

fn 设置工作区(根: &PathBuf) {
    // 安全说明：cargo test 默认单线程运行测试函数；并发用例仅在 fn 内部 spawn
    // 线程且不修改 env，env 仅由主线程串行设置/还原，避免数据竞争。
    unsafe { env::set_var("WORLD_WORKSPACE_ROOT", 根) };
}

fn 还原工作区() {
    unsafe { env::remove_var("WORLD_WORKSPACE_ROOT") };
}

fn 清理(根: &PathBuf) {
    let _ = fs::remove_dir_all(根);
}

fn 验收文件(根: &PathBuf) -> PathBuf {
    根.join(".上下文").join("状态").join("验收.jsonl")
}

#[test]
fn 裁决验收_通过_返回成功前缀且落盘() {
    let 根 = 临时工作区();
    设置工作区(&根);
    let 结果 = 裁决验收("要求-001", "通过", "通过-意见");
    assert!(结果.starts_with("验收已裁决"), "实际：{结果}");
    assert!(结果.contains("结论：通过"), "实际：{结果}");
    assert!(结果.contains("意见：通过-意见"), "实际：{结果}");

    let 文件 = 验收文件(&根);
    assert!(文件.exists(), "应落盘验收.jsonl");
    let 内容 = fs::read_to_string(&文件).unwrap();
    assert!(内容.contains("要求-001"), "实际：{内容}");
    assert!(内容.contains("通过"), "实际：{内容}");

    还原工作区();
    清理(&根);
}

#[test]
fn 裁决验收_打回_返回成功前缀且落盘() {
    let 根 = 临时工作区();
    设置工作区(&根);
    let 结果 = 裁决验收("要求-002", "打回", "未达验收标准");
    assert!(结果.starts_with("验收已裁决"), "实际：{结果}");
    assert!(结果.contains("结论：打回"), "实际：{结果}");
    assert!(结果.contains("意见：未达验收标准"), "实际：{结果}");

    let 文件 = 验收文件(&根);
    let 内容 = fs::read_to_string(&文件).unwrap();
    assert!(内容.contains("要求-002"), "实际：{内容}");
    assert!(内容.contains("打回"), "实际：{内容}");

    还原工作区();
    清理(&根);
}

#[test]
fn 裁决验收_非法结论_返回错误前缀且不落盘该要求id() {
    let 根 = 临时工作区();
    设置工作区(&根);
    let 结果 = 裁决验收("要求-003", "也许通过", "无所谓");
    assert!(
        结果.starts_with("结论需为 通过|打回，当前："),
        "实际：{结果}"
    );
    assert!(结果.contains("也许通过"), "实际：{结果}");

    let 文件 = 验收文件(&根);
    if 文件.exists() {
        let 内容 = fs::read_to_string(&文件).unwrap();
        assert!(
            !内容.contains("要求-003"),
            "非法结论不应落盘该要求id：{内容}"
        );
    }

    还原工作区();
    清理(&根);
}

#[test]
fn 裁决验收_意见含特殊字符_仍能落盘() {
    let 根 = 临时工作区();
    设置工作区(&根);
    let 意见 = "意见含\n换行\"引号\\反斜杠🚀emoji";
    let 结果 = 裁决验收("要求-004", "通过", 意见);
    assert!(结果.starts_with("验收已裁决"), "实际：{结果}");

    let 文件 = 验收文件(&根);
    let 内容 = fs::read_to_string(&文件).unwrap();
    assert!(内容.contains("要求-004"), "实际：{内容}");

    还原工作区();
    清理(&根);
}

#[test]
fn 裁决验收_空意见_仍能落盘() {
    let 根 = 临时工作区();
    设置工作区(&根);
    let 结果 = 裁决验收("要求-005", "通过", "");
    assert!(结果.starts_with("验收已裁决"), "实际：{结果}");

    let 文件 = 验收文件(&根);
    let 内容 = fs::read_to_string(&文件).unwrap();
    assert!(内容.contains("要求-005"), "实际：{内容}");

    还原工作区();
    清理(&根);
}

#[test]
fn 裁决验收_幂等性_相同要求id重复入队均成功且均落盘() {
    let 根 = 临时工作区();
    设置工作区(&根);

    let 一 = 裁决验收("要求-幂等", "通过", "首次");
    assert!(一.starts_with("验收已裁决"), "实际：{一}");
    let 二 = 裁决验收("要求-幂等", "打回", "再次");
    assert!(二.starts_with("验收已裁决"), "实际：{二}");

    let 文件 = 验收文件(&根);
    let 内容 = fs::read_to_string(&文件).unwrap();
    let 出现次数 = 内容.matches("要求-幂等").count();
    assert_eq!(出现次数, 2, "应落盘两条记录，实际：{内容}");

    还原工作区();
    清理(&根);
}

#[test]
fn 裁决验收_并发隔离_多线程独立要求id互不污染() {
    let 根 = 临时工作区();
    设置工作区(&根);

    let 线程数 = 4;
    let 每线程调用 = 5;
    let 屏障 = Arc::new(Barrier::new(线程数));
    let mut 句柄 = Vec::new();

    for t in 0..线程数 {
        let 屏障 = Arc::clone(&屏障);
        句柄.push(thread::spawn(move || {
            屏障.wait();
            let mut 局部结果 = Vec::new();
            for i in 0..每线程调用 {
                let 要求id = format!("并发-T{t}-I{i}");
                let 结果 = 裁决验收(&要求id, "通过", &format!("线程{t}-调用{i}"));
                局部结果.push((要求id, 结果));
            }
            局部结果
        }));
    }

    let mut 所有要求id = Vec::new();
    for h in 句柄 {
        let 局部 = h.join().expect("线程不应panic");
        for (id, 结果) in 局部 {
            assert!(
                结果.starts_with("验收已裁决"),
                "{id} 应成功，实际：{结果}"
            );
            所有要求id.push(id);
        }
    }

    let 文件 = 验收文件(&根);
    let 内容 = fs::read_to_string(&文件).unwrap();
    for id in &所有要求id {
        assert!(内容.contains(id), "应含 {id}，实际：{内容}");
    }

    let 总条目 = 内容
        .lines()
        .filter(|l| l.contains("并发-T"))
        .count();
    assert_eq!(
        总条目,
        线程数 * 每线程调用,
        "应落盘 {线程数}*{每线程调用}={} 条，实际：{内容}",
        线程数 * 每线程调用
    );

    还原工作区();
    清理(&根);
}