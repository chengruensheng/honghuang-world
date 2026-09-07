#[path = "任务测试.rs"]
mod 任务测试;

#[path = "任务标识测试.rs"]
mod 任务标识测试;

#[path = "定向回退测试.rs"]
mod 定向回退测试;

#[path = "依赖图测试.rs"]
mod 依赖图测试;

#[path = "五行层级标签测试.rs"]
mod 五行层级标签测试;

#[path = "扫尾测试.rs"]
mod 扫尾测试;

pub use 任务测试::*;
pub use 任务标识测试::*;
pub use 定向回退测试::*;
pub use 依赖图测试::*;
pub use 五行层级标签测试::*;
pub use 扫尾测试::*;