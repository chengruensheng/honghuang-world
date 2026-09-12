#[path = "事件定义-殿/模块.rs"]
mod 事件定义_殿;

#[path = "事件分发-殿/模块.rs"]
mod 事件分发_殿;

#[path = "对外接口-殿/模块.rs"]
mod 对外接口_殿;

pub use 事件定义_殿::*;
pub use 事件分发_殿::*;
pub use 对外接口_殿::*;