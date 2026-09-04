use std::sync::{Arc, Mutex};
use hm_signal::{信号总线, 信号类型};
use hm_domain_contract::事件总线契约;
use tc_task::TaskStore;
use lj_iteration::IterationLog;
use qk_memory::MemoryStore;
use dy_rule::RuleSet;
use hd_event::Event;

/// 相克：约束增长（过盛时限制新增，而非删除已有）。
///
/// 金克木（待受理任务过盛拒绝创建）、木克土（记忆过盛标注待归档）、
/// 水克火（进行中迭代过盛拒绝开启）、火克金（规则过盛拒绝新增）
/// 均通过给引擎注入容量上限实现；土克水（事件去重）保留信号驱动。

/// 金克木：待受理任务过盛时拒绝创建新任务
pub fn 克制金克木(任务仓库: &Arc<Mutex<TaskStore>>, 上限: usize) {
    任务仓库.lock().expect("引擎锁中毒").设置容量上限(上限);
}

/// 木克土：记忆过盛时新记忆降级为待归档
pub fn 克制木克土(记忆库: &Arc<Mutex<MemoryStore>>, 上限: usize) {
    记忆库.lock().expect("引擎锁中毒").设置容量上限(上限);
}

/// 水克火：进行中迭代过盛时拒绝开启新迭代
pub fn 克制水克火(迭代日志: &Arc<Mutex<IterationLog>>, 上限: usize) {
    迭代日志.lock().expect("引擎锁中毒").设置容量上限(上限);
}

/// 火克金：规则过盛时拒绝新增（严格数量上限）
pub fn 克制火克金(规则库: &Arc<Mutex<RuleSet>>, 上限: usize) {
    规则库.lock().expect("引擎锁中毒").设置容量上限(上限);
}

/// 土克水：记忆写入 → 事件去重（记忆固化事件流，重复事件合并）
pub fn 克制土克水(总线: &Arc<dyn 信号总线>, 事件总线: &Arc<Mutex<dyn 事件总线契约<Event>>>) {
    let 事件总线 = 事件总线.clone();
    总线.订阅(信号类型::记忆写入, Arc::new(move |_sig| {
        事件总线.lock().expect("引擎锁中毒").去重();
    }));
}
