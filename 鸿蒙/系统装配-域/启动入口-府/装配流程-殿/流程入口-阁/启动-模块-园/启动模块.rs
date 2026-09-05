use std::sync::Arc;

use crate::装配开发受理台;


pub fn 启动() -> hm_error::Result<()> {
    let config = hm_config::default_config();
    let logger = hm_log::Logger::new(&config.log);

    // 持久化目录：空字符串 = 纯内存（不持久化）
    let 持久化目录 = if config.persistence.dir.trim().is_empty() {
        None
    } else {
        Some(config.persistence.dir.clone())
    };

    // 生产路径装配五行：五引擎 + 信号总线 + 认知三态 + 日志记录器，串成闭环
    let 装配 = hm_linkage::五行装配::装配带持久化目录(持久化目录);

    // 通过运行时容器统一管理生命周期：注册日志器并初始化（府可插拔）
    装配.容器.注册初始化(Arc::new(logger));
    装配.容器.初始化()?;

    tracing::info!("{} v{} 启动成功", config.app.name, config.app.version);
    tracing::info!(
        "五行相生装配完成：任务/迭代/记忆/规则/事件 五引擎与信号总线已串联（已注册组件 {:?}）",
        装配.容器.组件名()
    );

    // 数据服务状态：五引擎 + 认知三态 + 日志记录器 + 开发执行台
    let 数据状态 = hm_http::数据服务状态::新(
        装配.任务仓库.clone(),
        装配.迭代日志.clone(),
        装配.记忆库.clone(),
        装配.规则库.clone(),
        装配.事件总线.clone(),
        装配.图谱.clone(),
        装配.心智地图.clone(),
        装配.语境.clone(),
        装配.日志记录器.clone(),
        Arc::new(hm_http::开发执行台::新()),
    );

    // 自主开发智能体上线：run_dev_agent=true 时装配到 HTTP 受理台（默认关闭）。
    // LLM key 缺失仅告警，受理台保持未上线（受理接口 503），不影响数据服务；
    // dev_task 非空时自动受理为首个任务
    if config.app.run_dev_agent {
        match 装配开发受理台(
            &数据状态.开发执行台,
            &config.app.dev_workspace,
            config.app.dev_max_rounds,
            config.app.executor_timeout_secs,
            config.app.executor_max_output_bytes,
        ) {
            Ok(()) => {
                tracing::info!("自主开发智能体已上线（HTTP 受理模式，工作区 {}）", config.app.dev_workspace);
                if !config.app.dev_task.trim().is_empty() {
                    match hm_http::受理开发任务(&数据状态, config.app.dev_task.clone()) {
                        Ok(id) => tracing::info!("已自动受理初始任务 id={id}：{}", config.app.dev_task),
                        Err(失败) => tracing::warn!("初始任务受理失败（不影响启动）: {失败:?}"),
                    }
                }
            }
            Err(e) => tracing::warn!("智能体装配失败，HTTP 受理不可用（不影响启动）: {e}"),
        }
    }

    // 启动数据服务：axum 同源托管前端静态文件 + API（独立线程，失败仅告警不影响主程序）
    hm_http::启动数据服务(数据状态, config.http.port, config.http.static_dir.clone());

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


    Ok(())
}
