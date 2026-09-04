use std::collections::HashMap;
use std::sync::Arc;
use hm_contract::{Component, 当前时间戳};
use hm_error::{Error, Result};
use hm_signal::{信号总线, 信号类型, 信号载荷};
use hm_signal::引擎支撑;
use hm_domain_contract::任务仓库契约;
use serde::{Deserialize, Serialize};
use crate::任务模型_殿::{Task, TaskStatus};

/// 容量上限默认值：不限（约束由装配层注入）。
/// 可移植写法：64 位平台可序列化 i64::MAX，32 位平台回退 usize::MAX 避免溢出。
fn 默认容量上限() -> usize {
    match usize::try_from(i64::MAX) {
        Ok(上限) => 上限,
        Err(_) => usize::MAX,
    }
}

/// 任务仓库：内存登记任务，支持查询、状态推进、容量约束与落盘持久化。
/// 以 Vec 保存任务保证持久化顺序，另建 id 索引使按 id 查询 O(1)。
#[derive(Clone, Serialize, Deserialize)]
pub struct TaskStore {
    tasks: Vec<Task>,
    next_id: u64,
    #[serde(default = "默认容量上限")]
    容量上限: usize,
    #[serde(skip)]
    索引: HashMap<u64, usize>,
    #[serde(skip)]
    信号总线: Option<Arc<dyn 信号总线>>,
    #[serde(skip)]
    持久化路径: Option<String>,
}

impl TaskStore {
    pub fn new() -> Self {
        TaskStore {
            tasks: Vec::new(),
            next_id: 1,
            容量上限: 默认容量上限(),
            索引: HashMap::new(),
            信号总线: None,
            持久化路径: None,
        }
    }

    引擎支撑!();

    /// 设置待受理任务容量上限（金克木：过盛时约束新增）
    pub fn 设置容量上限(&mut self, 上限: usize) {
        self.容量上限 = 上限;
    }

    /// 任务诞生：创建并登记，返回任务 id；待受理任务过盛时拒绝
    pub fn 创建(&mut self, title: String, description: String) -> Result<u64> {
        let 待受理数 = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::待受理)
            .count();
        if 待受理数 >= self.容量上限 {
            return Err(Error::容量超限(format!("待受理任务已达上限 {}", self.容量上限)));
        }
        let id = self.next_id;
        self.next_id += 1;
        let task = Task::新建(id, title, description, 当前时间戳());
        self.索引.insert(id, self.tasks.len());
        self.tasks.push(task);
        self.自动保存();
        Ok(id)
    }

    /// 按 id 查询任务（受理）
    pub fn 查询(&self, id: u64) -> Option<&Task> {
        self.索引.get(&id).map(|&i| &self.tasks[i])
    }

    /// 列出全部任务
    pub fn 全部(&self) -> Vec<&Task> {
        self.tasks.iter().collect()
    }

    /// 状态推进（木之生长），非法流转返回错误
    pub fn 推进(&mut self, id: u64, next: TaskStatus) -> Result<()> {
        let 下标 = *self.索引.get(&id).ok_or_else(|| Error::任务不存在(id))?;
        let task = &mut self.tasks[下标];
        if !task.status.可流转到(&next) {
            return Err(Error::状态流转非法(format!("{:?} → {:?}", task.status, next)));
        }
        let title = task.title.clone();
        let description = task.description.clone();
        task.status = next;
        if next == TaskStatus::已完成 {
            self.发布信号(
                信号类型::任务完成,
                信号载荷 {
                    标识: Some(id.to_string()),
                    标题: Some(title),
                    描述: Some(description),
                    ..信号载荷::default()
                },
            );
        } else {
            self.发布信号(
                信号类型::任务推进,
                信号载荷 {
                    标识: Some(id.to_string()),
                    标题: Some(title),
                    内容: Some(format!("{:?}", next)),
                    ..信号载荷::default()
                },
            );
        }
        self.自动保存();
        Ok(())
    }

    /// 保存到文件（落盘）
    pub fn 保存(&self, path: &str) -> Result<()> {
        let content = toml::to_string(self)
            .map_err(|e| Error::序列化(format!("序列化任务失败: {e}")))?;
        std::fs::write(path, content).map_err(Error::Io)?;
        Ok(())
    }

    /// 从文件加载（还原），信号总线需重新注入
    pub fn 加载(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
        let mut store: TaskStore = toml::from_str(&content)
            .map_err(|e| Error::反序列化(format!("解析任务文件失败: {e}")))?;
        store.重建索引();
        Ok(store)
    }

    /// 重建 id 索引（加载后调用）
    fn 重建索引(&mut self) {
        self.索引.clear();
        for (i, t) in self.tasks.iter().enumerate() {
            self.索引.insert(t.id, i);
        }
    }

}

impl Component for TaskStore {
    fn name(&self) -> &'static str { "任务仓库" }
}

impl 任务仓库契约<Task, TaskStatus> for TaskStore {
    fn 创建(&mut self, 标题: String, 描述: String) -> Result<u64> {
        TaskStore::创建(self, 标题, 描述)
    }

    fn 查询(&self, id: u64) -> Option<&Task> {
        TaskStore::查询(self, id)
    }

    fn 全部(&self) -> Vec<&Task> {
        TaskStore::全部(self)
    }

    fn 推进(&mut self, id: u64, 状态: TaskStatus) -> Result<()> {
        TaskStore::推进(self, id, 状态)
    }

}
