//! 类型声明：状态共享的键值类型。
//!
//! 状态共享按 `TypeId` 索引运行时状态，键值类型为辅助类型别名，
//! 便于调用方表达"状态键""状态值"语义，实际存取由 状态-存取-殿 完成。
//! 业务状态类型（当前想法id/当前要求id）也定义于此，供各府经状态共享读写。

use std::any::TypeId;
use std::sync::Arc;

/// 状态键：按类型 ID 索引，同一类型全局唯一。
pub type 状态键 = TypeId;

/// 状态值：类型擦除的共享状态实例，须 Send + Sync 以跨线程共享。
pub type 状态值 = Arc<dyn std::any::Any + Send + Sync>;

/// 由类型生成状态键——`状态键::of::<T>()` 等价于 `TypeId::of::<T>()`。
pub fn 状态键_of<T: 'static>() -> 状态键 {
    TypeId::of::<T>()
}

/// 当前想法id——主循环正在处理的想法id（运行时状态，供观览查询读取）。
/// 主政一轮开始时写入，观览查询经 取全局状态().读取::<当前想法id>() 取最新值。
#[derive(Clone, Debug)]
pub struct 当前想法id(pub String);

/// 当前要求id——主循环正在处理的要求id（运行时状态，供观览查询读取）。
/// 运行一轮开始时写入，观览查询经 取全局状态().读取::<当前要求id>() 取最新值。
#[derive(Clone, Debug)]
pub struct 当前要求id(pub String);
