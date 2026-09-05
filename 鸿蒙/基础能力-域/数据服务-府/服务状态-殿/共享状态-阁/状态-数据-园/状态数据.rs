use std::sync::{Arc, Mutex};
use hm_domain_contract::{任务仓库契约, 迭代日志契约, 记忆库契约, 规则库契约, 事件总线契约};
use hm_cognition::{图谱, 心智地图, 过程上下文};
use hm_log::运行日志记录器;
use crate::开发执行台;
use tc_task::{Task, TaskStatus};
use lj_iteration::{Iteration, Version};
use qk_memory::Memory;
use dy_rule::Rule;
use hd_event::Event;

/// 数据服务共享状态：持有五引擎、认知三态、日志记录器与开发执行台的可克隆句柄，
/// 由启动入口从装配体取出字段注入，供 axum 处理器读取。
#[derive(Clone)]
pub struct 数据服务状态 {
    pub 任务仓库: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>>,
    pub 迭代日志: Arc<Mutex<dyn 迭代日志契约<Iteration, Version>>>,
    pub 记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>>,
    pub 规则库: Arc<Mutex<dyn 规则库契约<Rule>>>,
    pub 事件总线: Arc<Mutex<dyn 事件总线契约<Event>>>,
    pub 图谱: Arc<Mutex<图谱>>,
    pub 心智地图: Arc<Mutex<心智地图>>,
    pub 语境: Arc<Mutex<过程上下文>>,
    pub 日志记录器: Arc<Mutex<运行日志记录器>>,
    pub 开发执行台: Arc<开发执行台>,
}

impl 数据服务状态 {
    pub fn 新(
        任务仓库: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>>,
        迭代日志: Arc<Mutex<dyn 迭代日志契约<Iteration, Version>>>,
        记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>>,
        规则库: Arc<Mutex<dyn 规则库契约<Rule>>>,
        事件总线: Arc<Mutex<dyn 事件总线契约<Event>>>,
        图谱: Arc<Mutex<图谱>>,
        心智地图: Arc<Mutex<心智地图>>,
        语境: Arc<Mutex<过程上下文>>,
        日志记录器: Arc<Mutex<运行日志记录器>>,
        开发执行台: Arc<开发执行台>,
    ) -> Self {
        数据服务状态 {
            任务仓库,
            迭代日志,
            记忆库,
            规则库,
            事件总线,
            图谱,
            心智地图,
            语境,
            日志记录器,
            开发执行台,
        }
    }
}