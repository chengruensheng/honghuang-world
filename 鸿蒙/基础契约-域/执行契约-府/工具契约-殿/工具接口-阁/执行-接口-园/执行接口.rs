use hm_contract::Component;
use hm_error::{Error, Result};

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
    /// 列出工作区内目录下的条目（一层，非递归），区分目录/文件，按名排序
    fn 列目录(&self, 路径: &str) -> Result<String>;
    /// 按 glob 模式（`*`/`**`/`?`）递归匹配工作区内文件，返回相对路径列表
    fn 按名找文件(&self, 模式: &str) -> Result<String>;
    /// 递归搜索工作区内文本文件内容，返回匹配行（路径:行号:内容）
    fn 搜索内容(&self, 关键词: &str) -> Result<String>;
    /// 将文件中唯一匹配的「旧」字符串替换为「新」（多处匹配报错要求更精确）
    fn 精确编辑(&self, 路径: &str, 旧: &str, 新: &str) -> Result<String>;
    /// 删除工作区内的文件（仅沙箱内清理临时/备份产物用）。
    /// 默认实现返回「不支持」，生产执行器（工作区沙箱）覆盖为真实删除，测试 mock 无需改动。
    fn 删除文件(&self, 路径: &str) -> Result<String> {
        Err(Error::Config(format!("此执行器不支持删除文件: {路径}")))
    }
}