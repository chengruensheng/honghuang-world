//! 号令-下达-殿 模块桥：声明本殿下属八个阁并统一 re-export 给命令-解析-殿

#[path = "想法-投递-阁/模块.rs"]
pub mod 想法_投递_阁;
#[path = "要求-提审-阁/模块.rs"]
pub mod 要求_提审_阁;
#[path = "设计-审定-阁/模块.rs"]
pub mod 设计_审定_阁;
#[path = "验收-裁决-阁/模块.rs"]
pub mod 验收_裁决_阁;
#[path = "版本-定档-阁/模块.rs"]
pub mod 版本_定档_阁;
#[path = "记忆-回填-阁/模块.rs"]
pub mod 记忆_回填_阁;
#[path = "巡世-执行-阁/模块.rs"]
pub mod 巡世_执行_阁;
#[path = "对话-发言-阁/模块.rs"]
pub mod 对话_发言_阁;

pub use 想法_投递_阁::*;
pub use 要求_提审_阁::*;
pub use 设计_审定_阁::*;
pub use 验收_裁决_阁::*;
pub use 版本_定档_阁::*;
pub use 记忆_回填_阁::*;
pub use 巡世_执行_阁::*;
pub use 对话_发言_阁::*;