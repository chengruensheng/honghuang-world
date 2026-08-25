// 工具 - 循环 - 园：function calling 工具执行循环
// 工具清单 是 manifest 注册表模块，需 pub 导出供 daoshu_fu::工具清单::清单() 访问。
#[path = "工具清单.rs"]
pub mod 工具清单;

#[path = "工具循环.rs"]
mod 工具循环;

#[path = "工具定义.rs"]
mod 工具定义;

#[path = "工具执行.rs"]
mod 工具执行;

#[cfg(test)]
#[path = "测试.rs"]
mod 测试;

pub use 工具定义::*;
pub use 工具循环::*;
pub use 工具执行::*;
pub use 工具清单::*;
