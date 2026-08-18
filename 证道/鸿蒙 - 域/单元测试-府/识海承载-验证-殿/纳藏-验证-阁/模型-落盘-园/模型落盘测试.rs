//! 模型-落盘-园 · 测试：工作区定位 + 格位/记录落盘读写。
//!
//! 测试隔离（2026-08-18 补齐）：进程级 `static Mutex<()>` 串行化 + 临时工作区用
//! `process::id()` 命名（照 `缓存读取.rs` 模式），并行 cargo test 不再因临时目录
//! 残留导致 假阴（`断言: left=26, right=1` 类污染）。

#[cfg(test)]
mod 测试 {
    use shihai_fu::{工作区, 模型存储, 记录};
    use std::fs;

    /// 本 crate 测试进程级 env 互斥锁：并行测试下 `write_record` 不互相覆盖残留。
    static 测试环境锁: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 造临时工作区：返回工作区根（用 process::id 隔离并行测试）。
    fn 建临时工作区(标签: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "识海测试-模型落盘-{}-{}-{}",
            标签,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn 写入再读回一致() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 目录 = 建临时工作区("写入");
        let 存储 = 模型存储::打开(&目录);
        let 记录 = 记录::新("结构", "鸿蒙/基础设施-域", "测试", "代码");
        存储.写记录(&记录).unwrap();
        let 读回 = 存储.读格位("结构").unwrap();
        assert_eq!(读回.len(), 1);
        assert_eq!(读回[0].内容, "鸿蒙/基础设施-域");
        let _ = fs::remove_dir_all(&目录);
    }

    #[test]
    fn 工作区初始化建目录() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 根 = 建临时工作区("初始化");
        let 工作区 = 工作区::新(&根);
        工作区.初始化().unwrap();
        assert!(工作区.格位目录().is_dir());
        assert!(工作区.会话目录().is_dir());
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 在工作区落盘到上下文() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 根 = 建临时工作区("落盘");
        let 工作区 = 工作区::新(&根);
        let 存储 = 模型存储::在工作区(&工作区);
        let 记录 = 记录::新("结构", "落盘", "测试", "代码");
        存储.写记录(&记录).unwrap();
        assert!(工作区.格位目录().join("结构.jsonl").exists());
        let _ = fs::remove_dir_all(&根);
    }
}
