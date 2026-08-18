//! 命令解析的共享默认环境夹具。

use super::契约::{CrossEntryInvariant, ErrorKind, SharedState, TestEnv, 三态判据};

/// 默认跨入口测试环境快照。
pub fn 环境快照() -> SharedState {
    SharedState::default()
}

/// 通过统一入口执行跨入口不变量检查。
pub fn 校验环境(环境: &TestEnv, 快照: &SharedState) -> 三态判据 {
    环境.校验(快照)
}

/// 稳定记录统一错误类别。
pub fn 统一错误(错误: ErrorKind) -> 三态判据 {
    CrossEntryInvariant::错误三态(环境快照_环境(), 错误)
}

fn 环境快照_环境() -> TestEnv {
    TestEnv::新()
}
