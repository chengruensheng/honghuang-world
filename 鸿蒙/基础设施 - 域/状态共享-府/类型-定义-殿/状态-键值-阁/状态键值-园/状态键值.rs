//! 类型声明：状态共享的键值类型。
//!
//! 状态共享按 `TypeId` 索引运行时状态，键值类型为辅助类型别名，
//! 便于调用方表达"状态键""状态值"语义，实际存取由 状态-存取-殿 完成。

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
