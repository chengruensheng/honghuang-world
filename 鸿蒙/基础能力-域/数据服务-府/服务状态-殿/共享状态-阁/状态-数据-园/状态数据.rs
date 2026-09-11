use std::sync::{Arc, Mutex};
use hm_domain_contract::{任务仓库契约, 迭代日志契约, 记忆库契约, 规则库契约, 事件总线契约};
use hm_cognition::{图谱, 心智地图, 过程上下文};
use hm_content::LLM池;
use hm_agent::{道祖接待, 认知注入};
use hm_log::运行日志记录器;
use crate::{开发执行台, 看板驱动台, 扫尾执行者};
use tc_task::{Task, TaskStatus, TaskBoard};
use lj_iteration::{Iteration, Version};
use qk_memory::Memory;
use dy_rule::Rule;
use hd_event::Event;

/// 工作区重装配回调：接收新工作区路径，重新创建执行器并装配到执行台
pub type 重装配回调 = Arc<dyn Fn(&str) -> hm_error::Result<()> + Send + Sync>;

/// 数据服务共享状态：持有五引擎、认知三态、任务看板、日志记录器与开发执行台的可克隆句柄，
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
    pub 任务看板: Arc<Mutex<TaskBoard>>,
    pub 日志记录器: Arc<Mutex<运行日志记录器>>,
    pub 开发执行台: Arc<开发执行台>,
    pub 看板驱动台: Arc<看板驱动台>,
    /// 道祖接待器（主控澄清会话，未装配时 None，对话接口返回未上线）
    pub 道祖接待: Option<Arc<Mutex<道祖接待>>>,
    /// 三态认知注入（未装配时 None，认知问答接口返回未装配；装配后供检索决策/答复注入）
    pub 认知注入: Option<认知注入>,
    /// 商业级 LLM 池（未配置/未装配时 None，LLM 相关接口返回 未配置）
    pub llm池: Option<Arc<LLM池>>,
    pub 鉴权令牌: Option<String>,
    pub 重装配工作区: Option<重装配回调>,
    /// 看板驱动台重装配回调：与 重装配工作区 同步切换，确保五层协作驱动器产出落同一工作区
    pub 重装配看板驱动: Option<重装配回调>,
    /// 扫尾执行者（太乙金仙清理后交付证据链的机器核验；未装配时 sweep 接口 503）
    pub 扫尾执行者: Option<Arc<扫尾执行者>>,
    /// 项目工作区根（相对或绝对路径；默认 ./，供 /api/files 清单与内容读取，顶栏可切换）
    pub 扫描根: Arc<Mutex<String>>,
    /// SSE 并发连接许可（所有 SSE 流共享，防止无限连接耗尽资源）
    pub sse信号量: Arc<tokio::sync::Semaphore>,
}

/// SSE 最大并发连接数（超出时新连接立即返回 429）
pub const SSE最大连接数: usize = 10;

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
        任务看板: Arc<Mutex<TaskBoard>>,
        日志记录器: Arc<Mutex<运行日志记录器>>,
        开发执行台: Arc<开发执行台>,
        看板驱动台: Arc<看板驱动台>,
        llm池: Option<Arc<LLM池>>,
        鉴权令牌: Option<String>,
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
            任务看板,
            日志记录器,
            开发执行台,
            看板驱动台,
            道祖接待: None,
            认知注入: None,
            llm池,
            鉴权令牌,
            重装配工作区: None,
            重装配看板驱动: None,
            扫尾执行者: None,
            扫描根: Arc::new(Mutex::new(String::from("./"))),
            sse信号量: Arc::new(tokio::sync::Semaphore::new(SSE最大连接数)),
        }
    }

    /// 链式注入扫尾执行者（启动装配调用；测试/未装配时保持 None）
    pub fn 设置扫尾执行者(mut self, 执行者: Arc<扫尾执行者>) -> Self {
        self.扫尾执行者 = Some(执行者);
        self
    }

    /// 链式注入项目工作区根（启动装配调用；默认 ./）
    pub fn 设置扫描根(self, 根: String) -> Self {
        *self.扫描根.lock().expect("扫描根锁中毒") = 根;
        self
    }

    /// 链式覆盖 SSE 并发上限（启动装配按对外契约注入；0 = 不限制）
    ///
    /// 0 取值取 tokio 信号量允许的最大许可数：`Semaphore::new(usize::MAX)` 会 panic
    /// （tokio 上限为 `usize::MAX >> 3`），故以 `Semaphore::MAX_PERMITS` 表达「不限制」。
    pub fn 设置并发上限(mut self, 上限: usize) -> Self {
        let 数 = if 上限 == 0 { tokio::sync::Semaphore::MAX_PERMITS } else { 上限 };
        self.sse信号量 = Arc::new(tokio::sync::Semaphore::new(数));
        self
    }
}
