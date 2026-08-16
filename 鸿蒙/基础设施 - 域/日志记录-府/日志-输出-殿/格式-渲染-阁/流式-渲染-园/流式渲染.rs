// 流式渲染 —— 日志行流式渲染：时间 - 级别 - 模块 - 消息
#![allow(non_snake_case)]

use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use time::OffsetDateTime;

/// 渲染器：把事件渲染成「时间 - 级别 - 模块 - 消息」一行
pub struct 渲染器 {
    加色: bool,
}

impl 渲染器 {
    /// 彩色渲染（控制台用）
    pub fn 彩色() -> Self {
        渲染器 { 加色: true }
    }

    /// 无色渲染（文件用，避免 ANSI 码污染）
    pub fn 无色() -> Self {
        渲染器 { 加色: false }
    }
}

impl<S, N> FormatEvent<S, N> for 渲染器
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let 元数据 = event.metadata();
        write!(
            writer,
            "{} - {} - {} - ",
            当前时间(),
            级别文本(元数据.level(), self.加色),
            元数据.target()
        )?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// 级别转中文（TRACE → 追踪 等）
fn 级别中文(级别: &tracing::Level) -> &'static str {
    match 级别.as_str() {
        "TRACE" => "追踪",
        "DEBUG" => "调试",
        "INFO" => "信息",
        "WARN" => "警告",
        "ERROR" => "错误",
        _ => "信息",
    }
}

/// 级别颜色码（ANSI）
fn 级别颜色(级别: &tracing::Level) -> &'static str {
    match 级别.as_str() {
        "TRACE" => "\x1b[90m",
        "DEBUG" => "\x1b[34m",
        "INFO" => "\x1b[32m",
        "WARN" => "\x1b[33m",
        "ERROR" => "\x1b[31m",
        _ => "\x1b[0m",
    }
}

/// 级别文本：中文 + 可选颜色
fn 级别文本(级别: &tracing::Level, 加色: bool) -> String {
    let 中文 = 级别中文(级别);
    if 加色 {
        format!("{}{}\x1b[0m", 级别颜色(级别), 中文)
    } else {
        中文.to_string()
    }
}

/// 当前时间：YYYY-MM-DD HH:MM:SS.mmm
fn 当前时间() -> String {
    let 本地 = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        本地.year(),
        本地.month() as u8,
        本地.day(),
        本地.hour(),
        本地.minute(),
        本地.second(),
        本地.millisecond(),
    )
}
