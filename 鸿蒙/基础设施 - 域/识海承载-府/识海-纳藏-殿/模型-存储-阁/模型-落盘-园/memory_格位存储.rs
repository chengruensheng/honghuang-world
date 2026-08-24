//! Memory 格位存储（§B.2.2 三个实现之三 — 测试用）。
//!
//! 用途：测试场景用 — 内存中存记录，不落盘，测试结束即清空。

use super::工作区;
use super::格位存储;
use crate::世界结果;
use crate::记录;
use std::sync::Mutex;

/// Memory 格位存储（线程安全）
pub struct Memory格位存储 {
    记录们: Mutex<Vec<记录>>,
}

impl Memory格位存储 {
    pub fn 新() -> Self {
        Self {
            记录们: Mutex::new(Vec::new()),
        }
    }
    pub fn 全部记录(&self) -> Vec<记录> {
        self.记录们.lock().unwrap().clone()
    }
}

impl 格位存储 for Memory格位存储 {
    fn 写记录(&self, 记录: &记录) -> 世界结果<()> {
        self.记录们.lock().unwrap().push(记录.clone());
        Ok(())
    }
    fn 在工作区(_工作区: &工作区) -> Box<dyn 格位存储> {
        Box::new(Memory格位存储::新())
    }
}
