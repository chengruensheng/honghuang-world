//! 隔离-互斥-园 · 共享测试设施：进程级环境变量互斥锁与临时工作区辅助。
//!
//! 并行 cargo test 下，进程级环境变量（如 WORLD_WORKSPACE_ROOT）会被多个测试同时改写，
//! 撤销恢复用例会指向错误工作区。本设施提供全局共享 static 互斥锁，让所有改写进程级
//! 环境变量的测试用例串行执行，根治并行污染。
//!
//! 跨多个测试文件共用同一把锁（不再各声明独立 Mutex），调用方只需：
//!   use crate::道术施展_验证_殿::手脚_验证_阁::隔离_互斥_园::隔离设施::设施::临时工作区;
//!   let (根, _锁) = 临时工作区("标记", "用例名");
//! 持有 guard 至测试结束即可。

#[cfg(test)]
pub mod 设施 {
    use std::path::PathBuf;
    use std::sync::{Mutex, MutexGuard};

    /// 进程级全局共享互斥锁：用于 WORLD_WORKSPACE_ROOT 等进程级环境变量串行设置/使用。
    /// 跨多个测试文件共用同一把锁，确保撤销恢复用例不会互相覆盖。
    /// 锁在测试开头取到、测试末尾才释放，全程独占。
    pub static 环境变量锁: Mutex<()> = Mutex::new(());

    /// 临时工作区：创建唯一临时目录、设置 WORLD_WORKSPACE_ROOT 到该目录，返回 (根, guard)。
    /// 标记：用于区分不同园/不同用例组的根目录命名空间（如 "写文件"、"改文件"），避免互相覆盖。
    /// 名：用于区分同一园内的多个用例（如 "备份"、"新建"）。
    /// 调用方必须持有 guard 至测试结束（drop 后才释放锁），全程独占进程级环境变量。
    pub fn 临时工作区(标记: &str, 名: &str) -> (PathBuf, MutexGuard<'static, ()>) {
        let 锁 = 环境变量锁.lock().unwrap_or_else(|e| e.into_inner());
        let 根 = std::env::temp_dir().join(format!(
            "手脚架_{标记}_工作区_{}_{}",
            std::process::id(),
            名
        ));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        (根, 锁)
    }
}