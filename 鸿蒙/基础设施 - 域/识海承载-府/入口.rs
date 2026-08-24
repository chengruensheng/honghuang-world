// 识海承载 - 府 —— 世界之脑：给项目造记忆体，而不是数据库。
//
// 本质：代码记事实，LLM 补语义，人兜底填根。
// 三殿：识海-铭记-殿（编码）· 识海-纳藏-殿（存储）· 识海-回想-殿（检索）。
// 跨府引用止步本入口，只认根符号，不深链到殿/阁/园。

#[path = "识海-铭记-殿/模块.rs"]
pub mod 识海_铭记_殿;

#[path = "识海-纳藏-殿/模块.rs"]
pub mod 识海_纳藏_殿;

#[path = "识海-回想-殿/模块.rs"]
pub mod 识海_回想_殿;

pub use 识海_回想_殿::*;
pub use 识海_纳藏_殿::*;
pub use 识海_铭记_殿::*;

// 显式导出 workspace 成员缓存公共函数（C2，2026-08-22 入稿）：
// 供天庭治理-府 模板生成 跨府调用，止步 lib 根。glob 透出对中英混排函数名偶有缓存不命中，显式导出兜底。
pub use 识海_回想_殿::{
    工作区成员摘要, 府依赖, 读workspace成员缓存, 读workspace成员缓存在
};

// §B.2.3 统一世界错误
pub mod 世界错误 {
    pub use super::错误::{世界错误, 世界结果};
}
#[path = "错误.rs"]
mod 错误;

// §B.2.4 元数据：版本 + 兼容矩阵
pub mod 元数据 {
    pub use super::元数据_模块::{版本, 兼容性, 当前版本, 协商};
}
#[path = "元数据.rs"]
mod 元数据_模块;

// §B.2.5 统一目录路径（12 crate 共享 — 替代各自 fn 状态目录/观测目录）
pub use 目录_模块::{状态目录, 观测目录};
#[path = "目录.rs"]
mod 目录_模块;

// §B.2.6 通用服务 trait（9 府都实现 — 后续 B.2.7 注册表实装）
pub use 服务_模块::服务;
#[path = "服务.rs"]
mod 服务_模块;

// §B.2.7 服务注册表 trait（包装 chajian_fu Any + Send + Sync 注册表）
pub use 服务注册表_模块::服务注册表;
#[path = "服务注册表.rs"]
mod 服务注册表_模块;

// §B.2.8 jsonl schema 校验 + 截断修复
pub use jsonl_工具_模块::读_jsonl;
#[path = "jsonl_工具.rs"]
mod jsonl_工具_模块;

/// 识海服务——识海承载-府的 Service Definition。
///
/// 暴露回想（检索投影）和铐记（回填记忆）两方法，包回想-殿/纳藏-殿的真实逻辑。
/// 实例持有模型存储，方法不再传存储参数。
pub trait 识海服务: Send + Sync {
    /// 检索投影——给定角色/格位/字符预算/调用方层级，调元数据层化返回拼装的投影文本。
    fn 回想(
        &self,
        角色: &str,
        格位们: &[格位],
        预算字符: usize,
        调用方: 调用方层级,
    ) -> Result<String, String>;
    /// 回填记忆——给定格位名/内容/来源/录入者，写一条记录落盘。
    fn 铐记(
        &self, 格位名: &str, 内容: &str, 来源: &str, 录入者: &str
    ) -> Result<(), String>;
}

/// 识海服务实例——持有模型存储，包回想-殿/纳藏-殿真实逻辑。
struct 识海服务实例 {
    存储: 模型存储,
}

impl 识海服务 for 识海服务实例 {
    fn 回想(
        &self,
        角色: &str,
        格位们: &[格位],
        预算字符: usize,
        调用方: 调用方层级,
    ) -> Result<String, String> {
        元数据层化(&self.存储, 角色, 格位们, 预算字符, 调用方)
    }
    fn 铐记(
        &self, 格位名: &str, 内容: &str, 来源: &str, 录入者: &str
    ) -> Result<(), String> {
        self.存储.写记录(&记录::新(格位名, 内容, 来源, 录入者))
    }
}

/// 识海承载-府插件接口。
pub struct 识海插件;

impl chajian_fu::府插件 for 识海插件 {
    fn 名称(&self) -> &str {
        "识海承载-府"
    }

    fn 注入(&self) -> Vec<&str> {
        vec![]
    }

    fn 应用(
        &self,
        ctx: &mut chajian_fu::插件上下文,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let 工作区 = 工作区::定位();
        let 存储 = 模型存储::在工作区(&工作区);
        let 服务: std::sync::Arc<dyn 识海服务> = std::sync::Arc::new(识海服务实例 { 存储 });
        ctx.注册服务(服务)?;
        Ok(())
    }
}
