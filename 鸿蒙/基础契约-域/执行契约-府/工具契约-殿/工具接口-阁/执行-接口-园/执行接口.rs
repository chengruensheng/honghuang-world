use hm_contract::Component;
use hm_error::Result;

/// 执行器契约：为自主智能体提供文件读写与命令执行能力。
///
/// 是自主开发智能体的"手脚"——大模型通过工具调用驱动它落盘、编译、测试。
/// 生产路径注入本地实现（工作区沙箱），测试路径注入 mock，无特权。
pub trait 执行器: Component {
    /// 读取工作区内文件的完整文本
    fn 读文件(&self, 路径: &str) -> Result<String>;
    /// 写入内容到工作区内文件（覆盖）
    fn 写文件(&self, 路径: &str, 内容: &str) -> Result<()>;
    /// 在工作区内运行命令，返回标准输出
    fn 运行命令(&self, 命令: &str) -> Result<String>;
}