// 兜底构建 —— 订阅器兜底构建：按配置装配并安装全局订阅器
#![allow(non_snake_case)]

use crate::{日志配置, 日志级别, 日志去向, 落地器, 渲染器};

use tracing_subscriber::util::SubscriberInitExt;

/// 按配置初始化全局订阅器；文件打不开时兜底退回仅控制台
pub fn 初始化(配置: &日志配置) {
    let 落地器 = 落地器::新建(&配置.去向).unwrap_or_else(|_| {
        eprintln!("日志文件打不开，退回仅控制台");
        落地器::新建(&日志去向::仅控制台).expect("仅控制台落地器必然可建")
    });

    let 渲染器 = match &配置.去向 {
        日志去向::仅控制台 => 渲染器::彩色(),
        _ => 渲染器::无色(),
    };

    let 订阅器 = tracing_subscriber::fmt()
        .with_max_level(级别过滤(配置.级别))
        .event_format(渲染器)
        .with_writer(落地器)
        .finish();

    // 已存在全局订阅器时忽略，保持幂等
    let _ = 订阅器.try_init();
}

/// 用默认配置初始化（信息级 + 仅控制台）
pub fn 初始化默认() {
    初始化(&日志配置::default());
}

fn 级别过滤(级别: 日志级别) -> tracing::Level {
    match 级别 {
        日志级别::追踪 => tracing::Level::TRACE,
        日志级别::调试 => tracing::Level::DEBUG,
        日志级别::信息 => tracing::Level::INFO,
        日志级别::警告 => tracing::Level::WARN,
        日志级别::错误 => tracing::Level::ERROR,
    }
}
