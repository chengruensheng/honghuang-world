// 道术施展 - 府 —— 世界之身：任务执行（角色工作流引擎 + 手脚工具集）。
//
// 本质：每个层级角色按自己的道术跑 L1-L4 工作流，只执行、不越级组织。
// 五殿：类型-定义-殿 · 角色-卡册-殿 · 工作流-编排-殿 · 任务-调度-殿 · 手脚-施展-殿。
// 手脚-施展-殿 = 智能体手脚架工具集（文件 / 目录 / 检索 / 命令，不含网络搜索）。
// 跨府引用止步本入口，只认根符号，不深链到殿/阁/园。

#[path = "类型-定义-殿/模块.rs"]
pub mod 类型_定义_殿;

#[path = "角色-卡册-殿/模块.rs"]
pub mod 角色_卡册_殿;

#[path = "工作流-编排-殿/模块.rs"]
pub mod 工作流_编排_殿;

#[path = "任务-调度-殿/模块.rs"]
pub mod 任务_调度_殿;

#[path = "手脚-施展-殿/模块.rs"]
pub mod 手脚_施展_殿;

pub use 任务_调度_殿::*;
pub use 工作流_编排_殿::*;
pub use 手脚_施展_殿::*;
pub use 类型_定义_殿::*;
pub use 角色_卡册_殿::*;

/// 道术服务——道术施展-府的 Service Definition。
///
/// 暴露执行任务方法。当前为占位实现，后续替换为调用角色工作流引擎的真实逻辑。
pub trait 道术服务: Send + Sync {
    /// 执行任务——给定任务描述，返回执行结果。
    fn 执行任务(&self, 任务: &str) -> Result<String, String>;
}

/// 道术服务占位实现——后续替换为真实逻辑。
struct 道术服务实例;

impl 道术服务 for 道术服务实例 {
    fn 执行任务(&self, _任务: &str) -> Result<String, String> {
        Err("道术服务.执行任务 尚未实现".to_string())
    }
}

/// 道术施展-府插件接口。
pub struct 道术插件;

impl chajian_fu::府插件 for 道术插件 {
    fn 名称(&self) -> &str {
        "道术施展-府"
    }

    fn 注入(&self) -> Vec<&str> {
        vec!["识海承载-府"]
    }

    fn 应用(
        &self,
        ctx: &mut chajian_fu::插件上下文,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let 服务: std::sync::Arc<dyn 道术服务> = std::sync::Arc::new(道术服务实例);
        ctx.注册服务(服务)?;
        Ok(())
    }
}
