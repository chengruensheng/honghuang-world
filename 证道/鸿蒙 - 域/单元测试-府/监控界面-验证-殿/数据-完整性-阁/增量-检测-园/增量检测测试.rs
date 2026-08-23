#![cfg(test)]
//! 数据完整性·增量检测：三源文件大小变化触发推送，断点续读。
//! 依据：融合蓝图 §6.3 数据完整性 + §9.3 白箱六字段。

use std::io::Write;
use std::sync::Mutex;

use jiankong_fu::{事件流路径, 观测记录路径, 识海记录路径};

/// 进程级全局锁：避免并行测试互相覆盖 WORLD_WORKSPACE_ROOT。
static 测试锁: Mutex<()> = Mutex::new(());

fn 唯一工作区(名: &str) -> std::path::PathBuf {
    let 进程id = std::process::id();
    let 纳秒 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("dsh-jiankong-{名}-{进程id}-{纳秒}"))
}

/// 写一条白箱事件到事件流 jsonl（手工构造，模拟天庭事件追加）。
fn 追加事件流(path: &std::path::Path, ts: u64, 动作: &str) {
    let 行 = format!(
        r#"{{"ts":{ts},"源":"鸿蒙/天庭治理-府","动作":"{}","影响":[],"token":{{}},"耗时ms":100}}
"#,
        动作
    );
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    f.write_all(行.as_bytes()).unwrap();
}

#[test]
fn 三源路径_切工作区生效() {
    let _锁 = 测试锁.lock().unwrap();
    let 根 = 唯一工作区("路径");
    std::fs::create_dir_all(根.join(".上下文/观测")).unwrap();
    unsafe {
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
    }

    let 事件流 = 事件流路径();
    let 观测 = 观测记录路径();
    let 识海 = 识海记录路径();

    // 三源路径的父目录都应在测试工作区 .上下文 下
    assert!(
        事件流.parent().unwrap().starts_with(根.join(".上下文")),
        "事件流路径应切到测试工作区：{:?}",
        事件流
    );
    assert!(
        观测.starts_with(根.join(".上下文")),
        "观测记录路径应切到测试工作区：{:?}",
        观测
    );
    assert!(
        识海.starts_with(根.join(".上下文")),
        "识海记录路径应切到测试工作区：{:?}",
        识海
    );

    unsafe {
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 事件流_追加后文件大小递增() {
    let _锁 = 测试锁.lock().unwrap();
    let 根 = 唯一工作区("追加");
    std::fs::create_dir_all(根.join(".上下文")).unwrap();
    unsafe {
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
    }

    let path = 事件流路径();
    assert!(!path.exists(), "初始不应存在");

    追加事件流(&path, 1000, "动作-A");
    let 大小1 = std::fs::metadata(&path).unwrap().len();
    assert!(大小1 > 0, "追加后大小 > 0");

    追加事件流(&path, 2000, "动作-B");
    let 大小2 = std::fs::metadata(&path).unwrap().len();
    assert!(大小2 > 大小1, "再次追加大小递增");

    unsafe {
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 三源独立_互不干扰() {
    let _锁 = 测试锁.lock().unwrap();
    let 根 = 唯一工作区("独立");
    std::fs::create_dir_all(根.join(".上下文/观测")).unwrap();
    unsafe {
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
    }

    // 仅追加观测记录
    let obs = 观测记录路径();
    std::fs::write(
        &obs,
        r#"{"ts":1,"源":"x","动作":"y","影响":[],"token":{},"耗时ms":0}
"#,
    )
    .unwrap();

    assert!(obs.exists(), "观测记录应存在：{:?}", obs);
    assert!(!事件流路径().exists(), "事件流应不存在");
    assert!(!识海记录路径().exists(), "识海记录应不存在");

    unsafe {
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }
    let _ = std::fs::remove_dir_all(&根);
}
