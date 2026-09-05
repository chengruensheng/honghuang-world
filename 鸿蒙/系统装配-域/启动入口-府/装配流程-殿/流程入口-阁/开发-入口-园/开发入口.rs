use std::sync::Arc;
use hm_agent::智能体;
use hm_content::对话生成器;
use hm_execute::本地执行器;
use hm_execute_contract::开发事件;
use hm_http::开发执行台;

/// 装配智能体到 HTTP 受理台（仅在配置显式开启 run_dev_agent 时调用）。
///
/// LLM 凭据从环境变量注入（缺失则返回 Err，受理台保持未上线、接口 503 fail-loud）；
/// 执行器在工作区沙箱内运行，超时/输出上限取自配置。
/// 智能体事件回调转发到受理台事件流，供前端事件查询接口拉取。
/// Ctrl+C 由主程序整体退出接管，智能体中断改走停止接口（executor_ctrlc 语义退役）。
pub fn 装配开发受理台(
    受理台: &Arc<开发执行台>,
    工作区: &str,
    最大轮数: usize,
    executor_timeout_secs: u64,
    executor_max_output_bytes: u64,
) -> hm_error::Result<()> {
    let 对话器 = Arc::new(对话生成器::从环境()?);
    let 执行器 = Arc::new(本地执行器::new_with_limits(
        工作区,
        executor_timeout_secs,
        executor_max_output_bytes,
    ));

    let 转发台 = 受理台.clone();
    let 智能体 = 智能体::new(对话器, 执行器, 最大轮数)
        .设置事件回调(Arc::new(move |事件: &开发事件| 转发台.记录事件(事件)));

    受理台.装配(Arc::new(智能体), 工作区);
    Ok(())
}
