use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use hm_contract::当前时间戳;
use hm_cognition::AgentRole;
use hm_error::{Error, Result};
use hm_signal::{信号, 信号总线, 信号类型, 信号载荷};
use crate::任务模型_殿::{
    Task, TaskStatus, StatusChange, RequirementDoc, DesignDoc, ImplementationDoc, VerificationDoc,
    FinalAcceptanceDoc,
};
use crate::任务看板_殿::{状态归属角色, 承接后状态};

/// 任务看板：洪荒五层协作的唯一通道，任务包含所有元信息。
///
/// 层级间所有信息传递都通过看板，不通过上下文传话；承接人只需看任务即可完整理解。
/// 存储采用 JSONL（每行一个任务 JSON），支持增量写入与全量加载。
pub struct TaskBoard {
    任务集: HashMap<u64, Task>,
    下一序号: u64,
    存储路径: PathBuf,

    信号总线: Option<Arc<dyn 信号总线>>,
}

impl TaskBoard {
    /// 初始化看板并绑定存储路径（不立即读盘，由 加载 决定）
    pub fn 新建(存储路径: impl Into<PathBuf>) -> Self {
        TaskBoard {
            任务集: HashMap::new(),
            下一序号: 1,
            存储路径: 存储路径.into(),
            信号总线: None,
        }
    }

    /// 注入信号总线（装配层串联：看板状态变更驱动五行相生闭环）
    pub fn 设置信号总线(&mut self, 总线: Arc<dyn 信号总线>) {
        self.信号总线 = Some(总线);
    }

    /// 发布信号（无总线时静默忽略，与引擎支撑宏行为一致）
    fn 发布信号(&self, 类型: &str, 载荷: 信号载荷) {
        if let Some(总线) = &self.信号总线 {
            总线.发布(&信号::新建(类型.to_string(), 载荷));
        }
    }

    /// 发布任务（道祖发布需求）；id 为 0 时自动分配
    pub fn 发布任务(&mut self, mut task: Task) -> Result<u64> {
        if task.id == 0 {
            task.id = self.下一序号;
        }
        self.下一序号 = self.下一序号.max(task.id + 1);
        if self.任务集.contains_key(&task.id) {
            return Err(Error::Other(format!("任务已存在: {}", task.id)));
        }
        task.状态历史.push(StatusChange {
            原状态: TaskStatus::待受理,
            新状态: task.status,
            操作者: task.发起人,
            时间: 当前时间戳(),
            备注: Some("发布".to_string()),
        });
        let id = task.id;
        let 标题 = task.title.clone();
        let 描述 = task.description.clone();
        self.任务集.insert(id, task);
        self.保存()?;
        self.发布信号(
            信号类型::任务推进,
            信号载荷 {
                标识: Some(id.to_string()),
                标题: Some(标题),
                描述: Some(描述),
                ..信号载荷::default()
            },
        );
        Ok(id)
    }

    /// 承接任务（圣人/大罗金仙/准圣/道祖承接，流转到「进行中」状态）
    pub fn 承接任务(&mut self, task_id: u64, role: AgentRole) -> Result<()> {
        let 目标 = {
            let task = self.任务集.get(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
            let 归属 = 状态归属角色(task.status)
                .ok_or_else(|| Error::状态流转非法(format!("{:?} 不可承接", task.status)))?;
            if role != 归属 {
                return Err(Error::Other(format!("角色不符：应由 {:?} 承接，实际 {:?}", 归属, role)));
            }
            承接后状态(task.status)
                .ok_or_else(|| Error::状态流转非法(format!("{:?} 不可承接", task.status)))?
        };
        let task = self.任务集.get_mut(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
        let 标题 = task.title.clone();
        task.承接历史.push(role);
        执行推进(task, 目标, role, Some("承接".to_string()))?;
        self.保存()?;
        self.发布信号(
            信号类型::任务推进,
            信号载荷 {
                标识: Some(task_id.to_string()),
                标题: Some(标题),
                内容: Some(format!("{:?}", 目标)),
                ..信号载荷::default()
            },
        );
        Ok(())
    }

    /// 提交任务（完成当前阶段，流转到下一状态）
    pub fn 提交任务(&mut self, task_id: u64, role: AgentRole, next: TaskStatus) -> Result<()> {
        {
            let task = self.任务集.get(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
            let 归属 = 状态归属角色(task.status)
                .ok_or_else(|| Error::状态流转非法(format!("{:?} 不可提交", task.status)))?;
            if role != 归属 {
                return Err(Error::Other(format!("角色不符：应由 {:?} 提交，实际 {:?}", 归属, role)));
            }
        }
        let task = self.任务集.get_mut(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
        let 标题 = task.title.clone();
        let 描述 = task.description.clone();
        执行推进(task, next, role, Some("提交".to_string()))?;
        self.保存()?;
        if next == TaskStatus::已完成 || next == TaskStatus::清理完成 {
            self.发布信号(
                信号类型::任务完成,
                信号载荷 {
                    标识: Some(task_id.to_string()),
                    标题: Some(标题),
                    描述: Some(描述),
                    ..信号载荷::default()
                },
            );
        } else {
            self.发布信号(
                信号类型::任务推进,
                信号载荷 {
                    标识: Some(task_id.to_string()),
                    标题: Some(标题),
                    内容: Some(format!("{:?}", next)),
                    ..信号载荷::default()
                },
            );
        }
        Ok(())
    }

    /// 按 id 查询任务
    pub fn 查询(&self, task_id: u64) -> Option<&Task> {
        self.任务集.get(&task_id)
    }

    /// 列出全部任务
    pub fn 全部(&self) -> Vec<&Task> {
        self.任务集.values().collect()
    }

    /// 按状态/角色筛选任务
    pub fn 筛选(&self, status: Option<TaskStatus>, role: Option<AgentRole>) -> Vec<&Task> {
        self.任务集
            .values()
            .filter(|t| status.map_or(true, |s| t.status == s))
            .filter(|t| role.map_or(true, |r| t.当前承接人 == r))
            .collect()
    }

    /// 更新道祖需求文档
    pub fn 更新需求文档(&mut self, task_id: u64, doc: RequirementDoc) -> Result<()> {
        let task = self.任务集.get_mut(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
        task.需求文档 = Some(doc);
        task.updated_at = 当前时间戳();
        self.保存()
    }

    /// 更新圣人设计文档
    pub fn 更新设计文档(&mut self, task_id: u64, doc: DesignDoc) -> Result<()> {
        let task = self.任务集.get_mut(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
        task.设计文档 = Some(doc);
        task.updated_at = 当前时间戳();
        self.保存()
    }

    /// 更新大罗金仙实现文档
    pub fn 更新实现文档(&mut self, task_id: u64, doc: ImplementationDoc) -> Result<()> {
        let task = self.任务集.get_mut(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
        task.实现文档 = Some(doc);
        task.updated_at = 当前时间戳();
        self.保存()
    }

    /// 更新准圣验收文档
    pub fn 更新验收文档(&mut self, task_id: u64, doc: VerificationDoc) -> Result<()> {
        let task = self.任务集.get_mut(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
        task.验收文档 = Some(doc);
        task.updated_at = 当前时间戳();
        self.保存()
    }

    /// 更新道祖终审文档
    pub fn 更新终审文档(&mut self, task_id: u64, doc: FinalAcceptanceDoc) -> Result<()> {
        let task = self.任务集.get_mut(&task_id).ok_or_else(|| Error::任务不存在(task_id))?;
        task.终审文档 = Some(doc);
        task.updated_at = 当前时间戳();
        self.保存()
    }

    /// 持久化到文件（JSONL，原子写：先写临时文件再重命名）
    pub fn 保存(&self) -> Result<()> {
        let mut 行集: Vec<String> = Vec::new();
        for task in self.任务集.values() {
            let line = serde_json::to_string(task)
                .map_err(|e| Error::序列化(format!("序列化任务失败: {e}")))?;
            行集.push(line);
        }
        let content = 行集.join("\n");
        let 临时路径 = self.存储路径.with_extension("jsonl.tmp");
        std::fs::write(&临时路径, content).map_err(Error::Io)?;
        std::fs::rename(&临时路径, &self.存储路径).map_err(Error::Io)?;
        Ok(())
    }

    /// 从文件加载；文件不存在时返回空看板
    pub fn 加载(存储路径: impl Into<PathBuf>) -> Result<Self> {
        let path = 存储路径.into();
        if !path.exists() {
            return Ok(TaskBoard::新建(path));
        }
        let content = std::fs::read_to_string(&path).map_err(Error::Io)?;
        let mut 任务集: HashMap<u64, Task> = HashMap::new();
        let mut 最大序号: u64 = 0;
        for line in content.lines().filter(|l| !l.trim().is_empty()) {
            let task: Task = serde_json::from_str(line)
                .map_err(|e| Error::反序列化(format!("解析任务行失败: {e}")))?;
            最大序号 = 最大序号.max(task.id);
            任务集.insert(task.id, task);
        }
        Ok(TaskBoard {
            任务集,
            下一序号: 最大序号 + 1,
            存储路径: path,
            信号总线: None,
        })
    }
}

/// 执行一次状态推进（校验流转 + 记录状态历史 + 更新承接人与统计）
fn 执行推进(task: &mut Task, next: TaskStatus, role: AgentRole, 备注: Option<String>) -> Result<()> {
    if !task.status.可流转到(&next) {
        return Err(Error::状态流转非法(format!("{:?} → {:?}", task.status, next)));
    }
    task.状态历史.push(StatusChange {
        原状态: task.status,
        新状态: next,
        操作者: role,
        时间: 当前时间戳(),
        备注,
    });
    task.status = next;
    task.当前承接人 = role;
    task.updated_at = 当前时间戳();
    if next == TaskStatus::待修复 {
        task.修复轮次 += 1;
    }
    Ok(())
}