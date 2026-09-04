use hm_contract::Component;
use hm_error::Result;

/// 内容生成器契约：接入外部大模型，根据提示词生成内容。
///
/// 是 LLM 内容演化层的第一步（内容生成地基）。生产路径注入真实实现，
/// 测试路径注入 mock，无 key 时 fail-loud，绝不降级 mock。
pub trait 内容生成器: Component {
    /// 根据提示词生成内容；失败返回错误。
    fn 生成(&self, 提示词: String) -> Result<String>;
}