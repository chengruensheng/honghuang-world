#[path = "契约.rs"]
pub mod 契约;
#[path = "实现.rs"]
pub mod 实现;
#[path = "测试.rs"]
#[cfg(test)]
mod 测试;

pub use 契约::后进先出栈;
pub use 实现::栈;
