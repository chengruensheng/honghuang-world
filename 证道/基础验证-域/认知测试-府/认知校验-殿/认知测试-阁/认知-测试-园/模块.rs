#[path = "认知测试.rs"]
mod 认知测试;
#[path = "认知校验-测试.rs"]
mod 认知校验_测试;
#[path = "检索校验-测试.rs"]
mod 检索校验_测试;
#[path = "认知存储-测试.rs"]
mod 认知存储_测试;

pub use 认知测试::*;
pub use 认知校验_测试::*;
pub use 检索校验_测试::*;
pub use 认知存储_测试::*;
