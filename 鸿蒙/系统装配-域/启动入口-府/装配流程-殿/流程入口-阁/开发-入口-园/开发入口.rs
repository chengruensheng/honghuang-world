use std::sync::Arc;
use std::sync::atomic::Ordering;
use hm_agent::智能体;
use hm_content::对话生成器;
use hm_execute::本地执行器;

/// 运行自主开发智能体（仅在配置显式开启 run_dev_agent 时调用）。
///
/// 从环境变量注入 LLM 凭据，在工作区沙箱内执行，实时打印每一步工具调用与结果。
/// `executor_timeout_secs` / `executor_max_output_bytes` 取自配置，替代执行器内部写死默认值。
/// 若 `executor_ctrlc` 为 true，则在智能体上挂载 Ctrl+C 回调：触发后通过中断句柄请求停止循环。
pub fn 运行自主开发(
    工作区: &str,
    任务: &str,
    最大轮数: usize,
    executor_timeout_secs: u64,
    executor_max_output_bytes: u64,
    executor_ctrlc: bool,
) -> hm_error::Result<String> {
    let 对话器 = Arc::new(对话生成器::从环境()?);
    let 执行器 = Arc::new(本地执行器::new_with_limits(
        工作区,
        executor_timeout_secs,
        executor_max_output_bytes,
    ));
    let 智能体 = 智能体::new(对话器, 执行器, 最大轮数);

    // Ctrl+C 接入：注册回调置位中断标志，智能体循环在每轮开头检查后自行停止；
    // 注册失败仅告警（非交互环境可能无控制台），不影响继续运行
    if executor_ctrlc {
        let 句柄 = 智能体.中断句柄();
        let 注册结果 = ctrlc::set_handler(move || {
            句柄.store(true, Ordering::SeqCst);
            tracing::warn!("收到 Ctrl+C，已请求停止智能体循环");
        });
        if let Err(e) = 注册结果 {
            tracing::warn!("注册 Ctrl+C 处理器失败（将继续运行，但 Ctrl+C 将无法停止循环）: {e}");
        }
    }

    智能体.运行(任务.to_string())
}
