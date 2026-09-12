#[path = "规则定义-殿/模块.rs"]
mod 规则定义_殿;

#[path = "规则收敛-殿/模块.rs"]
mod 规则收敛_殿;

#[path = "对外接口-殿/模块.rs"]
mod 对外接口_殿;

pub use 规则定义_殿::*;
pub use 规则收敛_殿::*;
pub use 对外接口_殿::*;