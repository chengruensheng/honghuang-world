use std::sync::{Arc, Mutex};
use hm_contract::Component;
use hm_signal::{信号总线, 信号类型, 载荷键, 信号};
use hm_signal_bus::内存信号总线;
use hm_domain_contract::{任务仓库契约, 迭代日志契约, 记忆库契约, 规则库契约, 事件总线契约};
use hm_container::组件容器;
use hm_cognition::{图谱, 心智地图, 过程上下文};
use hm_log::运行日志记录器;
use crate::{克制金克木, 克制木克土, 克制土克水, 克制水克火, 克制火克金};
use tc_task::{Task, TaskStatus, TaskStore};
use lj_iteration::{Iteration, Version, IterationLog, 迭代状态};
use qk_memory::{Memory, MemoryStore};
use dy_rule::{Rule, RuleSet};
use hd_event::{Event, EventBus};

/// 火生土写入记忆时的默认标签（迭代产出的经验），跨桥接与自检共用
pub const 迭代产出标签: &str = "迭代产出";
/// 五行引擎默认容量上限（相克：过盛才约束新增）
const 默认容量上限: usize = 100;
/// 水生木生成任务标题的前缀
const 事件驱动前缀: &str = "事件驱动: ";
/// 木生火生成迭代变更说明的前缀
const 任务完成前缀: &str = "任务完成: ";
/// 土生金生成规则的优先级（记忆生成 = 低优先级，人工添加可传更高）
const 记忆生成规则优先级: u32 = 1;
/// 信号转日志的默认样式（动作类）
const 信号日志样式: &str = "act";
/// 五行引擎持久化文件名（相对持久化目录，跨引擎统一避免魔法字符串）
const 持久化文件_任务: &str = "任务.toml";
const 持久化文件_迭代: &str = "迭代.toml";
const 持久化文件_记忆: &str = "记忆.toml";
const 持久化文件_规则: &str = "规则.toml";
const 持久化文件_事件: &str = "事件.toml";

/// 五行装配：持有五引擎（领域契约 trait 对象）、认知三态、日志记录器与信号总线的完整装配体
///
/// 府可插拔：各引擎字段均为 `Arc<Mutex<dyn 领域契约>>`，
/// 生产路径装配真实实现，测试路径注入 mock，核心无特权。
pub struct 五行装配 {
    pub 容器: Arc<组件容器>,
    pub 信号总线: Arc<dyn 信号总线>,
    pub 任务仓库: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>>,
    pub 迭代日志: Arc<Mutex<dyn 迭代日志契约<Iteration, Version>>>,
    pub 记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>>,
    pub 规则库: Arc<Mutex<dyn 规则库契约<Rule>>>,
    pub 事件总线: Arc<Mutex<dyn 事件总线契约<Event>>>,
    pub 图谱: Arc<Mutex<图谱>>,
    pub 心智地图: Arc<Mutex<心智地图>>,
    pub 语境: Arc<Mutex<过程上下文>>,
    pub 日志记录器: Arc<Mutex<运行日志记录器>>,
}

impl 五行装配 {
    /// 装配五行（生产路径）：默认容量上限，创建五引擎 + 信号总线，串成闭环
    pub fn 装配() -> Self {
        Self::装配带上限(默认容量上限)
    }

    /// 装配五行并指定容量上限（相克约束：过盛才约束新增，而非删除已有）
    pub fn 装配带上限(容量上限: usize) -> Self {
        Self::装配带持久化(容量上限, None, None)
    }

    /// 装配五行并指定持久化目录（容量上限取默认值）：历史文件存在则加载，否则新建
    pub fn 装配带持久化目录(持久化目录: Option<String>) -> Self {
        Self::装配带持久化(默认容量上限, 持久化目录, None)
    }

    /// 装配五行并指定持久化目录与图谱扫描根：扫描根 Some 时自动扫描 Rust workspace
    /// 构建世界态图谱（扫描失败告警回退空图谱，不阻断启动）；None 保持空图谱（v1.40 行为）
    pub fn 装配带扫描(持久化目录: Option<String>, 扫描根: Option<String>) -> Self {
        Self::装配带持久化(默认容量上限, 持久化目录, 扫描根)
    }

    /// 装配五行并指定容量上限与持久化目录：历史文件存在则加载，否则新建；
    /// 同时实例化认知三态（空结构）与运行日志记录器。
    pub fn 装配带持久化(容量上限: usize, 持久化目录: Option<String>, 扫描根: Option<String>) -> Self {
        let 容器 = Arc::new(组件容器::new());

        let 信号总线 = Arc::new(内存信号总线::new());
        容器.注册(信号总线.clone());
        let 信号: Arc<dyn 信号总线> = 信号总线.clone();

        let 任务路径 = 持久化目录.as_ref().map(|d| format!("{d}/{}", 持久化文件_任务));
        let 任务仓库 = 加载或新建::<TaskStore>(任务路径.clone(), TaskStore::new, TaskStore::加载);
        let 任务仓库 = Arc::new(Mutex::new(任务仓库));
        {
            let mut 引擎 = 任务仓库.lock().expect("引擎锁中毒");
            容器.注册命名(引擎.name());
            引擎.设置信号总线(信号.clone());
            if let Some(路径) = &任务路径 { 引擎.设置持久化路径(路径.clone()); }
        }
        克制金克木(&任务仓库, 容量上限);
        let 任务仓库: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>> = 任务仓库;

        let 迭代路径 = 持久化目录.as_ref().map(|d| format!("{d}/{}", 持久化文件_迭代));
        let 迭代日志 = 加载或新建::<IterationLog>(迭代路径.clone(), IterationLog::new, IterationLog::加载);
        let 迭代日志 = Arc::new(Mutex::new(迭代日志));
        {
            let mut 引擎 = 迭代日志.lock().expect("引擎锁中毒");
            容器.注册命名(引擎.name());
            引擎.设置信号总线(信号.clone());
            if let Some(路径) = &迭代路径 { 引擎.设置持久化路径(路径.clone()); }
        }
        克制水克火(&迭代日志, 容量上限);
        let 迭代日志: Arc<Mutex<dyn 迭代日志契约<Iteration, Version>>> = 迭代日志;

        let 记忆路径 = 持久化目录.as_ref().map(|d| format!("{d}/{}", 持久化文件_记忆));
        let 记忆库 = 加载或新建::<MemoryStore>(记忆路径.clone(), MemoryStore::new, MemoryStore::加载);
        let 记忆库 = Arc::new(Mutex::new(记忆库));
        {
            let mut 引擎 = 记忆库.lock().expect("引擎锁中毒");
            容器.注册命名(引擎.name());
            引擎.设置信号总线(信号.clone());
            if let Some(路径) = &记忆路径 { 引擎.设置持久化路径(路径.clone()); }
        }
        克制木克土(&记忆库, 容量上限);
        let 记忆库: Arc<Mutex<dyn 记忆库契约<Memory>>> = 记忆库;

        let 规则路径 = 持久化目录.as_ref().map(|d| format!("{d}/{}", 持久化文件_规则));
        let 规则库 = 加载或新建::<RuleSet>(规则路径.clone(), RuleSet::new, RuleSet::加载);
        let 规则库 = Arc::new(Mutex::new(规则库));
        {
            let mut 引擎 = 规则库.lock().expect("引擎锁中毒");
            容器.注册命名(引擎.name());
            引擎.设置信号总线(信号.clone());
            if let Some(路径) = &规则路径 { 引擎.设置持久化路径(路径.clone()); }
        }
        克制火克金(&规则库, 容量上限);
        let 规则库: Arc<Mutex<dyn 规则库契约<Rule>>> = 规则库;

        let 事件路径 = 持久化目录.as_ref().map(|d| format!("{d}/{}", 持久化文件_事件));
        let 事件总线 = 加载或新建::<EventBus>(事件路径.clone(), EventBus::new, EventBus::加载);
        let 事件总线 = Arc::new(Mutex::new(事件总线));
        {
            let mut 引擎 = 事件总线.lock().expect("引擎锁中毒");
            容器.注册命名(引擎.name());
            引擎.设置信号总线(信号.clone());
            if let Some(路径) = &事件路径 { 引擎.设置持久化路径(路径.clone()); }
        }
        let 事件总线: Arc<Mutex<dyn 事件总线契约<Event>>> = 事件总线;

        // 认知三态：世界态图谱——扫描根 Some 时自动扫描 Rust workspace 构建（代码即真源）；
        // 扫描失败告警回退空图谱，不阻断启动（v1.40 起为空结构，等待填充）
        let 图谱 = Arc::new(Mutex::new(match &扫描根 {
            Some(根) => match hm_cognition::Rust扫描器::新().扫描(std::path::Path::new(根)) {
                Ok(图) => {
                    tracing::info!(
                        "图谱扫描完成：{} 模块、{} 符号、{} 依赖、技术栈 {} 项",
                        图.模块集.len(),
                        图.符号集.len(),
                        图.依赖集.len(),
                        图.技术栈.len()
                    );
                    图
                }
                Err(失败) => {
                    tracing::warn!("图谱扫描失败，回退空图谱（不影响启动）: {失败}");
                    图谱::新()
                }
            },
            None => 图谱::新(),
        }));
        let 心智地图 = Arc::new(Mutex::new(心智地图::新()));
        // 建图后自动提炼：把图谱摘要写入心智格位（外在·结构/依赖/接口、执行·技术栈/工具/产出，
        // 目标·现况/偏移/度量、经历·事件/教训/时间），使三态心智从"36 空格位"进入"有摘要"
        // （规则提炼器，零网络、可重现；迭代日志提供目标/经历维度线索）
        {
            // 提取迭代线索（跨域解耦为 hm-cognition 轻量结构，供目标/经历维度提炼）
            let 迭代们: Vec<hm_cognition::迭代线索> = {
                let 日志 = 迭代日志.lock().expect("迭代日志锁中毒");
                日志
                    .全部()
                    .iter()
                    .map(|迭代| hm_cognition::迭代线索 {
                        版本: format!("v{}.{}.{}", 迭代.version.主, 迭代.version.次, 迭代.version.修订),
                        状态: match 迭代.status {
                            迭代状态::已完成 => "已完成".into(),
                            迭代状态::已放弃 => "已放弃".into(),
                            迭代状态::进行中 => "进行中".into(),
                        },
                        变更说明: 迭代.变更说明.clone(),
                        时间: 迭代.created_at,
                    })
                    .collect()
            };
            let 图 = 图谱.lock().expect("图谱锁中毒");
            let 候选 = hm_cognition::规则提炼器.提炼_带历史(&图, &迭代们);
            drop(图);
            if !候选.is_empty() {
                let mut 心智 = 心智地图.lock().expect("心智锁中毒");
                let mut 写成功 = 0;
                for 摘要 in 候选 {
                    match 心智.写(摘要.维度, 摘要.格位名.as_str(), 摘要.摘要, 摘要.可信度, 摘要.证据引用) {
                        Ok(_) => 写成功 += 1,
                        Err(失败) => tracing::warn!("格位提炼写入失败: {失败}"),
                    }
                }
                tracing::info!("图谱提炼完成：写入 {} 个心智格位", 写成功);
            }
        }
        let 语境 = Arc::new(Mutex::new(过程上下文::新()));
        // 运行日志记录器（信号转日志，供客户端日志视图读取）
        let 日志记录器 = Arc::new(Mutex::new(运行日志记录器::new()));

        桥接木生火(&信号, &迭代日志);
        桥接火生土(&信号, &记忆库);
        桥接土生金(&信号, &规则库);
        桥接金生水(&信号, &事件总线);
        桥接水生木(&信号, &任务仓库);

        // 五行相克：土克水（事件去重）保留信号驱动；其余约束已通过容量上限内置引擎
        克制土克水(&信号, &事件总线);

        // 信号转日志：把每类信号记录为一条运行日志
        桥接信号转日志(&信号, &日志记录器);

        五行装配 {
            容器,
            信号总线: 信号,
            任务仓库,
            迭代日志,
            记忆库,
            规则库,
            事件总线,
            图谱,
            心智地图,
            语境,
            日志记录器,
        }
    }
}

/// 加载或新建：历史文件存在则加载，损坏则告警回退新建；否则新建
fn 加载或新建<T>(
    路径: Option<String>,
    新建: impl FnOnce() -> T,
    加载: impl FnOnce(&str) -> hm_error::Result<T>,
) -> T {
    match 路径 {
        Some(p) if std::path::Path::new(&p).exists() => match 加载(&p) {
            Ok(引擎) => 引擎,
            Err(e) => {
                tracing::warn!("加载持久化数据失败，回退新建: {e}");
                新建()
            }
        },
        _ => 新建(),
    }
}

/// 信号转日志：订阅全部七类信号，每条信号记录为一条运行日志
fn 桥接信号转日志(总线: &Arc<dyn 信号总线>, 日志记录器: &Arc<Mutex<运行日志记录器>>) {
    let 日志记录器 = 日志记录器.clone();
    let 处理器 = Arc::new(move |sig: &信号| {
        let 内容 = 信号摘要(sig);
        日志记录器
            .lock()
            .expect("日志锁中毒")
            .记日志(sig.类型.clone(), 信号日志样式.to_string(), 内容);
    });
    let 类型列表 = [
        信号类型::任务完成,
        信号类型::任务推进,
        信号类型::迭代完成,
        信号类型::迭代放弃,
        信号类型::记忆写入,
        信号类型::规则命中,
        信号类型::事件发布,
    ];
    for 类型 in 类型列表 {
        总线.订阅(类型, 处理器.clone());
    }
}

/// 从信号载荷提取摘要文本（优先标题 → 内容 → 变更说明 → 规则名 → 结论 → 类型 → 占位）
fn 信号摘要(sig: &信号) -> String {
    let 载荷 = &sig.载荷;
    if let Some(标题) = &载荷.标题 { return 标题.clone(); }
    if let Some(内容) = &载荷.内容 { return 内容.clone(); }
    if let Some(变更说明) = &载荷.变更说明 { return 变更说明.clone(); }
    if let Some(规则名) = &载荷.规则名 { return 规则名.clone(); }
    if let Some(结论) = &载荷.结论 { return 结论.clone(); }
    if let Some(类型) = &载荷.类型 { return 类型.clone(); }
    "（无载荷）".to_string()
}

/// 木生火：任务完成 → 开启迭代
fn 桥接木生火(总线: &Arc<dyn 信号总线>, 迭代日志: &Arc<Mutex<dyn 迭代日志契约<Iteration, Version>>>) {
    let 迭代日志 = 迭代日志.clone();
    总线.订阅(信号类型::任务完成, Arc::new(move |sig| {
        if let Some(title) = sig.载荷.标题.clone() {
            let 描述 = sig.载荷.描述.clone().unwrap_or_default();
            let 变更说明 = if 描述.is_empty() {
                format!("{任务完成前缀}{title}")
            } else {
                format!("{任务完成前缀}{title}：{描述}")
            };
            let mut log = 迭代日志.lock().expect("引擎锁中毒");
            // 防重复：已有进行中迭代时不再开启新迭代
            if log.全部().iter().any(|it| it.status == 迭代状态::进行中) {
                tracing::warn!("木生火跳过：已有进行中迭代，避免重复开启");
                return;
            }
            let version = log.当前版本().递增次();
            if let Err(e) = log.开启(version, 变更说明) {
                tracing::warn!("木生火开启迭代失败: {e}");
            }
        }
    }));
}

/// 火生土：迭代完成 → 写入记忆
fn 桥接火生土(总线: &Arc<dyn 信号总线>, 记忆库: &Arc<Mutex<dyn 记忆库契约<Memory>>>) {
    let 记忆库 = 记忆库.clone();
    总线.订阅(信号类型::迭代完成, Arc::new(move |sig| {
        if let Some(说明) = sig.载荷.变更说明.clone() {
            if let Err(e) = 记忆库
                .lock()
                .expect("引擎锁中毒")
                .写入(说明, 迭代产出标签.to_string())
            {
                tracing::warn!("火生土写入记忆失败: {e}");
            }
        }
    }));
}

/// 土生金：记忆写入 → 添加规则（条件取自记忆标签，非空，评估时需事实匹配）
fn 桥接土生金(总线: &Arc<dyn 信号总线>, 规则库: &Arc<Mutex<dyn 规则库契约<Rule>>>) {
    let 规则库 = 规则库.clone();
    总线.订阅(信号类型::记忆写入, Arc::new(move |sig| {
        let 内容 = sig.载荷.内容.clone().unwrap_or_default();
        let 标签 = sig.载荷.标签.clone().unwrap_or_default();
        if 标签.is_empty() {
            tracing::warn!("土生金跳过：记忆无标签，无法生成有意义的规则条件");
            return;
        }
        let 条件 = vec![(载荷键::标签.to_string(), 标签.clone())];
        if let Err(e) = 规则库
            .lock()
            .expect("引擎锁中毒")
            .添加规则(&标签, 条件, &内容, 记忆生成规则优先级)
        {
            tracing::warn!("土生金添加规则失败: {e}");
        }
    }));
}

/// 金生水：规则命中 → 发布事件
fn 桥接金生水(总线: &Arc<dyn 信号总线>, 事件总线: &Arc<Mutex<dyn 事件总线契约<Event>>>) {
    let 事件总线 = 事件总线.clone();
    总线.订阅(信号类型::规则命中, Arc::new(move |sig| {
        let 规则名 = sig.载荷.规则名.clone().unwrap_or_default();
        let 结论 = sig.载荷.结论.clone().unwrap_or_default();
        if 结论.is_empty() {
            tracing::warn!("金生水跳过：规则命中无结论，不发布空事件");
            return;
        }
        事件总线.lock().expect("引擎锁中毒").发布(规则名, vec![(载荷键::结论.to_string(), 结论)]);
    }));
}

/// 水生木：事件发布 → 创建任务
fn 桥接水生木(总线: &Arc<dyn 信号总线>, 任务仓库: &Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>>) {
    let 任务仓库 = 任务仓库.clone();
    总线.订阅(信号类型::事件发布, Arc::new(move |sig| {
        if let Some(类型) = sig.载荷.类型.clone() {
            let 描述 = sig.载荷.内容.clone().unwrap_or_default();
            if let Err(e) = 任务仓库
                .lock()
                .expect("引擎锁中毒")
                .创建(format!("{事件驱动前缀}{类型}"), 描述)
            {
                tracing::warn!("水生木创建任务失败: {e}");
            }
        }
    }));
}
