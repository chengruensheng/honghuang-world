// 模型 - 落盘 - 园：工作区定位 + 心智模型 + 格位/记录落盘读写
//
// §B.2.2 三个实现：Jsonl (default) / Sqlite (B.3 实装) / Memory (测试用)
#[path = "模型落盘.rs"]
pub mod 模型落盘;
#[path = "sqlite_格位存储.rs"]
pub mod sqlite_格位存储;
#[path = "memory_格位存储.rs"]
pub mod memory_格位存储;

pub use 模型落盘::*;
pub use sqlite_格位存储::*;
pub use memory_格位存储::*;
