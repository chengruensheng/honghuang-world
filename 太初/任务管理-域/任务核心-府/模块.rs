#[path = "任务模型-殿/模块.rs"]
mod 任务模型_殿;

#[path = "任务仓库-殿/模块.rs"]
mod 任务仓库_殿;

#[path = "任务看板-殿/模块.rs"]
mod 任务看板_殿;

#[path = "对外接口-殿/模块.rs"]
mod 对外接口_殿;

pub use 任务模型_殿::*;
pub use 任务仓库_殿::*;
pub use 任务看板_殿::*;
pub use 对外接口_殿::*;
pub use hm_cognition::AgentRole;