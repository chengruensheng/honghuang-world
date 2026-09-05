use std::sync::Arc;

use crate::运行自主开发;


pub fn 启动() -> hm_error::Result<()> {
    let config = hm_config::default_config();
    let logger = hm_log::Logger::new(&config.log);

    // 生产路径装配五行：五引擎 + 信号总线，串成闭环
    let 装配 = hm_linkage::五行装配::装配();

    // 通过运行时容器统一管理生命周期：注册日志器并初始化（府可插拔）
    装配.容器.注册初始化(Arc::new(logger));
    装配.容器.初始化()?;

    tracing::info!("{} v{} 启动成功", config.app.name, config.app.version);
    tracing::info!(
        "五行相生装配完成：任务/迭代/记忆/规则/事件 五引擎与信号总线已串联（已注册组件 {:?}）",
        装配.容器.组件名()
    );

    // 启动自检仅在显式开启时运行（默认关闭，避免污染真实业务数据）；
    // 验证失败仅告警并继续启动，不得因失败导致程序退出
    if config.app.run_self_test {
        match hm_linkage::演示闭环(&装配) {
            Ok(状态) => tracing::info!("五行闭环验证通过：任务 {}、迭代 {}、记忆 {}、规则 {}、事件 {}", 状态.任务数, 状态.迭代数, 状态.记忆数, 状态.规则数, 状态.事件数),
            Err(e) => tracing::warn!("五行闭环验证失败（不影响启动）: {e}"),
        }
        match hm_linkage::演示相克(&装配) {
            Ok(克制) => tracing::info!("五行克制验证通过：事件去重 {}", 克制.去重事件),
            Err(e) => tracing::warn!("五行克制验证失败（不影响启动）: {e}"),
        }
    }

    // 自主开发智能体入口仅在显式开启且配置了任务时运行（默认关闭，避免意外调用外部模型）
    if config.app.run_dev_agent {
        if config.app.dev_task.trim().is_empty() {
            tracing::warn!("run_dev_agent 已开启但 dev_task 为空，跳过自主开发入口");
        } else {
            tracing::info!("进入自主开发智能体：工作区 {}，任务 {}", config.app.dev_workspace, config.app.dev_task);
            match 运行自主开发(
                &config.app.dev_workspace,
                &config.app.dev_task,
                config.app.dev_max_rounds,
                config.app.executor_timeout_secs,
                config.app.executor_max_output_bytes,
                config.app.executor_ctrlc,
            ) {
                Ok(答复) => tracing::info!("自主开发完成：{答复}"),
                Err(e) => tracing::warn!("自主开发失败（不影响启动）: {e}"),
            }
        }
    }

    Ok(())
}