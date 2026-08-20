// 天庭治理 - 府 —— 世界之身：组织编排（管组织）。
//
// 本质：想法 → 要求 → 设计 → 验收 → 版本 → 进化。
// 七殿：类型-定义-殿 · 天层-主控-殿 · 智慧-主控-殿 · 队列-调度-殿 · 团队-调度-殿 · 版本-存档-殿 · 进化-主控-殿。
// 跨府引用止步本入口，只认根符号，不深链到殿/阁/园。

#[path = "类型-定义-殿/模块.rs"]
pub mod 类型_定义_殿;

#[path = "天层-主控-殿/模块.rs"]
pub mod 天层_主控_殿;

#[path = "智慧-主控-殿/模块.rs"]
pub mod 智慧_主控_殿;

#[path = "队列-调度-殿/模块.rs"]
pub mod 队列_调度_殿;

#[path = "团队-调度-殿/模块.rs"]
pub mod 团队_调度_殿;

#[path = "版本-存档-殿/模块.rs"]
pub mod 版本_存档_殿;

#[path = "进化-主控-殿/模块.rs"]
pub mod 进化_主控_殿;

#[path = "驱动-入口-殿/模块.rs"]
pub mod 驱动_入口_殿;

pub use 团队_调度_殿::*;
pub use 天层_主控_殿::*;
pub use 智慧_主控_殿::*;
pub use 版本_存档_殿::*;
pub use 类型_定义_殿::*;
pub use 进化_主控_殿::*;
pub use 队列_调度_殿::*;
pub use 驱动_入口_殿::*;

/// 天庭服务——天庭治理-府的 Service Definition。
///
/// 暴露调度要求、验收产物、归档版本三方法。当前为占位实现，后续替换为真实逻辑。
pub trait 天庭服务: Send + Sync {
    /// 调度要求——给定要求id，推进要求流转。
    fn 调度要求(&self, 要求id: &str) -> Result<(), String>;
    /// 验收产物——给定要求id，执行验收。
    fn 验收产物(&self, 要求id: &str) -> Result<(), String>;
    /// 归档版本——给定要求id，归档定档。
    fn 归档版本(&self, 要求id: &str) -> Result<(), String>;
}

/// 天庭服务占位实现——后续替换为真实逻辑。
struct 天庭服务实例;

impl 天庭服务 for 天庭服务实例 {
    fn 调度要求(&self, _要求id: &str) -> Result<(), String> {
        Err("天庭服务.调度要求 尚未实现".to_string())
    }
    fn 验收产物(&self, _要求id: &str) -> Result<(), String> {
        Err("天庭服务.验收产物 尚未实现".to_string())
    }
    fn 归档版本(&self, _要求id: &str) -> Result<(), String> {
        Err("天庭服务.归档版本 尚未实现".to_string())
    }
}

/// 天庭治理-府插件接口。
pub struct 天庭插件;

impl chajian_fu::府插件 for 天庭插件 {
    fn 名称(&self) -> &str {
        "天庭治理-府"
    }

    fn 注入(&self) -> Vec<&str> {
        vec!["识海承载-府", "道术施展-府"]
    }

    fn 应用(
        &self,
        ctx: &mut chajian_fu::插件上下文,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let 服务: std::sync::Arc<dyn 天庭服务> = std::sync::Arc::new(天庭服务实例);
        ctx.注册服务(服务)?;
        Ok(())
    }
}
