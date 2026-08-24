//! §B.2.6 通用服务 trait（9 府都实现）。
//!
//! 跨府 Service Definition 抽象 — chajian_fu 已有 Any + Send + Sync 机制，
//! 本 trait 标记服务名 + 版本，让跨府服务查找有类型 + 语义双重保障。

/// 通用服务 trait：所有 9 府服务都实现。
pub trait 服务: Send + Sync {
    /// 服务名（短，例如"识海承载.回想"）
    fn 名称(&self) -> &str;
    /// 服务版本（语义化）
    fn 版本(&self) -> &str;
}

// 实现示例：模型存储 实现 服务（之前未实现 — 现在补）
// 注：impl 自动满足 Send + Sync（模型存储 派生 Clone + Debug + PartialEq 都 Send）
// 完整实现留待 B.2.7 服务注册表时一并实装 — 此处只占位
