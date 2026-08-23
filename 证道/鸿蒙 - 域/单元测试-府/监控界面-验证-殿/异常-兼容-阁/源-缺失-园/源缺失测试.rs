#![cfg(test)]
//! 异常兼容·源缺失：三源文件不存在/为空/损坏时不阻断直播。
//! 依据：融合蓝图 §6.3 异常兼容「任一源府 lib 根读失败 → 直播只标该庭暂无事件，不阻塞他庭」。

use std::io::Write;
use std::sync::Mutex;

use jiankong_fu::{事件流路径, 观测记录路径, 识海记录路径, 读事件流};

/// 进程级全局锁：避免并行测试互相覆盖 WORLD_WORKSPACE_ROOT。
/// 测试串行执行（毫秒级），不阻塞整体。
static 测试锁: Mutex<()> = Mutex::new(());

fn 唯一工作区(名: &str) -> std::path::PathBuf {
    let 进程id = std::process::id();
    let 纳秒 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("dsh-jiankong-{名}-{进程id}-{纳秒}"))
}

fn 清空三源(根: &std::path::Path) {
    let _ = std::fs::create_dir_all(根.join(".上下文/观测"));
    let _ = std::fs::remove_file(根.join(".上下文/事件流.jsonl"));
    let _ = std::fs::remove_file(根.join(".上下文/观测/记录.jsonl"));
    let _ = std::fs::remove_file(根.join(".上下文/记录.jsonl"));
}

/// 三源全缺失：路径函数仍可调用，不 panic。
#[test]
fn 三源全缺失_路径函数不panic() {
    let _锁 = 测试锁.lock().unwrap();
    let 根 = 唯一工作区("全缺失");
    清空三源(&根);
    unsafe {
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
    }

    let ef = 事件流路径();
    let obs = 观测记录路径();
    let sh = 识海记录路径();
    // 路径可计算（不 panic）
    assert!(ef.to_string_lossy().contains("事件流"));
    assert!(obs.to_string_lossy().contains("观测"));
    assert!(sh.to_string_lossy().contains("记录"));
    // 文件实际不存在
    assert!(!ef.exists(), "事件流不应存在：{:?}", ef);
    assert!(!obs.exists(), "观测记录不应存在：{:?}", obs);
    assert!(!sh.exists(), "识海记录不应存在：{:?}", sh);

    unsafe {
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }
    let _ = std::fs::remove_dir_all(&根);
}

/// 单源缺失：另两源仍可访问。
#[test]
fn 单源缺失_他源仍可读() {
    let _锁 = 测试锁.lock().unwrap();
    let 根 = 唯一工作区("单缺失");
    清空三源(&根);
    unsafe {
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
    }

    // 只写事件流
    let 内容 = "{\"ts\":1,\"源\":\"x\",\"动作\":\"y\",\"影响\":[],\"token\":{},\"耗时ms\":0}\n";
    let mut f = std::fs::File::create(事件流路径()).unwrap();
    f.write_all(内容.as_bytes()).unwrap();

    assert!(事件流路径().exists(), "事件流应存在：{:?}", 事件流路径());
    assert!(!观测记录路径().exists(), "观测记录应不存在");
    assert!(!识海记录路径().exists(), "识海记录应不存在");

    unsafe {
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }
    let _ = std::fs::remove_dir_all(&根);
}

/// 源文件损坏：含无效 JSON 行 + 合法行。
#[test]
fn 源文件损坏_读函数不panic() {
    let _锁 = 测试锁.lock().unwrap();
    let 根 = 唯一工作区("损坏");
    清空三源(&根);
    unsafe {
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
    }

    // 混合：garbage + 合法 + garbage
    let 路径 = 事件流路径();
    let mut f = std::fs::File::create(&路径).unwrap();
    f.write_all(b"garbage line\n").unwrap();
    let 合法行 = "{\"ts\":1,\"源\":\"x\",\"动作\":\"y\",\"影响\":[],\"token\":{},\"耗时ms\":0}\n";
    f.write_all(合法行.as_bytes()).unwrap();
    f.write_all(b"more garbage\n").unwrap();
    drop(f);

    // 读 0..全量，跳过非法行返回合法事件
    let 大小 = std::fs::metadata(&路径).unwrap().len();
    let 事件们 = 读事件流(0, 大小);
    assert!(!事件们.is_empty(), "应至少解析 1 条合法事件：{:?}", 事件们);

    unsafe {
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }
    let _ = std::fs::remove_dir_all(&根);
}
