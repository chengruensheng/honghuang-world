//! 对外接口-殿/接口共享-阁/接口-定义-园：对外接口共用的请求/响应结构。

use serde::{Deserialize, Serialize};

/// 错误响应体（受理/驱动等写接口共用）
#[derive(Debug, Serialize)]
pub struct 受理错误响应 {
    pub 错误: String,
}

/// 事件游标查询参数（执行台/驱动事件增量拉取）
#[derive(Debug, Deserialize)]
pub struct 事件游标 {
    pub since: Option<u64>,
}
