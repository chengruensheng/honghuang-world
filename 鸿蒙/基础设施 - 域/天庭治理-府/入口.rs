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

/// 读可操作规则（§4.2 规则7）：经 shihai_fu::注入规则 从 细则·解读 格位读链头最新规则文本，
/// 注入各提示角色首行。空串不注入（格位无有效记录时不影响现有行为）。
/// 提取至 crate 根供天层-主控-殿/智慧-主控-殿共享调用，消除两处重复定义。
pub(crate) fn 读可操作规则() -> String {
    let 工作区 = shihai_fu::工作区::定位();
    let 存储 = shihai_fu::模型存储::打开(工作区.格位目录());
    shihai_fu::注入规则(&存储, "细则·解读")
}

/// 天庭服务——天庭治理-府的 Service Definition。
///
/// 暴露调度要求、验收产物、归档版本三方法。调度要求包主政一轮真实逻辑；
/// 验收产物/归档版本阶段一占位（验收/归档在调度要求内完成）。
/// 实例持有模型配置/模型存储/任务调度，方法不再传这些依赖参数。
pub trait 天庭服务: Send + Sync {
    /// 调度要求——给定想法，调主政一轮推进想法流转，返回主政回执。
    fn 调度要求(&self, 想法: &想法) -> Result<主政回执, String>;
    /// 验收产物——阶段一占位：验收在调度要求内完成，独立验收待阶段二。
    fn 验收产物(&self, 要求id: &str) -> Result<(), String>;
    /// 归档版本——阶段一占位：归档在调度要求内完成，独立归档待阶段二。
    fn 归档版本(&self, 要求id: &str) -> Result<(), String>;
}

/// 天庭服务实例——持有模型配置/模型存储，包驱动-入口-殿真实逻辑。
struct 天庭服务实例 {
    配置: moxing_fu::模型配置,
    存储: shihai_fu::模型存储,
}

impl 天庭服务 for 天庭服务实例 {
    fn 调度要求(&self, 想法: &想法) -> Result<主政回执, String> {
        主政一轮(想法, &self.配置, &self.存储)
    }
    fn 验收产物(&self, _要求id: &str) -> Result<(), String> {
        Err("天庭服务.验收产物 阶段一占位：验收在调度要求内完成".to_string())
    }
    fn 归档版本(&self, _要求id: &str) -> Result<(), String> {
        Err("天庭服务.归档版本 阶段一占位：归档在调度要求内完成".to_string())
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
        let 工作区 = shihai_fu::工作区::定位();
        let peizhi配置 =
            peizhi_fu::读模型配置(工作区.根路径().join(".env").to_str().unwrap_or(""));
        let 配置 = moxing_fu::模型配置 {
            密钥: peizhi配置.密钥,
            地址: peizhi配置.地址,
            模型: peizhi配置.模型,
        };
        let 存储 = shihai_fu::模型存储::在工作区(&工作区);
        let 服务: std::sync::Arc<dyn 天庭服务> =
            std::sync::Arc::new(天庭服务实例 { 配置, 存储 });
        ctx.注册服务(服务)?;
        Ok(())
    }
}

/// crate 级测试共享锁：所有设置 WORLD_WORKSPACE_ROOT 环境变量的测试共用此锁，
/// 防并行测试竞态改写同一进程级环境变量（终裁.rs / 要求化.rs 原各自一把锁不互斥）。
#[cfg(test)]
pub(crate) static 工作区测试锁: std::sync::Mutex<()> = std::sync::Mutex::new(());
