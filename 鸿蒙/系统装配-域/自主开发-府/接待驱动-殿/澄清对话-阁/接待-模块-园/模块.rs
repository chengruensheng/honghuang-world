#[path = "接待模块.rs"]
mod 接待模块;
pub use 接待模块::*;

#[cfg(test)]
#[path = "接待测试.rs"]
mod 接待测试;