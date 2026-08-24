//! 世界：workspace 顶层包根。
//!
//! 依据：融合蓝图 §B.2.1 抽象统一（workspace 顶层 `pub mod 世界` 包根）。
//! 作用：12 个 crate 的核心 API 单一入口，避免下游直接依赖 13 个具体 crate。
//!
//! 当前为骨架：pub use 各府核心 API + Service trait 重新导出。
//! 后续 B.2.2-2.7 抽象 trait + 注册表会注入这里。

#![allow(non_snake_case)]

/// 世界 服务：跨府总入口（占位 — B.2.6 抽象 Service trait）
pub mod 服务占位 {
    /// 跨府服务查询（占位 — B.2.7 注册表）
    pub fn 占位查询() -> &'static str {
        "世界::服务占位 — 待 B.2.6 抽象 + B.2.7 注册表实装"
    }
}

// 各府核心 API 重新导出（§B.2.1）
pub use shihai_fu as 识海;
