//! 府插件接口——每个府是一个插件，暴露 Service Definition。
//! 设计依据：层级结构-设计.md §三边界模型、§五鸿蒙地基模型。
//! 府作为插件单元，暴露 Service Definition（插件接口）。
//! 跨府引用经插件注册表查找，止步于 Service Definition。

use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;

use rizhi_fu::{error, info};

/// 府插件接口——每个府是一个插件。
///
/// 府实现此 trait，在启动时注册到插件注册表。
/// 名称/注入/应用三方法构成插件元信息与生命周期。
pub trait 府插件: Send + Sync {
    /// 插件名称（府名，如"识海承载-府"）。
    fn 名称(&self) -> &str;

    /// 依赖的其他插件名（注入，如["识海承载-府"]）。
    fn 注入(&self) -> Vec<&str>;

    /// 应用插件——注册到插件上下文，初始化插件资源。
    fn 应用(
        &self, ctx: &mut 插件上下文
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// 插件上下文——插件注册表 + 事件总线 + 状态共享的访问入口。
///
/// 设计依据：层级结构-设计.md §五鸿蒙地基模型。
/// 鸿蒙是插件注册表 + 事件总线 + 状态共享的提供者。
pub struct 插件上下文 {
    /// 插件注册表（府名 → 府插件实例）。
    注册表: RwLock<HashMap<String, Box<dyn 府插件>>>,
    /// 服务注册表（类型 ID → 服务实例）。
    ///
    /// 服务实例通常是 `Arc<dyn 服务trait>`，按 `TypeId` 索引。
    服务表: RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl 插件上下文 {
    /// 构造空插件上下文。
    pub fn 新() -> Self {
        Self {
            注册表: RwLock::new(HashMap::new()),
            服务表: RwLock::new(HashMap::new()),
        }
    }

    /// 注册插件——检查依赖是否已注册，注册到注册表。
    pub fn 注册(&mut self, 插件: Box<dyn 府插件>) -> Result<(), String> {
        let 名称 = 插件.名称().to_string();
        let 依赖们 = 插件.注入();
        // 先读后写：依赖检查与注册分两段持锁，避免一次写锁独占过久
        for 依赖 in &依赖们 {
            if !self.注册表.read().contains_key(*依赖) {
                error!("插件{}依赖{}未注册", 名称, 依赖);
                return Err(format!("插件{}依赖{}未注册", 名称, 依赖));
            }
        }
        self.注册表.write().insert(名称.clone(), 插件);
        info!("插件{}已注册", 名称);
        Ok(())
    }

    /// 查找插件——按名称查找已注册的插件。
    pub fn 查找(&self, 名称: &str) -> Option<String> {
        self.注册表.read().get(名称).map(|p| p.名称().to_string())
    }

    /// 已注册的插件名列表。
    pub fn 已注册(&self) -> Vec<String> {
        self.注册表.read().keys().cloned().collect()
    }

    /// 注册服务——将服务实例注册到服务表，按类型 ID 索引。
    ///
    /// 服务实例通常是 `Arc<dyn 服务trait>`，它实现了 `Any + Send + Sync + Clone`。
    pub fn 注册服务<T: Any + Send + Sync>(&mut self, 服务: T) -> Result<(), String> {
        let 类型id = TypeId::of::<T>();
        if self.服务表.read().contains_key(&类型id) {
            return Err(format!("服务类型{:?}已注册", 类型id));
        }
        self.服务表.write().insert(类型id, Box::new(服务));
        info!("服务类型{:?}已注册", 类型id);
        Ok(())
    }

    /// 查找服务——按类型 ID 查找服务实例，返回克隆。
    ///
    /// 调用方传入 `T = Arc<dyn 服务trait>`，查找成功返回 `Option<Arc<dyn 服务trait>>`。
    pub fn 查找服务<T: Any + Send + Sync + Clone>(&self) -> Option<T> {
        let 类型id = TypeId::of::<T>();
        let 服务表 = self.服务表.read();
        let 任意 = 服务表.get(&类型id)?;
        任意.downcast_ref::<T>().cloned()
    }
}

impl Default for 插件上下文 {
    fn default() -> Self {
        Self::新()
    }
}
