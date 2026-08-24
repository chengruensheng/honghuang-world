//! 命令操作-府（mingling_fu）—— 世界的操作台：命令解析 + 权限校验 + 观览查询 + 号令下达 + 输出呈现
//! 已接线鸿蒙六府：想法 → 运行一轮（天庭治理）→ 道术执行 → 定档进识海记忆。

#![allow(non_snake_case)]

#[path = "号令-下达-殿/模块.rs"]
pub mod 号令_下达_殿;
#[path = "命令-解析-殿/模块.rs"]
pub mod 命令_解析_殿;
#[path = "权限-校验-殿/模块.rs"]
pub mod 权限_校验_殿;
#[path = "观览-查询-殿/模块.rs"]
pub mod 观览_查询_殿;
#[path = "输出-呈现-殿/模块.rs"]
pub mod 输出_呈现_殿;

pub use 号令_下达_殿::*;
pub use 命令_解析_殿::*;
pub use 权限_校验_殿::*;
pub use 观览_查询_殿::*;
pub use 输出_呈现_殿::*;

/// crate 级测试共享锁：所有设置 WORLD_WORKSPACE_ROOT 环境变量或写入状态目录的测试共用此锁，
/// 避免并行测试互相覆盖环境变量或并发追加同一 jsonl 文件。
#[cfg(test)]
pub mod 测试设施 {
    pub static 工作区测试锁: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
