// 增量记录 —— 跨度增量记录：透出 tracing 跨度宏
#![allow(non_snake_case)]

pub use tracing::{span, trace_span, debug_span, info_span, warn_span, error_span, instrument};
