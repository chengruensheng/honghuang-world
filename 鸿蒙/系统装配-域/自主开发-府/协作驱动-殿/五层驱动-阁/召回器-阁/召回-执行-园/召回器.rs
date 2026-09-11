use std::sync::{Arc, Mutex};
use uuid::Uuid;
use hm_contract::当前时间戳;
use tc_task::{StatusChange, TaskBoard, TaskStatus, 五行层级, 任务依赖图, 状态层级标签};
use crate::协作驱动_殿::五层驱动_阁::召回器_阁::{召回事件, 召回事件状态};

/// 召回器：任务回退后的错误传染治理（纯规则，不用 LLM）。
/// - 影响分析：沿依赖图向上游查被依赖链（深度≤2，上限 10 条，超限截断）
/// - 执行召回：受影响任务按当前状态置为对应「待重新*」状态 + 召回标记
/// - 解除召回：触发任务重新完成后，恢复被召回任务原层级状态
#[derive(Default)]
pub struct 召回器 {
    /// 召回事件记录（内存；过渡期不落盘，解除召回按触发任务检索）
    pub 事件记录: Arc<Mutex<Vec<召回事件>>>,
}

impl 召回器 {
    pub fn 新() -> Self {
        召回器::default()
    }

    /// 影响分析：回退任务的上游被依赖任务（深度≤2），上限 10 条（超出截断）
    pub fn 影响分析(&self, 回退任务id: Uuid, 依赖图: &任务依赖图) -> Vec<Uuid> {
        let mut 结果 = 依赖图.查询传递被依赖(回退任务id, 2);
        if 结果.len() > 10 {
            结果.truncate(10);
        }
        结果
    }

    /// 执行召回：对每个受影响任务按其当前状态置为「待重新*」状态 + 召回标记=true，
    /// 记录召回事件（状态=召回中）。已在召回中的任务跳过。
    /// 返回本次产生的召回事件（仅含实际被召回任务非空的事件）。
    pub fn 执行召回(
        &self,
        触发任务id: Uuid,
        任务列表: Vec<Uuid>,
        原因: &str,
        看板: &mut TaskBoard,
    ) -> Vec<召回事件> {
        let mut 实际召回 = Vec::new();
        for id in 任务列表 {
            let 结果 = 看板.按标识改写(&id, |t| {
                if t.召回标记 {
                    return;
                }
                if let Some(目标) = 目标召回状态(&t.status) {
                    t.召回标记 = true;
                    let 原 = t.status;
                    t.status = 目标;
                    t.当前层级 = 状态层级(&目标);
                    t.状态历史.push(StatusChange {
                        原状态: 原,
                        新状态: 目标,
                        操作者: t.当前承接人,
                        时间: 当前时间戳(),
                        备注: Some(format!("召回({原因})")),
                    });
                    t.updated_at = 当前时间戳();
                    实际召回.push(id);
                }
            });
            if 结果.is_err() {
                tracing::warn!("召回跳过不存在的任务标识 {id}");
            }
        }
        if 实际召回.is_empty() {
            return Vec::new();
        }
        let 事件 = 召回事件::新(触发任务id, 原因, 实际召回, 当前时间戳());
        self.事件记录.lock().expect("召回事件记录锁中毒").push(事件.clone());
        vec![事件]
    }

    /// 解除召回：触发任务重新完成后，恢复其全部召回事件中被召回任务的原层级状态，
    /// 召回标记=false，事件状态=已解除。返回恢复的任务数。
    pub fn 解除召回(&self, 触发任务id: Uuid, _依赖图: &任务依赖图, 看板: &mut TaskBoard) -> usize {
        let 记录 = self.事件记录.lock().expect("召回事件记录锁中毒");
        let 待解除: Vec<Vec<Uuid>> = 记录
            .iter()
            .filter(|e| e.触发任务id == 触发任务id && e.状态 == 召回事件状态::召回中)
            .map(|e| e.影响任务.clone())
            .collect();
        let mut 恢复数 = 0;
        for 列表 in 待解除 {
            for id in 列表 {
                let 结果 = 看板.按标识改写(&id, |t| {
                    if let Some(恢复) = 解除映射(&t.status) {
                        t.召回标记 = false;
                        let 原 = t.status;
                        t.status = 恢复;
                        t.当前层级 = 状态层级(&恢复);
                        t.状态历史.push(StatusChange {
                            原状态: 原,
                            新状态: 恢复,
                            操作者: t.当前承接人,
                            时间: 当前时间戳(),
                            备注: Some(format!("召回解除→{:?}", 恢复)),
                        });
                        t.updated_at = 当前时间戳();
                        恢复数 += 1;
                    }
                });
                if 结果.is_err() {
                    tracing::warn!("召回解除跳过不存在的任务标识 {id}");
                }
            }
        }
        drop(记录);
        let mut 记录 = self.事件记录.lock().expect("召回事件记录锁中毒");
        for 事件 in 记录.iter_mut() {
            if 事件.触发任务id == 触发任务id && 事件.状态 == 召回事件状态::召回中 {
                事件.状态 = 召回事件状态::已解除;
            }
        }
        恢复数
    }
}

/// 状态 → 五行层级标签（收敛到 tc-task 权威映射，保持同签名避免改两处调用点）
fn 状态层级(状态: &TaskStatus) -> 五行层级 {
    状态层级标签(*状态)
}

/// 当前状态 → 召回目标状态（按五行层级：设计中→待重新设计；实现层→待重新实现；验收/已完成→待重新验收；水层→待重新清理）
fn 目标召回状态(状态: &TaskStatus) -> Option<TaskStatus> {
    use TaskStatus::*;
    match 状态 {
        待圣人设计 | 圣人设计中 | 待重新设计 => Some(待重新设计),
        待大罗金仙实现 | 大罗金仙实现中 | 待修复 | 待重新实现 => Some(待重新实现),
        待准圣验收 | 准圣验收中 | 待道祖终审 | 道祖终审中 | 待人工验收 | 人工验收中 | 已完成 | 待重新验收 => Some(待重新验收),
        待清理 | 清理中 | 待重新清理 => Some(待重新清理),
        _ => None,
    }
}

/// 召回状态 → 恢复原层级状态
fn 解除映射(状态: &TaskStatus) -> Option<TaskStatus> {
    use TaskStatus::*;
    match 状态 {
        待重新设计 => Some(待圣人设计),
        待重新实现 => Some(待大罗金仙实现),
        待重新验收 => Some(待准圣验收),
        待重新清理 => Some(待清理),
        _ => None,
    }
}
