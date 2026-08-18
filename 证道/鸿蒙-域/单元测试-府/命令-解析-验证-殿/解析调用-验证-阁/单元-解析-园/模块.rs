//! 解析调用夹具模块。

#[path = "契约.rs"]
pub mod 契约;
#[path = "fixture.rs"]
pub mod fixture;
pub use 契约::*;
pub use fixture::*;
