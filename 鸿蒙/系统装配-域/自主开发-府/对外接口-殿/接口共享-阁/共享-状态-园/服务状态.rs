//! 对外接口-殿/接口共享-阁/共享-状态-园：对外接口服务共享状态。
//!
//! 由启动入口从装配体取出字段注入，供本府 axum 处理器读取；
//! 记忆库以泛型参数承载领域模型，使系统装配层不反向依赖乾坤引擎的具体类型。

use std::sync::{Arc, Mutex};
use hm_domain_contract::记忆库契约;
use tc_task::TaskBoard;
use crate::{开发执行台, 看板驱动台, 道祖接待, 扫尾执行者};

/// 工作区重装配回调：接收新工作区路径，重新创建执行器并装配到执行台
pub type 重装配回调 = Arc<dyn Fn(&str) -> hm_error::Result<()> + Send + Sync>;

/// 对外接口服务共享状态：持有开发执行台、看板驱动台、任务看板、道祖接待与扫尾执行者的
/// 可克隆句柄；未装配项以 None 表达（对应接口 fail-loud 返回 503）。
pub struct 开发服务状态<M> {
    pub 任务看板: Arc<Mutex<TaskBoard>>,
    pub 开发执行台: Arc<开发执行台>,
    pub 看板驱动台: Arc<看板驱动台>,
    /// 道祖接待器（主控澄清会话，未装配时 None，对话接口返回未上线）
    pub 道祖接待: Option<Arc<Mutex<道祖接待>>>,
    /// 记忆库（道祖对齐需求沉淀记忆）
    pub 记忆库: Arc<Mutex<dyn 记忆库契约<M>>>,
    /// 扫尾执行者（太乙金仙清理后交付证据链的机器核验；未装配时 sweep 接口 503）
    pub 扫尾执行者: Option<Arc<扫尾执行者>>,
    /// 项目工作区根（相对或绝对路径；默认 ./，供 /api/files 清单与内容读取，顶栏可切换）
    pub 扫描根: Arc<Mutex<String>>,
    /// 开发受理台重装配回调（未装配时 None，工作区切换接口 503）
    pub 重装配工作区: Option<重装配回调>,
    /// 看板驱动台重装配回调：与 重装配工作区 同步切换，确保五层协作驱动器产出落同一工作区
    pub 重装配看板驱动: Option<重装配回调>,
    /// SSE 并发连接许可（所有 SSE 流共享，防止无限连接耗尽资源）
    pub sse信号量: Arc<tokio::sync::Semaphore>,
}

/// SSE 最大并发连接数（超出时新连接立即返回 429）
pub const SSE最大连接数: usize = 10;

/// 接口处理器状态别名：集中泛型约束，避免逐个处理器重复书写
pub type 接口状态<M> = axum::extract::State<开发服务状态<M>>;

/// 手写 Clone：`#[derive(Clone)]` 会误加 `M: Clone` 约束，而记忆库领域模型无需实现 Clone。
impl<M> Clone for 开发服务状态<M> {
    fn clone(&self) -> Self {
        开发服务状态 {
            任务看板: self.任务看板.clone(),
            开发执行台: self.开发执行台.clone(),
            看板驱动台: self.看板驱动台.clone(),
            道祖接待: self.道祖接待.clone(),
            记忆库: self.记忆库.clone(),
            扫尾执行者: self.扫尾执行者.clone(),
            扫描根: self.扫描根.clone(),
            重装配工作区: self.重装配工作区.clone(),
            重装配看板驱动: self.重装配看板驱动.clone(),
            sse信号量: self.sse信号量.clone(),
        }
    }
}

impl<M> 开发服务状态<M> {
    /// 构造对外接口服务状态（可选装配项默认 None，扫描根默认 ./）
    pub fn 新(
        任务看板: Arc<Mutex<TaskBoard>>,
        开发执行台: Arc<开发执行台>,
        看板驱动台: Arc<看板驱动台>,
        记忆库: Arc<Mutex<dyn 记忆库契约<M>>>,
    ) -> Self {
        开发服务状态 {
            任务看板,
            开发执行台,
            看板驱动台,
            道祖接待: None,
            记忆库,
            扫尾执行者: None,
            扫描根: Arc::new(Mutex::new(String::from("./"))),
            重装配工作区: None,
            重装配看板驱动: None,
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
