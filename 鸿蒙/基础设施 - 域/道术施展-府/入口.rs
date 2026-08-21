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
/// 暴露执行任务方法，包任务-调度-殿的派遣执行真实逻辑。
/// 实例持有任务调度（内含模型配置+工作区根），方法不再传配置/工作区根参数。
pub trait 道术服务: Send + Sync {
    /// 执行任务——给定任务id/任务/背景/现状/涉及路径/设计方案/验收标准，调派遣执行返回回执。
    #[allow(clippy::too_many_arguments)]
    fn 执行任务(
        &self,
        任务id: &str,
        任务: &执行任务,
        背景: &str,
        现状: &str,
        涉及路径: &[String],
        设计方案: &str,
        验收标准: &str,
    ) -> Result<执行回执, String>;
}

/// 道术服务实例——持有任务调度，包任务-调度-殿真实逻辑。
struct 道术服务实例 {
    调度: 任务调度,
}

impl 道术服务 for 道术服务实例 {
    #[allow(clippy::too_many_arguments)]
    fn 执行任务(
        &self,
        任务id: &str,
        任务: &执行任务,
        背景: &str,
        现状: &str,
        涉及路径: &[String],
        设计方案: &str,
        验收标准: &str,
    ) -> Result<执行回执, String> {
        self.调度
            .派遣执行(任务id, 任务, 背景, 现状, 涉及路径, 设计方案, 验收标准)
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
        let 工作区 = shihai_fu::工作区::定位();
        let peizhi配置 =
            peizhi_fu::读模型配置(工作区.根路径().join(".env").to_str().unwrap_or(""));
        let 配置 = moxing_fu::模型配置 {
            密钥: peizhi配置.密钥,
            地址: peizhi配置.地址,
            模型: peizhi配置.模型,
        };
        let 调度 = 任务调度::新(配置, 工作区.根路径().to_path_buf());
        let 服务: std::sync::Arc<dyn 道术服务> = std::sync::Arc::new(道术服务实例 { 调度 });
        ctx.注册服务(服务)?;
        Ok(())
    }
}
