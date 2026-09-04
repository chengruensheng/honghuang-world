use std::sync::Arc;
use hm_agent::智能体;
use hm_content::对话生成器;
use hm_execute::本地执行器;

/// 运行自主开发智能体（仅在配置显式开启 run_dev_agent 时调用）。
///
/// 从环境变量注入 LLM 凭据，在工作区沙箱内执行，实时打印每一步工具调用与结果。
pub fn 运行自主开发(工作区: &str, 任务: &str, 最大轮数: usize) -> hm_error::Result<String> {
    let 对话器 = Arc::new(对话生成器::从环境()?);
    let 执行器 = Arc::new(本地执行器::new(工作区));
    let 智能体 = 智能体::new(对话器, 执行器, 最大轮数);
    智能体.运行(任务.to_string())
}