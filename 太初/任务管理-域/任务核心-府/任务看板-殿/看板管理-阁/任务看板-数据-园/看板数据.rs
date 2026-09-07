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
    任务标识, 层级记录, 层级记录状态, 产物记录, 回退记录, 五行层级, 澄清记录,
};
use crate::任务看板_殿::{状态归属角色, 承接后状态, 任务依赖图};

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

    /// 发布任务（道祖发布需求）；id 为 0 时自动分配；未初始化的任务标识自动生成（UUID v4 + 时间戳）
    pub fn 发布任务(&mut self, mut task: Task) -> Result<u64> {
        if task.id == 0 {
            task.id = self.下一序号;
        }
        self.下一序号 = self.下一序号.max(task.id + 1);
        if self.任务集.contains_key(&task.id) {
            return Err(Error::Other(format!("任务已存在: {}", task.id)));
        }
        if task.任务标识.未初始化() {
            task.任务标识 = 任务标识::生成(task.title.clone(), None, Vec::new());
        }
        task.当前层级 = 五行层级::木;
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
        // 记录层级处理开始（任务标识驱动：每次承接追加一条处理中层级记录）
        let 层级 = 五行层级::from(role);
        task.层级历史.push(层级记录::开始(层级, role.名称(), 当前时间戳()));
        task.当前层级 = 层级;
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
        // 完成该角色的进行中层级记录（记录完成时间/状态/产物），产物由实现文档等推导
        完成当前层级记录(task, role);
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

    /// 按任务标识（UUID）查询任务
    pub fn 查询标识(&self, 任务id: &uuid::Uuid) -> Option<&Task> {
        self.任务集.values().find(|t| t.任务标识.任务id == *任务id)
    }

    /// 由任务集重建依赖图：节点=全部任务标识，边=(任务, 其依赖任务)
    /// （过渡期依赖关系直接读各任务的任务标识.依赖任务，无需独立持久化）
    pub fn 构建依赖图(&self) -> 任务依赖图 {
        let mut 图 = 任务依赖图::新();
        for task in self.任务集.values() {
            let 标识 = task.任务标识.clone();
            if 标识.未初始化() {
                continue;
            }
            图.添加任务(标识.clone());
            for 依赖 in &标识.依赖任务 {
                图.添加依赖(标识.任务id, *依赖);
            }
        }
        图
    }

    /// 按任务标识（UUID）变更任务状态与回退记录（定向回退等机制在锁内原子完成）
    pub fn 按标识改写(&mut self, 任务id: &uuid::Uuid, 处理: impl FnOnce(&mut Task)) -> Result<()> {
        let task = self
            .任务集
            .values_mut()
            .find(|t| t.任务标识.任务id == *任务id)
            .ok_or_else(|| Error::Other(format!("任务标识 {任务id} 不存在")))?;
        处理(task);
        self.保存()
    }

    /// 定向回退：按根源层级把任务重置到对应层级（过渡期土→「待修复」别名；火→待圣人设计；木→待道祖澄清）。
    /// 写回退记录 + 末条层级记录标已回退 + 更新当前层级 + 状态历史留痕；回退次数 >3 强制升级木（道祖澄清）。
    /// 返回（新状态, 累计回退次数）。
    pub fn 定向回退(
        &mut self,
        任务id: u64,
        根源层级: 五行层级,
        错误描述: &str,
    ) -> Result<(TaskStatus, u32)> {
        let (新状态, 累计) = {
            let task = self.任务集.get_mut(&任务id).ok_or_else(|| Error::任务不存在(任务id))?;
            let 累计 = task.回退来源.as_ref().map(|r| r.回退次数).unwrap_or(0) + 1;
            let 目标 = if 累计 > 3 { 五行层级::木 } else { 根源层级 };
            let 新状态 = match 目标 {
                五行层级::土 => TaskStatus::待修复,
                五行层级::火 => TaskStatus::待圣人设计,
                五行层级::木 => TaskStatus::待道祖澄清,
                五行层级::金 => TaskStatus::待准圣验收,
                五行层级::水 => TaskStatus::待清理,
            };
            let 原因 = match 目标 {
                五行层级::木 => "需求偏差/回退超限升级道祖",
                五行层级::火 => "设计缺陷",
                五行层级::土 => "实现Bug",
                五行层级::金 => "验收问题",
                五行层级::水 => "清理问题",
            };
            let 原状态 = task.status;
            task.回退来源 = Some(回退记录::新(当前时间戳(), 五行层级::金, 目标, 原因, 错误描述, 累计));
            if let Some(末) = task.层级历史.last_mut() {
                if 末.处理中() {
                    let 时间 = 当前时间戳();
                    *末 = 末.clone().标记回退(时间);
                }
            }
            task.当前层级 = 目标;
            task.状态历史.push(StatusChange {
                原状态,
                新状态,
                操作者: task.当前承接人,
                时间: 当前时间戳(),
                备注: Some(format!("定向回退→{}", 目标.名())),
            });
            task.status = 新状态;
            task.updated_at = 当前时间戳();
            (新状态, 累计)
        };
        self.保存()?;
        Ok((新状态, 累计))
    }

    /// 道祖（用户）澄清并推进：仅 `待道祖澄清` 可澄清。
    /// 写入澄清记录 → 承接（待道祖澄清 → 道祖澄清中）→ 按 `继续` 提交（→ 待圣人设计 或 已取消）。
    /// 返回推进后的最终状态。
    pub fn 澄清并推进(&mut self, 任务id: u64, 记录: 澄清记录) -> Result<TaskStatus> {
        let 目标 = if 记录.继续 { TaskStatus::待圣人设计 } else { TaskStatus::已取消 };
        {
            let task = self.任务集.get_mut(&任务id).ok_or_else(|| Error::任务不存在(任务id))?;
            if task.status != TaskStatus::待道祖澄清 {
                return Err(Error::状态流转非法(format!("{:?} 不可澄清（仅待道祖澄清）", task.status)));
            }
            task.澄清记录 = Some(记录);
        }
        // 复用标准流转：承接 → （澄清中）→ 提交到目标，保证状态历史/信号/持久化一致
        self.承接任务(任务id, AgentRole::道祖)?;
        self.提交任务(任务id, AgentRole::道祖, 目标)?;
        Ok(目标)
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

/// 完成该角色最近一条进行中的层级记录（写完成时间/状态/产物）
fn 完成当前层级记录(task: &mut Task, role: AgentRole) {
    let 产物 = 提取产物(task, role);
    let 名称 = role.名称();
    if let Some(i) = task
        .层级历史
        .iter()
        .rposition(|r| r.处理者id == 名称 && r.状态 == 层级记录状态::处理中)
    {
        let 记录 = task.层级历史[i].clone().完成(当前时间戳(), 产物, "提交".to_string());
        task.层级历史[i] = 记录;
    }
}

/// 提取某角色本次提交的产物记录（大罗金仙按实现文档代码变更逐文件提取；其余层暂无文件级产物）
fn 提取产物(task: &Task, role: AgentRole) -> Vec<产物记录> {
    let 关联 = task.任务标识.任务id;
    match role {
        AgentRole::大罗金仙 => task
            .实现文档
            .as_ref()
            .map(|d| {
                d.代码变更
                    .iter()
                    .map(|c| 产物记录::新(c.文件路径.clone(), c.摘要.clone(), 关联))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}