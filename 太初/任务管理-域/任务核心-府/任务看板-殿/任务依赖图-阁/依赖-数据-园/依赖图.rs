use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::任务模型_殿::任务标识;

/// 任务依赖图：以任务标识（UUID）为节点、依赖关系为边的有向图。
/// 边 (A, B) 语义：A 依赖 B（A 的完成需要 B 先完成）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct 任务依赖图 {
    pub 节点: HashMap<Uuid, 任务标识>,
    /// (来源任务, 被依赖任务)：来源任务依赖被依赖任务
    pub 边: Vec<(Uuid, Uuid)>,
}

impl 任务依赖图 {
    pub fn 新() -> Self {
        任务依赖图 { 节点: HashMap::new(), 边: Vec::new() }
    }

    pub fn 添加任务(&mut self, 标识: 任务标识) {
        self.节点.insert(标识.任务id, 标识);
    }

    /// 添加依赖：任务 A 依赖任务 B（A 的完成需要 B 先完成）
    pub fn 添加依赖(&mut self, 任务a: Uuid, 任务b: Uuid) {
        if 任务a == 任务b {
            return; // 自依赖环不允许写入（防自环）
        }
        if !self.边.contains(&(任务a, 任务b)) {
            self.边.push((任务a, 任务b));
        }
    }

    /// 这个任务依赖谁（直接依赖）
    pub fn 查询依赖(&self, 任务id: Uuid) -> Vec<Uuid> {
        self.边
            .iter()
            .filter(|(a, _)| *a == 任务id)
            .map(|(_, b)| *b)
            .collect()
    }

    /// 谁依赖这个任务（直接被依赖）
    pub fn 查询被依赖(&self, 任务id: Uuid) -> Vec<Uuid> {
        self.边
            .iter()
            .filter(|(_, b)| *b == 任务id)
            .map(|(a, _)| *a)
            .collect()
    }

    /// 递归查「被依赖」链（谁直接/间接依赖该任务），去重保序；深度上限默认 2。
    /// 用于影响分析：任务 B 回退后，依赖 B（及依赖其下游）的任务都应被召回。
    pub fn 查询传递被依赖(&self, 任务id: Uuid, 深度上限: usize) -> Vec<Uuid> {
        let mut 结果 = Vec::new();
        let mut 已见 = HashSet::new();
        let mut 队列: Vec<(Uuid, usize)> = self
            .查询被依赖(任务id)
            .into_iter()
            .map(|u| (u, 1))
            .collect();
        while let Some((当前, 深度)) = 队列.pop() {
            if !已见.insert(当前) {
                continue;
            }
            结果.push(当前);
            if 深度 < 深度上限 {
                for 上游 in self.查询被依赖(当前) {
                    队列.push((上游, 深度 + 1));
                }
            }
        }
        结果
    }

    /// DFS 检测循环依赖，返回任一环路径（A→B→A）
    pub fn 检测循环依赖(&self) -> Option<Vec<Uuid>> {
        fn dfs(
            当前: &Uuid,
            邻接: &std::collections::HashMap<Uuid, Vec<Uuid>>,
            路径: &mut Vec<Uuid>,
            在栈: &mut HashSet<Uuid>,
        ) -> Option<Vec<Uuid>> {
            if !在栈.insert(*当前) {
                let 起点 = 路径.iter().position(|u| u == 当前)?;
                let mut 环 = 路径[起点..].to_vec();
                环.push(*当前);
                return Some(环);
            }
            路径.push(*当前);
            if let Some(下游) = 邻接.get(当前) {
                for 下一 in 下游 {
                    if let Some(环) = dfs(下一, 邻接, 路径, 在栈) {
                        return Some(环);
                    }
                }
            }
            路径.pop();
            在栈.remove(当前);
            None
        }
        let mut 邻接: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (a, b) in &self.边 {
            邻接.entry(*a).or_default().push(*b);
        }
        let 节点: Vec<Uuid> = self.节点.keys().copied().collect();
        let mut 路径 = Vec::new();
        let mut 在栈 = HashSet::new();
        for 起点 in 节点 {
            路径.clear();
            在栈.clear();
            if let Some(环) = dfs(&起点, &邻接, &mut 路径, &mut 在栈) {
                return Some(环);
            }
        }
        None
    }

    /// 移除任务及其相关边
    pub fn 移除任务(&mut self, 任务id: Uuid) {
        self.节点.remove(&任务id);
        self.边.retain(|(a, b)| *a != 任务id && *b != 任务id);
    }
}
