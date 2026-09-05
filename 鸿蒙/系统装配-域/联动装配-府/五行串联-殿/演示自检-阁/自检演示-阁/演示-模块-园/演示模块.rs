use hm_signal::载荷键;
use tc_task::TaskStatus;
use lj_iteration::迭代状态;
use crate::{五行装配, 迭代产出标签};

/// 演示/自检数据的标题前缀（区分真实业务数据，便于识别与清理）
const 演示前缀: &str = "[演示] ";

/// 五行相生闭环自检后的状态快照
pub struct 闭环状态 {
    pub 任务数: usize,
    pub 迭代数: usize,
    pub 记忆数: usize,
    pub 规则数: usize,
    pub 事件数: usize,
}

/// 五行相克闭环自检后的状态快照
pub struct 克制状态 {
    pub 去重事件: bool,
}

/// 启动自检：触发一次完整五行相生闭环，返回各引擎计数。
///
/// 本函数为演示/自检代码，不进入生产路径；由启动入口通过 `run_self_test`
/// 配置开关控制，默认关闭。演示数据统一加 `演示前缀` 标记，可与真实数据区分。
pub fn 演示闭环(装配: &五行装配) -> hm_error::Result<闭环状态> {
    // 木生火：创建任务并推进到已完成
    let 任务 = 装配
        .任务仓库
        .lock()
        .expect("引擎锁中毒")
        .创建(format!("{演示前缀}启动自检任务"), "启动演示".to_string())?;
    装配
        .任务仓库
        .lock()
        .expect("引擎锁中毒")
        .推进(任务, TaskStatus::进行中)?;
    装配
        .任务仓库
        .lock()
        .expect("引擎锁中毒")
        .推进(任务, TaskStatus::已完成)?;

    // 火生土 + 土生金：完成刚开启的进行中迭代
    let 迭代 = 装配
        .迭代日志
        .lock()
        .expect("引擎锁中毒")
        .全部()
        .iter()
        .find(|it| it.status == 迭代状态::进行中)
        .map(|it| it.id);
    if let Some(id) = 迭代 {
        装配.迭代日志.lock().expect("引擎锁中毒").完成(id)?;
    }

    // 金生水 + 水生木：评估含标签的事实，命中土生金生成的规则 → 事件 → 任务
    装配.规则库.lock().expect("引擎锁中毒").评估(&[(
        载荷键::标签.to_string(),
        迭代产出标签.to_string(),
    )]);

    Ok(闭环状态 {
        任务数: 装配.任务仓库.lock().expect("引擎锁中毒").全部().len(),
        迭代数: 装配.迭代日志.lock().expect("引擎锁中毒").全部().len(),
        记忆数: 装配.记忆库.lock().expect("引擎锁中毒").全部().len(),
        规则数: 装配.规则库.lock().expect("引擎锁中毒").全部().len(),
        事件数: 装配.事件总线.lock().expect("引擎锁中毒").全部().len(),
    })
}

/// 启动自检：验证土克水（事件去重）克制闭环可运行。
///
/// 本函数为演示/自检代码，不进入生产路径；由启动入口通过 `run_self_test`
/// 配置开关控制，默认关闭。
pub fn 演示相克(装配: &五行装配) -> hm_error::Result<克制状态> {
    // 制造重复事件，写入记忆触发土克水去重
    装配
        .事件总线
        .lock()
        .expect("引擎锁中毒")
        .发布(format!("{演示前缀}重复事件"), vec![]);
    装配
        .事件总线
        .lock()
        .expect("引擎锁中毒")
        .发布(format!("{演示前缀}重复事件"), vec![]);
    let 去重前 = 装配.事件总线.lock().expect("引擎锁中毒").全部().len();
    装配
        .记忆库
        .lock()
        .expect("引擎锁中毒")
        .写入(format!("{演示前缀}演示记忆"), "演示".to_string())?;
    let 去重后 = 装配.事件总线.lock().expect("引擎锁中毒").全部().len();

    Ok(克制状态 {
        去重事件: 去重后 < 去重前,
    })
}