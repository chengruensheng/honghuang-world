//! §B.2.7 服务注册表（跨实例统一查询）。
//!
//! 简版：B.2.6 的 `pub trait 服务` + chajian_fu 的 TypeId 注册表已足够；
//! 本文件作为注册表的 trait 包装，未来 9 府全注册时统一查询。

/// 服务注册表 trait（让 chajian_fu 的 `Any + Send + Sync` 注册表有 trait 抽象）。
pub trait 服务注册表: Send + Sync {
    /// 注册服务（按 TypeId）
    fn 注册服务_任意(&mut self, 类型: std::any::TypeId, 实例: Box<dyn std::any::Any + Send + Sync>);
    /// 查找服务（按 TypeId）
    fn 查找服务_任意(&self, 类型: std::any::TypeId) -> Option<Box<dyn std::any::Any + Send + Sync>>;
}
