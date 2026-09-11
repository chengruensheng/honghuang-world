use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),
    #[error("日志错误: {0}")]
    Log(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("模型错误: {0}")]
    模型(String),
    #[error("任务不存在: {0}")]
    任务不存在(u64),
    #[error("迭代不存在: {0}")]
    迭代不存在(u64),
    #[error("规则已存在: {0}")]
    规则已存在(String),
    #[error("规则不存在: {0}")]
    规则不存在(String),
    #[error("记忆不存在: {0}")]
    记忆不存在(u64),
    #[error("事件不存在: {0}")]
    事件不存在(u64),
    #[error("信号类型未知: {0}")]
    信号类型未知(String),
    #[error("状态流转非法: {0}")]
    状态流转非法(String),
    #[error("容量超限: {0}")]
    容量超限(String),
    #[error("序列化失败: {0}")]
    序列化(String),
    #[error("反序列化失败: {0}")]
    反序列化(String),
    #[error("命令超时: {0}")]
    命令超时(String),
    #[error("超出轮数: {0}")]
    超出轮数(String),
    #[error("退化循环熔断: {0}")]
    退化循环熔断(String),
    #[error("已中断: {0}")]
    中断(String),
    #[error("未知工具: {0}")]
    未知工具(String),
    #[error("缺少参数: {0}")]
    缺少参数(String),
    #[error("危险命令已拦截: {0}")]
    危险命令(String),
    #[error("格位已存在: {0}")]
    格位已存在(String),
    #[error("格位不存在: {0}")]
    格位不存在(String),
    #[error("上下文不存在: {0}")]
    上下文不存在(String),
    #[error("未知错误: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;