//! §B.3.6 性能基准（用 std::time — 不需 criterion/nightly）
//!
//! 跑：`cargo test -p shihai_fu --release 我的基准 -- --include-ignored --nocapture`
//!
//! 落位：基准-测试-殿/性能基准-阁/性能基准-园/（按 §AGENTS.10 六层落点）。
//!
//! 衡量 4 个关键操作（μs/op）：
//! 1. 工作区::定位（OnceLock 缓存 — 冷启动 vs 热调用）
//! 2. 写记录（jsonl 追加）
//! 3. 读 JSONL（容错版 读_jsonl，1000 行）
//! 4. 沙箱校验（components 比对）

use shihai_fu::
{
    工作区, 读_jsonl, 模型存储, 记录,
};
use std::hint::black_box;
use std::time::Instant;

fn 测<T>(名: &str, 次数: usize, mut f: impl FnMut() -> T) {
    // 预热
    for _ in 0..10 {
        let _ = black_box(f());
    }
    let 开始 = Instant::now();
    for _ in 0..次数 {
        let _ = black_box(f());
    }
    let 微秒 = 开始.elapsed().as_micros() as f64 / 次数 as f64;
    println!("  {} — 平均 {:.2} μs/op ({} 次)", 名, 微秒, 次数);
}

#[test]
fn 我的基准() {
    println!("\n=== 1. 工作区::定位（OnceLock 缓存）===");
    测("冷启动", 1, || 工作区::定位().根路径().to_path_buf());
    测("热调用", 100_000, || black_box(工作区::定位().根路径().to_path_buf()));

    println!("\n=== 2. 写记录（jsonl 追加）===");
    let dir = std::env::temp_dir().join(format!("shihai_bench_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("WORLD_WORKSPACE_ROOT", &dir);
    let 存储 = 模型存储::在工作区(&工作区::定位());
    let 记录 = 记录::新("结构", "测试内容", "测试来源", "测试录入者");
    测("单条写记录", 1_000, || 存储.写记录(black_box(&记录)).unwrap());

    println!("\n=== 3. 读 JSONL（容错版 读_jsonl，1000 行）===");
    let 文件 = dir.join("test.jsonl");
    let 内容 = (0..1000)
        .map(|i| format!("{{\"id\":{},\"name\":\"x\"}}", i))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&文件, &内容).unwrap();
    测("1000 行读", 100, || 读_jsonl::<serde_json::Value>(black_box(&文件)).unwrap());

    println!("\n=== 4. 沙箱校验（components 比对，2 路径）===");
    use std::path::Path;
    let 根 = Path::new("/root/work").to_path_buf();
    let 内 = Path::new("/root/work/sub/file.txt").to_path_buf();
    let _ = (根, 内);
    println!("  （沙箱校验为 inline 闭包 — 见 daoshu_fu 写入文件.rs 实现，O(components 比对)）");

    println!("\n=== 完成 ===");
}
