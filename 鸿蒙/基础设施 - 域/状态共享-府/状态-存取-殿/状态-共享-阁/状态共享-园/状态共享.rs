//! 状态共享——跨府运行时状态，按类型 ID 索引。
//!
//! 各府经本园读取/写入共享状态，不直接传参。写入按类型 ID 覆盖（后写覆盖先写），
//! 读取按类型 ID 查找 + 向下转型 + Clone 返回。全局状态共享用 `OnceLock` 存全局，
//! 启动时初始化一次，之后可读可写（`RwLock` 保护，线程安全）。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// 状态共享——跨府运行时状态，按类型 ID 索引。
pub struct 状态共享 {
    状态表: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl 状态共享 {
    pub fn 新() -> Self {
        Self {
            状态表: RwLock::new(HashMap::new()),
        }
    }

    /// 写入状态：按类型 ID 覆盖（后写覆盖先写）。
    pub fn 写入<T: Any + Send + Sync>(&self, 状态: T) -> Result<(), String> {
        let mut 表 = self.状态表.write().unwrap_or_else(|毒| 毒.into_inner());
        表.insert(TypeId::of::<T>(), Arc::new(状态));
        Ok(())
    }

    /// 读取状态：按类型 ID 查找 + 向下转型 + Clone 返回。
    pub fn 读取<T: Any + Send + Sync + Clone>(&self) -> Option<T> {
        let 表 = self.状态表.read().unwrap_or_else(|毒| 毒.into_inner());
        表.get(&TypeId::of::<T>())
            .and_then(|弧| 弧.downcast_ref::<T>())
            .cloned()
    }

    /// 移除状态：按类型 ID 积除。
    pub fn 移除<T: Any + Send + Sync>(&self) -> Result<(), String> {
        let mut 表 = self.状态表.write().unwrap_or_else(|毒| 毒.into_inner());
        表.remove(&TypeId::of::<T>());
        Ok(())
    }
}

impl Default for 状态共享 {
    fn default() -> Self {
        Self::新()
    }
}

/// 全局状态共享——`OnceLock` 存全局，启动时初始化，之后可读可写。
static 全局状态: OnceLock<状态共享> = OnceLock::new();

/// 初始化全局状态共享——若已初始化则返回既有实例。
pub fn 初始化全局状态() -> &'static 状态共享 {
    全局状态.get_or_init(状态共享::新)
}

/// 取全局状态共享——未初始化返回 `None`。
pub fn 取全局状态() -> Option<&'static 状态共享> {
    全局状态.get()
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 写入后读取返回正确值() {
        let 共享 = 状态共享::新();
        共享.写入(42_i32).unwrap();
        assert_eq!(共享.读取::<i32>(), Some(42));
    }

    #[test]
    fn 覆盖写入返回最新值() {
        let 共享 = 状态共享::新();
        共享.写入(1_i32).unwrap();
        共享.写入(2_i32).unwrap();
        assert_eq!(共享.读取::<i32>(), Some(2));
    }

    #[test]
    fn 读取未写入返回none() {
        let 共享 = 状态共享::新();
        assert_eq!(共享.读取::<i32>(), None);
    }

    #[test]
    fn 移除后读取返回none() {
        let 共享 = 状态共享::新();
        共享.写入(42_i32).unwrap();
        assert_eq!(共享.读取::<i32>(), Some(42));
        共享.移除::<i32>().unwrap();
        assert_eq!(共享.读取::<i32>(), None);
    }

    #[test]
    fn 不同类型各自独立() {
        let 共享 = 状态共享::新();
        共享.写入(42_i32).unwrap();
        共享.写入(String::from("洪荒")).unwrap();
        assert_eq!(共享.读取::<i32>(), Some(42));
        assert_eq!(共享.读取::<String>(), Some(String::from("洪荒")));
    }

    #[test]
    fn 全局状态写入读取正常() {
        let 共享 = 初始化全局状态();
        共享.写入(99_i64).unwrap();
        assert_eq!(共享.读取::<i64>(), Some(99));
        // 取全局状态应返回同一实例。
        let 再次取 = 取全局状态().expect("全局状态应已初始化");
        assert_eq!(再次取.读取::<i64>(), Some(99));
    }
}
