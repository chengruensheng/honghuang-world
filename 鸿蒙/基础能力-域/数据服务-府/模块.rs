#[path = "服务状态-殿/模块.rs"]
mod 服务状态_殿;
#[path = "请求处理-殿/模块.rs"]
mod 请求处理_殿;
#[path = "观测读取-殿/模块.rs"]
mod 观测读取_殿;
#[path = "接口路由-殿/模块.rs"]
mod 接口路由_殿;

pub use 服务状态_殿::*;
pub use 请求处理_殿::*;
pub use 观测读取_殿::*;
pub use 接口路由_殿::*;