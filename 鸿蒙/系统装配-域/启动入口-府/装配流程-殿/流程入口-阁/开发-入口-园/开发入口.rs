use std::sync::{Arc, Mutex};
use hm_agent::{智能体, 五层协作驱动器};
use hm_cognition::ContextManager;
use hm_content::对话生成器;
use hm_execute::本地执行器;
use hm_execute_contract::开发事件;
use hm_http::{开发执行台, 看板驱动台};
use tc_task::TaskBoard;

/// 装配智能体到 HTTP 受理台（仅在配置显式开启 run_dev_agent 时调用）。
///
/// LLM 凭据从环境变量注入（缺失则返回 Err，受理台保持未上线、接口 503 fail-loud）；
/// 执行器在工作区沙箱内运行，超时/输出上限取自配置。
/// 智能体事件回调转发到受理台事件流，供前端事件查询接口拉取。
/// Ctrl+C 由主程序整体退出接管，智能体中断走停止接口。
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

/// 装配看板驱动台：构造 五层协作驱动器（看板 + ContextManager + 对话器 + 执行器）并装配。
///
/// 与受理台共用同一组对话/执行配置但持有独立实例（两通道互不阻塞）；
/// ContextManager 持久化到 上下文路径（父目录不存在时自动创建）；
/// LLM 凭据缺失时返回 Err，驱动台保持未就绪、驱动接口 503 fail-loud。
pub fn 装配看板驱动台(
    驱动台: &Arc<看板驱动台>,
    看板: Arc<Mutex<TaskBoard>>,
    上下文路径: &str,
    工作区: &str,
    最大轮数: usize,
    executor_timeout_secs: u64,
    executor_max_output_bytes: u64,
) -> hm_error::Result<()> {
    if let Some(父) = std::path::Path::new(上下文路径).parent() {
        std::fs::create_dir_all(父).map_err(hm_error::Error::Io)?;
    }
    let 上下文 = Arc::new(Mutex::new(ContextManager::新(上下文路径)));
    let 对话器 = Arc::new(对话生成器::从环境()?);
    let 执行器 = Arc::new(本地执行器::new_with_limits(
        工作区,
        executor_timeout_secs,
        executor_max_output_bytes,
    ));
    let 驱动器 = Arc::new(五层协作驱动器::新(看板, 上下文, 对话器, 执行器, 最大轮数));
    驱动台.装配(驱动器);
    Ok(())
}
