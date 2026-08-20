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
        _ctx: &mut chajian_fu::插件上下文,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
