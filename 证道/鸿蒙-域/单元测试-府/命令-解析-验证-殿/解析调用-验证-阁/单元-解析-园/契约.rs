//! 命令解析测试的跨入口契约、错误分类与黄金 fixture。

#![allow(non_snake_case)]

use std::sync::{Arc, Mutex, MutexGuard};

/// 测试需保持穷举一致性的错误类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ErrorKind {
    成功 = 0,
    输入为空,
    参数格式错误,
    未知参数,
    参数重复,
    缺少参数,
    参数越界,
    工作区未就绪,
    状态目录不可用,
    状态锁中毒,
    命令不存在,
    权限不足,
    模型配置缺失,
    模型密钥缺失,
    模型地址缺失,
    模型调用失败,
    任务不存在,
    任务状态冲突,
    存储读取失败,
    存储写入失败,
    序列化失败,
    内部错误,
}

impl ErrorKind {
    /// 22 变体穷举清单；新增或改名时必须同步更新。
    pub const 清单: [Self; 22] = [
        Self::成功,
        Self::输入为空,
        Self::参数格式错误,
        Self::未知参数,
        Self::参数重复,
        Self::缺少参数,
        Self::参数越界,
        Self::工作区未就绪,
        Self::状态目录不可用,
        Self::状态锁中毒,
        Self::命令不存在,
        Self::权限不足,
        Self::模型配置缺失,
        Self::模型密钥缺失,
        Self::模型地址缺失,
        Self::模型调用失败,
        Self::任务不存在,
        Self::任务状态冲突,
        Self::存储读取失败,
        Self::存储写入失败,
        Self::序列化失败,
        Self::内部错误,
    ];
}

/// 错误处理统一三态。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum 三态判据 {
    #[default]
    通过,
    降级,
    阻断,
}

/// 所有命令入口都应实现的跨入口不变量。
pub trait CrossEntryInvariant {
    type 状态;
    fn 校验(&self, 状态: &Self::状态) -> 三态判据;
    fn 错误三态(&self, 错误: ErrorKind) -> 三态判据;
}

pub fn 错误三态(错误: ErrorKind) -> 三态判据 {
    match 错误 {
        ErrorKind::成功 => 三态判据::通过,
        ErrorKind::输入为空
        | ErrorKind::参数格式错误
        | ErrorKind::未知参数
        | ErrorKind::参数重复
        | ErrorKind::缺少参数
        | ErrorKind::参数越界
        | ErrorKind::命令不存在
        | ErrorKind::模型配置缺失
        | ErrorKind::模型密钥缺失
        | ErrorKind::任务不存在
        | ErrorKind::任务状态冲突 => 三态判据::降级,
        ErrorKind::工作区未就绪
        | ErrorKind::状态目录不可用
        | ErrorKind::状态锁中毒
        | ErrorKind::权限不足
        | ErrorKind::模型地址缺失
        | ErrorKind::模型调用失败
        | ErrorKind::存储读取失败
        | ErrorKind::存储写入失败
        | ErrorKind::序列化失败
        | ErrorKind::内部错误 => 三态判据::阻断,
    }
}

/// 黄金错误判据在编译期嵌入。
pub const 错误判据文档: &str = include_str!("errors.md");

/// 跨入口共享状态模型。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SharedState {
    pub 活动入口: Vec<String>,
    pub 已处理数: u64,
    pub 待处理数: usize,
    pub 版本: u32,
    pub 上次错误: Option<ErrorKind>,
}

/// 可由所有测试入口共享的测试环境。
#[derive(Clone, Default)]
pub struct TestEnv(Arc<Mutex<SharedState>>);

impl TestEnv {
    pub fn 新() -> Self {
        Self::default()
    }

    pub fn 共享存储(&self) -> Arc<Mutex<SharedState>> {
        Arc::clone(&self.0)
    }

    pub fn 加锁(&self) -> MutexGuard<'_, SharedState> {
        self.0.lock().unwrap_or_else(|毒锁| 毒锁.into_inner())
    }

    pub fn 更新(&self, 更新: impl FnOnce(&mut SharedState)) {
        let mut 状态 = self.加锁();
        更新(&mut 状态);
    }
}

impl CrossEntryInvariant for TestEnv {
    type 状态 = SharedState;

    fn 校验(&self, 状态: &Self::状态) -> 三态判据 {
        match self.0.lock() {
            Ok(实际状态) if 实际状态.as_ref() == 状态 => 三态判据::通过,
            Ok(_) | Err(_) => 三态判据::阻断,
        }
    }

    fn 错误三态(&self, 错误: ErrorKind) -> 三态判据 {
        错误三态(错误)
    }
}

/// 固定默认值快照；环境变量只允许在调用边界覆盖。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct 默认值快照 {
    pub 工作线程数: usize,
    pub 空轮询间隔秒: u64,
    pub 异常重试间隔秒: u64,
    pub 对话历史条数: usize,
    pub 日志摘要长度: usize,
    pub 锁失败重试次数: u8,
}

impl Default for 默认值快照 {
    fn default() -> Self {
        Self {
            工作线程数: 4,
            空轮询间隔秒: 2,
            异常重试间隔秒: 5,
            对话历史条数: 20,
            日志摘要长度: 120,
            锁失败重试次数: 1,
        }
    }
}

pub fn 默认快照() -> 默认值快照 {
    默认值快照::default()
}

#[cfg(test)]
mod 测试 {
    use super::{默认快照, 错误判据文档, 默认值快照, ErrorKind, SharedState, TestEnv, 三态判据, CrossEntryInvariant};

    #[test]
    fn 二十二种错误均有三态() {
        assert_eq!(ErrorKind::清单.len(), 22);
        let 状态们 = [三态判据::通过, 三态判据::降级, 三态判据::阻断];
        let mut 已覆盖 = [false; 3];
        for 错误 in ErrorKind::清单 {
            let 状态 = super::错误三态(错误);
            已覆盖[状态们.iter().position(|候选| *候选 == 状态).unwrap()] = true;
        }
        assert!(已覆盖.into_iter().all(|存在| 存在));
    }

    #[test]
    fn 黄金判据由编译期嵌入() {
        assert!(错误判据文档.contains("## 通过"));
        assert!(错误判据文档.contains("## 降级"));
        assert!(错误判据文档.contains("## 阻断"));
    }

    #[test]
    fn 跨入口共享状态满足统一不变量() {
        let 环境 = TestEnv::新();
        let 状态 = SharedState { 已处理数: 1, 待处理数: 2, ..SharedState::default() };
        环境.更新(|共享状态| *共享状态 = 状态.clone());
        assert_eq!(环境.校验(&状态), 三态判据::通过);
        环境.更新(|共享状态| 共享状态.已处理数 = 2);
        assert_eq!(环境.校验(&状态), 三态判据::阻断);
    }

    #[test]
    fn 默认值快照固定() {
        let 预期 = 默认值快照 {
            工作线程数: 4,
            空轮询间隔秒: 2,
            异常重试间隔秒: 5,
            对话历史条数: 20,
            日志摘要长度: 120,
            锁失败重试次数: 1,
        };
        assert_eq!(默认快照(), 预期);
    }
}
