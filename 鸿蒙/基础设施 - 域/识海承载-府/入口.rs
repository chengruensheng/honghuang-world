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

/// 识海服务——识海承载-府的 Service Definition。
///
/// 暴露回想（检索投影）和铐记（回填记忆）两方法。
/// 当前为占位实现，后续替换为调用回想-殿/铭记-殿的真实逻辑。
pub trait 识海服务: Send + Sync {
    /// 检索投影——给定查询，返回拼装的投影文本。
    fn 回想(&self, 查询: &str) -> Result<String, String>;
    /// 回填记忆——给定内容，记入识海。
    fn 铐记(&self, 内容: &str) -> Result<(), String>;
}

/// 识海服务占位实现——后续替换为真实逻辑。
struct 识海服务实例;

impl 识海服务 for 识海服务实例 {
    fn 回想(&self, _查询: &str) -> Result<String, String> {
        Err("识海服务.回想 尚未实现".to_string())
    }
    fn 铐记(&self, _内容: &str) -> Result<(), String> {
        Err("识海服务.铐记 尚未实现".to_string())
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
        let 服务: std::sync::Arc<dyn 识海服务> = std::sync::Arc::new(识海服务实例);
        ctx.注册服务(服务)?;
        Ok(())
    }
}
