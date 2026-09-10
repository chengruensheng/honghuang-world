use std::sync::{Arc, Mutex};
use hm_agent::{智能体, 五层协作驱动器, 认知注入};
use hm_cognition::ContextManager;
use hm_content_contract::工具对话器;
use hm_execute::本地执行器;
use hm_execute_contract::开发事件;
use hm_http::{开发执行台, 看板驱动台, 驱动过程事件};
use tc_task::TaskBoard;

/// 装配智能体到 HTTP 受理台（仅在配置显式开启 run_dev_agent 时调用）。
///
/// 对话器为按智能体绑定的 LLM 视图（Arc<dyn 工具对话器>，绑定/全局选择动态解析）；
/// 未装配（未配置供应商且环境密钥缺失）返回 Err，受理台保持未上线、接口 503 fail-loud；
/// 执行器在工作区沙箱内运行，超时/输出上限取自配置。
/// 智能体事件回调转发到受理台事件流，供前端事件查询接口拉取。
/// Ctrl+C 由主程序整体退出接管，智能体中断走停止接口。
pub fn 装配开发受理台(
    受理台: &Arc<开发执行台>,
    工作区: &str,
    最大轮数: usize,
    executor_timeout_secs: u64,
    executor_max_output_bytes: u64,
    对话器: Option<Arc<dyn 工具对话器>>,
) -> hm_error::Result<()> {
    let 对话器: Arc<dyn 工具对话器> = 对话器.ok_or_else(|| {
        hm_error::Error::Config("LLM 池未装配（未配置供应商且环境密钥缺失）".into())
    })?;
    let 执行器 = Arc::new(本地执行器::new_with_limits(
        工作区,
        executor_timeout_secs,
        executor_max_output_bytes,
    ));

    let 转发台 = 受理台.clone();
    let 智能体 = 智能体::new(对话器, 执行器, 最大轮数)
        .设置工作区(工作区.to_string())
        .设置事件回调(Arc::new(move |事件: &开发事件| 转发台.记录事件(事件)));

    受理台.装配(Arc::new(智能体), 工作区);
    Ok(())
}

/// 装配看板驱动台：构造 五层协作驱动器（看板 + ContextManager + 对话器 + 执行器）并装配。
///
/// 与受理台共用按智能体绑定的 LLM 视图（「执行」身份，独立模型绑定动态解析）；
/// ContextManager 持久化到 上下文路径（父目录不存在时自动创建）；
/// 未装配时返回 Err，驱动台保持未就绪、驱动接口 503 fail-loud。
pub fn 装配看板驱动台(
    驱动台: &Arc<看板驱动台>,
    看板: Arc<Mutex<TaskBoard>>,
    上下文路径: &str,
    工作区: &str,
    最大轮数: usize,
    executor_timeout_secs: u64,
    executor_max_output_bytes: u64,
    认知: Option<认知注入>,
    对话器: Option<Arc<dyn 工具对话器>>,
    会话目录: &str,
) -> hm_error::Result<()> {
    let 对话器: Arc<dyn 工具对话器> = 对话器.ok_or_else(|| {
        hm_error::Error::Config("LLM 池未装配（未配置供应商且环境密钥缺失）".into())
    })?;
    if let Some(父) = std::path::Path::new(上下文路径).parent() {
        std::fs::create_dir_all(父).map_err(hm_error::Error::Io)?;
    }
    let 上下文 = Arc::new(Mutex::new(ContextManager::新(上下文路径)));
    let 执行器 = Arc::new(本地执行器::new_with_limits(
        工作区,
        executor_timeout_secs,
        executor_max_output_bytes,
    ));
    let mut 驱动器 = 五层协作驱动器::新(看板, 上下文, 对话器, 执行器, 最大轮数);
    驱动器 = 驱动器.设置工作区(工作区.to_string());
    if let Some(注入) = 认知 {
        驱动器 = 驱动器.装配认知(注入);
    }
    // 注入过程事件回调：把智能体循环每步事件转为驱动过程事件，记录到驱动台供前端轮询
    let 驱动台克隆 = 驱动台.clone();
    驱动器 = 驱动器.设置过程回调(Arc::new(move |任务id, 角色名, 事件| {
        驱动台克隆.记录过程事件(&驱动过程事件 {
            序号: 0,
            任务id: Some(任务id),
            角色: Some(角色名.to_string()),
            轮次: 事件.轮次,
            类型: match 事件.类型 {
                hm_execute_contract::开发事件类型::思考 => "思考".to_string(),
                hm_execute_contract::开发事件类型::工具调用 => "工具调用".to_string(),
                hm_execute_contract::开发事件类型::工具结果 => "工具结果".to_string(),
                hm_execute_contract::开发事件类型::任务答复 => "任务答复".to_string(),
            },
            工具名: 事件.工具名.clone(),
            内容: 事件.内容.clone(),
            时间: hm_contract::当前时间戳(),
        });
    }));
    // 注入运行检查点回调：每轮末把智能体消息快照+任务清单快照落盘到当前会话，供 Resume/Fork 断点续跑
    let 驱动台检查点 = 驱动台.clone();
    驱动器 = 驱动器.设置检查点回调(Arc::new(move |任务id, 角色名, 轮次, 阶段提示, 消息, 清单| {
        驱动台检查点.写入检查点(任务id, 角色名, 轮次, 阶段提示, 消息, 清单);
    }));
    驱动台.装配会话存储(会话目录);
    驱动台.装配(Arc::new(驱动器));
    Ok(())
}
