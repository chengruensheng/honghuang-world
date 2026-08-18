//! 落盘 - 取队 - 园 · 落盘取队测试：入队取队水位与八态状态机。
//!
//! 测试隔离（2026-08-18 补齐）：进程级 `static Mutex<()>` 串行化 + 临时路径用
//! `process::id()` 命名（照 `模型落盘测试.rs` 模式），并行 cargo test 不再因
//! `std::env::temp_dir()` 残留导致水位断言假阴。

#[cfg(test)]
mod 测试 {
    use serde::{Deserialize, Serialize};
    use std::fs;
    use tianting_fu::{要求状态, 落盘队列, 状态推进};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct 测试项 { 名: String }

    /// 本 crate 测试进程级互斥锁：并行测试下 临时路径 不互相残留。
    static 测试环境锁: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 造临时路径（用 process::id 隔离并行测试）。
    fn 建临时路径(标签: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "识海测试-队列-{}-{}-{}",
            标签,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn 入队取队水位() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 路径 = 建临时路径("水位");
        let 队列 = 落盘队列::<测试项>::打开(&路径);
        队列.入队(&测试项 { 名: "一".to_string() }).unwrap();
        队列.入队(&测试项 { 名: "二".to_string() }).unwrap();
        assert_eq!(队列.水位().unwrap(), 2);
        let 取 = 队列.取队().unwrap().unwrap();
        assert_eq!(取.名, "一");
        assert_eq!(队列.水位().unwrap(), 1);
        let _ = fs::remove_file(&路径);
    }

    #[test]
    fn 非法迁移被拒() {
        assert!(状态推进(&要求状态::待领, &要求状态::已存档).is_err());
        assert!(状态推进(&要求状态::待确认, &要求状态::设计中).is_ok());
    }
}
