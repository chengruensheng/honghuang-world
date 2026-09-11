use std::sync::{Arc, Mutex};
use uuid::Uuid;
use hm_cognition::{AgentRole, ContextManager, 语境消息, 消息角色};
use hm_content_contract::{工具对话器, 对话消息};
use hm_contract::当前时间戳;
use hm_error::{Error, Result};
use hm_execute_contract::{执行器, 开发事件};
use tc_task::{
    Task, TaskBoard, TaskStatus, DesignDoc, ImplementationDoc, VerificationDoc,
    FinalAcceptanceDoc, 五行层级, 状态归属角色,
};
use crate::循环驱动_殿::{智能体, 认知注入, 退化检测器};
use crate::协作驱动_殿::五层驱动_阁::错误追溯_阁::追溯器;
use crate::协作驱动_殿::五层驱动_阁::召回器_阁::召回器;
use crate::任务项;

/// 跨层回退告警阈值：同一任务因验收不通过跨层回退达此次数仍未收敛，输出退化告警
const 回退告警阈值: u32 = 2;
/// 跨层回退熔断阈值：回退达此次数（看板侧累计 >3 即强制升木层→待道祖澄清），
/// 判定为跨层退化循环，终止本轮自动驱动以免继续空转（任务保持「待道祖澄清」等人工介入）
const 回退熔断阈值: u32 = 4;

/// 驱动一轮的结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 驱动结果 {
    /// 看板无待承接任务
    空闲,
    /// 某任务的某阶段完成并提交到新状态
    阶段完成 { 任务id: u64, 角色: AgentRole, 新状态: TaskStatus },
}

/// 五层协作驱动器：确定性控制看板任务的状态流转，LLM 只产出各阶段文档。
///
/// 执行一轮 = 扫描看板 → 找到待承接任务 → 按状态归属角色组装阶段提示 →
/// 调智能体循环执行（LLM 可调用工具实际干活）→ 解析阶段产出为文档 →
/// 承接 + 写文档 + 提交到下一状态（锁内原子完成）。
/// 失败路径：LLM 报错 / 产出非 JSON / 流转非法 → 返回 Err，任务保持待承接可重试。
pub struct 五层协作驱动器 {
    看板: Arc<Mutex<TaskBoard>>,
    上下文: Arc<Mutex<ContextManager>>,
    对话器: Arc<dyn 工具对话器>,
    执行器: Arc<dyn 执行器>,
    最大轮数: usize,
    /// 三态认知注入（可选）：装配后每轮智能体带 推/拉/流 认知，并把过程记录回临时态
    认知: Option<认知注入>,
    /// 过程事件回调（可选）：智能体循环每步（思考/工具调用/工具结果/答复）通知外部
    过程回调: Option<Arc<dyn Fn(u64, &str, &开发事件) + Send + Sync>>,
    /// 工作区根路径（可选）：注入智能体系统提示告知 LLM 用相对路径探索工作区
    工作区根: Option<String>,
    /// 运行检查点回调（可选）：每轮末透传（任务id, 角色名, 轮次, 阶段提示, 消息快照, 任务清单快照）供外部落盘断点
    检查点回调: Option<Arc<dyn Fn(u64, &str, usize, &str, Vec<对话消息>, Vec<任务项>) + Send + Sync>>,
    /// 召回器：定向回退后影响分析召回受影响任务；回退任务重新通过验收后解除召回
    召回器: Arc<召回器>,
    /// 跨轮共享的退化检测器：每轮新建的智能体都注入同一实例，
    /// 使同一任务在五层流转中的同签名工具失败跨轮累积（否则计数随实例创建被重置、熔断永不触发）
    退化检测器: 退化检测器,
    /// 当前正在驱动的任务 id：任务切换时重置退化检测器，避免上一任务的失败账记到新任务
    当前任务id: Mutex<Option<u64>>,
}

impl 五层协作驱动器 {
    pub fn 新(
        看板: Arc<Mutex<TaskBoard>>,
        上下文: Arc<Mutex<ContextManager>>,
        对话器: Arc<dyn 工具对话器>,
        执行器: Arc<dyn 执行器>,
        最大轮数: usize,
    ) -> Self {
        五层协作驱动器 { 看板, 上下文, 对话器, 执行器, 最大轮数, 认知: None, 过程回调: None, 工作区根: None, 检查点回调: None, 召回器: Arc::new(召回器::新()), 退化检测器: 退化检测器::新(), 当前任务id: Mutex::new(None) }
    }

    /// 链式注入召回器（可测试注入带预置事件记录的实例）
    pub fn 装配召回器(mut self, 召回器: Arc<召回器>) -> Self {
        self.召回器 = 召回器;
        self
    }

    /// 链式装配三态认知注入：未装配时行为与旧版完全一致
    pub fn 装配认知(mut self, 认知: 认知注入) -> Self {
        self.认知 = Some(认知);
        self
    }

    /// 链式设置工作区根：透传给智能体，用于注入系统提示
    pub fn 设置工作区(mut self, 根: String) -> Self {
        self.工作区根 = Some(根);
        self
    }

    /// 链式注入过程事件回调：智能体循环每步通知外部（任务id, 角色名, 开发事件）
    pub fn 设置过程回调(mut self, 回调: Arc<dyn Fn(u64, &str, &开发事件) + Send + Sync>) -> Self {
        self.过程回调 = Some(回调);
        self
    }

    /// 链式注入运行检查点回调：每轮末透传（任务id, 角色名, 轮次, 阶段提示, 消息快照, 任务清单快照）供外部落盘断点
    pub fn 设置检查点回调(mut self, 回调: Arc<dyn Fn(u64, &str, usize, &str, Vec<对话消息>, Vec<任务项>) + Send + Sync>) -> Self {
        self.检查点回调 = Some(回调);
        self
    }

    /// 执行一轮：扫描看板找首个可承接任务并推进（无恢复起点）
    pub fn 执行一轮(&self) -> Result<驱动结果> {
        self.执行一轮_恢复(None)
    }

    /// 执行一轮（可选带恢复起点）：`恢复`=Some((任务id, 已存消息)) 时定向推进该任务并基于已存消息继续；
    /// None 时扫描看板任选首个可承接任务。返回 阶段完成 或 空闲。
    pub fn 执行一轮_恢复(&self, 恢复: Option<(u64, Vec<对话消息>)>) -> Result<驱动结果> {
        // 1. 选候选（短锁）：恢复指定任务 or 扫描找首个可承接
        let 快照 = match &恢复 {
            Some((任务id, _)) => {
                let 看板 = self.看板.lock().expect("看板锁中毒");
                match 看板.全部().into_iter().filter(|t| t.id == *任务id).next() {
                    Some(任务) => 任务快照(&任务),
                    None => return Err(Error::Other(format!("恢复目标任务 {任务id} 不存在"))),
                }
            }
            None => {
                let 看板 = self.看板.lock().expect("看板锁中毒");
                match 看板.全部().into_iter().filter(|t| 可承接(t.status)).min_by_key(|t| t.id) {
                    Some(任务) => 任务快照(&任务),
                    None => return Ok(驱动结果::空闲),
                }
            }
        };

        // 2. 判定角色
        let 角色 = 状态归属角色(快照.状态)
            .ok_or_else(|| Error::状态流转非法(format!("{:?} 无可承接角色", 快照.状态)))?;

        // 3. 创建角色上下文
        let 上下文id = {
            let mut 上下文 = self.上下文.lock().expect("上下文锁中毒");
            上下文.创建上下文(角色, Some(快照.id))
        };

        // 3a. 任务切换时重置退化检测器：同签名失败计数只在同一任务内累积，避免上一任务的失败账误伤新任务
        {
            let mut 当前 = self.当前任务id.lock().expect("当前任务锁中毒");
            if *当前 != Some(快照.id) {
                *当前 = Some(快照.id);
                self.退化检测器.重置();
            }
        }

        // 3b. 流态第三态：任务承接即注入其临时规则（任务终态后清除，见 6d）
        if let Some(认知) = &self.认知 {
            if !快照.临时规则.is_empty() {
                认知.注入临时规则(快照.临时规则.clone());
                tracing::info!("任务 #{} 注入临时规则 {} 条", 快照.id, 快照.临时规则.len());
            }
        }

        // 4. 组装阶段提示并执行（智能体循环，LLM 可用工具实际干活）
        let 提示 = 阶段提示(&角色, &快照);
        let mut 智能体 = 智能体::new(self.对话器.clone(), self.执行器.clone(), self.最大轮数);
        // 注入跨轮共享的退化检测器：使同签名工具失败跨轮/跨层累积，而非随本实例创建被重置
        智能体 = 智能体.装配退化检测器(self.退化检测器.clone());
        if let Some(根) = &self.工作区根 {
            智能体 = 智能体.设置工作区(根.clone());
        }
        if let Some(认知) = &self.认知 {
            智能体 = 智能体.装配认知(认知.clone());
        }
        // 注入过程事件回调：把智能体循环每步事件通知外部（带任务id和角色名）
        if let Some(回调) = &self.过程回调 {
            let 任务id = 快照.id;
            let 角色名 = 角色.名称().to_string();
            let 回调克隆 = 回调.clone();
            智能体 = 智能体.设置事件回调(Arc::new(move |事件| {
                回调克隆(任务id, &角色名, 事件);
            }));
        }
        // 注入运行检查点回调：每轮末透传 任务id/角色名/轮次/阶段提示/消息快照/任务清单快照 供外部落盘断点
        if let Some(回调) = &self.检查点回调 {
            let 任务id = 快照.id;
            let 角色名 = 角色.名称().to_string();
            let 阶段提示克隆 = 提示.clone();
            let 回调克隆 = 回调.clone();
            智能体 = 智能体.设置检查点回调(Arc::new(move |轮次, 消息, 清单| {
                回调克隆(任务id, &角色名, 轮次, &阶段提示克隆, 消息.to_vec(), 清单.to_vec());
            }));
        }
        // 运行：恢复起点存在则基于已存消息继续，否则从头
        let 答复 = match &恢复 {
            Some((_, 已存消息)) => 智能体.运行从(提示.clone(), 已存消息.clone())?,
            None => 智能体.运行(提示.clone())?,
        };

        // 5. 解析阶段产出为文档（失败 → 回喂 serde 错误原文重试，上限 2 次；仍失败任务保持待承接可重试）
        let (写文档, 下一状态) = 解析并构造带重试(&角色, &答复, &提示, &智能体)?;

        // 5b. 清理残留核验门：太乙金仙产出解析通过且要推进「清理完成」时，先用执行器实扫工作区
        //     确认 .bak/.tmp 已清空；有残留则返回 Err，任务保持待清理可重试（防模型「自报完成但产物残留」）
        if 角色 == AgentRole::太乙金仙 && 下一状态 == TaskStatus::清理完成 {
            self.清理残留核验()?;
        }

        // 6. 承接 + 写文档 + 提交（锁内原子）；记录是否发生定向回退
        let (实际新状态, 回退信息) = {
            let mut 看板 = self.看板.lock().expect("看板锁中毒");
            看板.承接任务(快照.id, 角色)?;
            写文档(&mut 看板, 快照.id)?;
            if 下一状态 == TaskStatus::待修复 && 角色 == AgentRole::准圣 {
                // 验收不通过：先提交到「待修复」（完成金层级记录），再按任务标识追溯根源定向回退
                看板.提交任务(快照.id, 角色, TaskStatus::待修复)?;
                let (根源层级, 错误描述) = 追溯根源(&快照, &答复);
                let (回退状态, 次数) = 看板.定向回退(快照.id, 根源层级, &错误描述)?;
                tracing::warn!(
                    "任务 #{} 验收不通过，定向回退到 {}（第 {次数} 次，{错误描述}）→ {:?}",
                    快照.id,
                    根源层级.名(),
                    回退状态
                );
                (回退状态, Some((根源层级, 错误描述, 次数)))
            } else {
                看板.提交任务(快照.id, 角色, 下一状态)?;
                (下一状态, None)
            }
        };

        // 6b. 定向回退后的错误传染治理：影响分析 → 召回受影响任务（锁外，召回器内部自行取锁）
        if let Some((_, 错误描述, _)) = &回退信息 {
            let 影响 = {
                let 看板 = self.看板.lock().expect("看板锁中毒");
                self.召回器.影响分析(快照.uuid, &看板.构建依赖图())
            };
            if !影响.is_empty() {
                let 事件们 = {
                    let mut 看板 = self.看板.lock().expect("看板锁中毒");
                    self.召回器.执行召回(快照.uuid, 影响, &错误描述, &mut 看板)
                };
                for 事件 in 事件们 {
                    tracing::warn!(
                        "任务 #{} 回退触发召回 {} 个受影响任务（原因：{}）",
                        快照.id,
                        事件.影响任务.len(),
                        事件.召回原因
                    );
                }
            }
        }

        // 6c. 回退任务重新通过验收（准圣提交通过进入终审）：解除其召回事件
        if 下一状态 == TaskStatus::待道祖终审 && 角色 == AgentRole::准圣 {
            let mut 看板 = self.看板.lock().expect("看板锁中毒");
            let 图 = 看板.构建依赖图();
            let 恢复数 = self.召回器.解除召回(快照.uuid, &图, &mut 看板);
            if 恢复数 > 0 {
                tracing::info!("任务 #{} 通过验收，解除召回并恢复 {恢复数} 个任务", 快照.id);
            }
        }

        // 6d. 任务终态（清理完成）：清除临时规则，规则不外溢到后续任务
        if 实际新状态 == TaskStatus::清理完成 {
            if let Some(认知) = &self.认知 {
                认知.清除临时规则();
                tracing::info!("任务 #{} 已终态（清理完成），临时规则已清除", 快照.id);
            }
        }

        // 7. 上下文记录后清理
        {
            let mut 上下文 = self.上下文.lock().expect("上下文锁中毒");
            if let Err(失败) = 上下文.添加消息(&上下文id, 语境消息 { 角色: 消息角色::用户, 内容: 提示.clone() }) {
                tracing::warn!("上下文记录阶段提示失败: {失败}");
            }
            if let Err(失败) = 上下文.添加消息(&上下文id, 语境消息 { 角色: 消息角色::助手, 内容: 答复.clone() }) {
                tracing::warn!("上下文记录阶段答复失败: {失败}");
            }
            if let Err(失败) = 上下文.清理上下文(&上下文id) {
                tracing::warn!("上下文清理失败: {失败}");
            }
        }

        // 8. 跨层回退熔断：同一任务「验收不通过 → 跨层回退」反复达阈值 ⇒ 分级告警；超限强制熔断终止本轮驱动。
        //    看板侧回退累计 >3 已把任务强制升木（→「待道祖澄清」），此处再以熔断语义（error 日志 + Err）
        //    提前掐断「验收不通过→回退→再验收」的跨层空转，而非让驱动静默空闲（2026-09-11 真机实测回退 4 次）。
        if let Some((_, _, 次数)) = 回退信息 {
            if 次数 >= 回退熔断阈值 {
                let 提示 = format!(
                    "任务 #{} 已连续 {} 次验收不通过并跨层回退，判定为跨层退化循环，已终止本轮自动驱动以防空转（任务已转入「待道祖澄清」等待人工介入）。",
                    快照.id, 次数
                );
                tracing::error!("【退化循环熔断】{}", 提示);
                return Err(Error::退化循环熔断(提示));
            }
            if 次数 >= 回退告警阈值 {
                tracing::warn!(
                    "【退化循环告警】任务 #{} 已跨层回退 {} 次仍未收敛（熔断阈值 {} 次）",
                    快照.id, 次数, 回退熔断阈值
                );
            }
        }

        Ok(驱动结果::阶段完成 { 任务id: 快照.id, 角色, 新状态: 实际新状态 })
    }

    /// 连续驱动直到空闲或达到上限
    pub fn 执行到空闲(&self, 上限: usize) -> Result<Vec<驱动结果>> {
        let mut 结果 = Vec::new();
        for _ in 0..上限 {
            match self.执行一轮()? {
                驱动结果::空闲 => break,
                r => 结果.push(r),
            }
        }
        Ok(结果)
    }

    /// 供外部注入事件回调的便捷方法（转发给循环的事件回调在此不展开，MVP 由调用方直接构造智能体）
    pub fn 看板(&self) -> Arc<Mutex<TaskBoard>> {
        self.看板.clone()
    }

    /// 清理残留机器核验门：太乙金仙宣告「清理完成」前，扫描工作区内 `.bak`/`.tmp` 临时产物；
    /// 存在残留则拒绝推进（任务保持待清理可重试），杜绝模型「自报清理完成但产物真实残留」的虚假完成。
    fn 清理残留核验(&self) -> Result<()> {
        let mut 残留: Vec<String> = Vec::new();
        for 模式 in ["**/*.bak", "**/*.tmp"] {
            match self.执行器.按名找文件(模式) {
                Ok(输出) => {
                    for 行 in 输出.lines() {
                        let 行 = 行.trim();
                        if !行.is_empty() && 行 != "（无匹配）" {
                            残留.push(行.to_string());
                        }
                    }
                }
                Err(e) => tracing::warn!("清理残留核验扫描 {模式} 失败: {e}"),
            }
        }
        if 残留.is_empty() {
            Ok(())
        } else {
            Err(Error::Other(format!(
                "清理残留核验未通过：工作区仍存在 {} 个临时/备份文件（{}），已拒绝推进「清理完成」；请用「删除文件」工具清理后再重试",
                残留.len(),
                残留.join("、")
            )))
        }
    }

    /// Resume 前置闸门：校验任务当前状态是否仍可由 `角色名` 承接（即该检查点对应阶段尚未推进成功）。
    /// 已推进（当前归属角色 ≠ 检查点角色 / 状态不可承接）时返回 Err(原因)，HTTP 层映射 400「无需恢复」。
    pub fn 恢复闸门(&self, 任务id: u64, 角色名: &str) -> std::result::Result<(), String> {
        let 状态 = {
            let 看板 = self.看板.lock().expect("看板锁中毒");
            看板.全部().into_iter().find(|t| t.id == 任务id).map(|t| t.status)
        };
        match 状态 {
            None => Err(format!("任务 {任务id} 不存在，无法恢复")),
            Some(状态) => match 状态归属角色(状态) {
                Some(角色) if 角色.名称() == 角色名 => Ok(()),
                Some(角色) => Err(format!(
                    "任务 {任务id} 已推进（现应由 {} 承接），检查点角色 {} 的阶段已完成，无需恢复",
                    角色.名称(),
                    角色名
                )),
                None => Err(format!("任务 {任务id} 状态 {:?} 不可承接，无需恢复", 状态)),
            },
        }
    }
}

/// 任务快照：短锁内复制所需字段，避免执行期间持锁
struct 任务快照 {
    id: u64,
    /// 任务唯一标识（UUID，定向回退/召回按标识追溯）
    uuid: Uuid,
    标题: String,
    描述: String,
    状态: TaskStatus,
    需求: Option<String>,
    设计: Option<String>,
    实现: Option<String>,
    验收: Option<String>,
    终审: Option<String>,
    /// 结构化阶段文档（供错误追溯器纯规则判定）
    设计对象: Option<DesignDoc>,
    实现对象: Option<ImplementationDoc>,
    /// 任务级临时规则（流态第三态：承接时注入，任务终态后清除）
    临时规则: Vec<String>,
}

fn 任务快照(t: &Task) -> 任务快照 {
    任务快照 {
        id: t.id,
        uuid: t.任务标识.任务id,
        标题: t.title.clone(),
        描述: t.description.clone(),
        状态: t.status,
        需求: t.需求文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        设计: t.设计文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        实现: t.实现文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        验收: t.验收文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        终审: t.终审文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        设计对象: t.设计文档.clone(),
        实现对象: t.实现文档.clone(),
        临时规则: t.临时规则.clone(),
    }
}

/// 可承接状态集合（与流转逻辑-园的状态归属一致；
/// 待道祖澄清/道祖澄清中 需道祖人工澄清，不纳入自动驱动候选，避免被误当终审；
/// 待重新* 为召回暂停态，也不纳入自动驱动候选，待解除召回后恢复）
fn 可承接(状态: TaskStatus) -> bool {
    matches!(
        状态,
        TaskStatus::待圣人设计
            | TaskStatus::待大罗金仙实现
            | TaskStatus::待准圣验收
            | TaskStatus::待道祖终审
            | TaskStatus::待修复
            | TaskStatus::待清理
            | TaskStatus::清理中
    )
}

/// 组装阶段提示：角色使命 + 任务信息 + 已有文档 + 本阶段输出要求
fn 阶段提示(角色: &AgentRole, 快照: &任务快照) -> String {
    let 已有文档 = 拼接已有文档(快照);
    let 阶段要求 = match 角色 {
        AgentRole::圣人 => {
            "你的任务是【设计】。基于需求设计实现方案，输出**严格符合以下类型**的设计文档 JSON。\n\
             关键约束：边界定义是一个「键=>纯文本字符串」的对象，其每个键的值**只能是字符串**，绝不能是对象或数组；安全区域/修改文件/新建文件是字符串数组；契约/依赖是对象数组，其中各字段的值均为字符串。\n\
             严格示例（字段名与值类型必须与之一致）：\n\
             {\"边界定义\":{\"输入边界\":\"字符串\",\"输出边界\":\"字符串\"},\"安全区域\":[],\"契约\":[{\"契约名\":\"\",\"方法\":[{\"名称\":\"\",\"签名\":\"\",\"描述\":\"\"}],\"描述\":\"\"}],\"修改文件\":[],\"新建文件\":[],\"依赖\":[{\"来源模块\":\"\",\"目标模块\":\"\",\"描述\":\"\"}]}"
        }
        AgentRole::大罗金仙 => {
            "你的任务是【实现】。在工作区实际实现设计（可用读文件/写文件/运行命令/搜索内容/精确编辑等工具），\
             改完必须运行 cargo build 与 cargo test 验证。完成后输出实现文档 JSON：\
             {\"代码变更\":[{\"文件路径\":\"\",\"变更类型\":\"\",\"摘要\":\"\"}],\
             \"自检\":{\"通过\":true,\"边界合规\":true,\"契约合规\":true,\"问题\":[]}}"
        }
        AgentRole::准圣 => {
            "你的任务是【验收】。逐项对照需求与实现进行验收（可运行 cargo test 等），\
             输出验收文档 JSON：\
             {\"轮次\":[{\"轮次\":1,\"通过\":true,\"边界检查\":true,\"契约检查\":true,\"安全检查\":true,\"事实检查\":true,\"完整性检查\":true,\"问题\":[],\"建议\":\"\"}],\
             \"最终结果\":true}"
        }
        AgentRole::道祖 => {
            "你的任务是【终审】。综合审查需求/设计/实现/验收全部文档，做最终决策。\
             输出终审文档 JSON：\
             {\"通过\":true,\"需求满足度\":10,\"可维护性\":9,\"代码质量\":9,\"风险评估\":\"\",\"评语\":\"\"}"
        }
        AgentRole::太乙金仙 => {
            "你的任务是【清理】。对已终审通过的任务做收尾清理：核对产物、归档、移除临时文件。\n\
             清理纪律（强制，否则清理会被机器核验驳回）：\n\
             1) 先用「按名找文件」扫描工作区残留临时/备份文件，工具调用必须显式携带模式参数，\
             形如 {\"模式\":\"**/*.bak\"} 与 {\"模式\":\"**/*.tmp\"}（参数为空会被拒绝，禁止省略）；\n\
             2) 对每个残留文件调用「删除文件」工具逐一删除，形如 {\"路径\":\"<扫描返回的相对路径>\"}；\n\
             3) 删除后再用「按名找文件」重新扫描同一模式，确认已无匹配（返回（无匹配））才可宣告完成；\n\
             4) 若工具调用失败，必须读取错误信息修正后重试，不得在工具失败时直接宣告清理完成（机器会实扫工作区，谎报必被驳回）。\n\
             输出清理记录 JSON：\n\
             {\"清理项\":[{\"项\":\"\",\"结果\":\"已清理\"}],\"归档完成\":true}"
        }
    };
    format!(
        "你是{角色}（{职责}），在「洪荒·世界」项目五层协作中负责本阶段。\n\
         \n\
         任务 #{id}：{标题}\n\
         任务描述：{描述}\n\
         \n\
         {已有文档}\n\
         \n\
         {阶段要求}\n\
         请只输出 JSON，不要输出任何解释文字。",
        角色 = 角色.名称(),
        职责 = 角色.职责(),
        id = 快照.id,
        标题 = 快照.标题,
        描述 = 快照.描述,
        已有文档 = 已有文档,
        阶段要求 = 阶段要求,
    )
}

fn 拼接已有文档(快照: &任务快照) -> String {
    let mut 段 = Vec::new();
    if let Some(d) = &快照.需求 { 段.push(format!("【需求文档】\n{d}")); }
    if let Some(d) = &快照.设计 { 段.push(format!("【设计文档】\n{d}")); }
    if let Some(d) = &快照.实现 { 段.push(format!("【实现文档】\n{d}")); }
    if let Some(d) = &快照.验收 { 段.push(format!("【验收文档】\n{d}")); }
    if let Some(d) = &快照.终审 { 段.push(format!("【终审文档】\n{d}")); }
    if 段.is_empty() {
        "（暂无已有阶段文档）".to_string()
    } else {
        段.join("\n\n")
    }
}

/// 解析阶段产出：提取 JSON → 注入 created_at → 反序列化为目标文档 → 返回（写文档闭包, 下一状态）
fn 解析并构造(
    角色: &AgentRole,
    答复: &str,
) -> Result<(Box<dyn FnOnce(&mut TaskBoard, u64) -> Result<()>>, TaskStatus)> {
    // LLM 常在 JSON 前包裹 <think>...</think> 思考标签或 ```json 代码块，
    // 提取json 会从第一个 { 开始匹配，可能抓到 think 内部的碎片 JSON 而非真正的阶段产出。
    // 先剥离这些杂质再提取，避免误抓导致的反序列化失败回喂重试浪费轮次。
    let 净化答复 = 剥离杂质标签(答复);
    let json = 提取json(&净化答复)
        .ok_or_else(|| Error::反序列化(format!("阶段产出无 JSON（答复前 200 字：{}）", 截断(答复, 200))))?;
    let mut 值: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| Error::反序列化(format!("阶段产出非合法 JSON: {e}；前 200 字：{}", 截断(&json, 200))))?;
    // 阶段文档的 created_at 为必填字段，由驱动器统一注入
    if let serde_json::Value::Object(map) = &mut 值 {
        map.insert("created_at".to_string(), serde_json::json!(当前时间戳()));
    } else {
        return Err(Error::反序列化(format!("阶段产出 JSON 顶层必须是对象，前 200 字：{}", 截断(&json, 200))));
    }
    match 角色 {
        AgentRole::圣人 => {
            let doc: DesignDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("设计文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            Ok((Box::new(move |看板, id| 看板.更新设计文档(id, doc)), TaskStatus::待大罗金仙实现))
        }
        AgentRole::大罗金仙 => {
            let doc: ImplementationDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("实现文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            Ok((Box::new(move |看板, id| 看板.更新实现文档(id, doc)), TaskStatus::待准圣验收))
        }
        AgentRole::准圣 => {
            let doc: VerificationDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("验收文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            let 下一状态 = if doc.最终结果 { TaskStatus::待道祖终审 } else { TaskStatus::待修复 };
            Ok((Box::new(move |看板, id| 看板.更新验收文档(id, doc)), 下一状态))
        }
        AgentRole::道祖 => {
            let doc: FinalAcceptanceDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("终审文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            let 下一状态 = if doc.通过 { TaskStatus::待清理 } else { TaskStatus::待修复 };
            Ok((Box::new(move |看板, id| 看板.更新终审文档(id, doc)), 下一状态))
        }
        AgentRole::太乙金仙 => {
            // 清理阶段无文档槽位（Task 未扩展字段）：校验顶层对象即可，推进到 清理完成
            if !值.is_object() {
                return Err(Error::反序列化(format!("清理记录 JSON 顶层必须是对象，前 200 字：{}", 截断(&json, 200))));
            }
            Ok((Box::new(|_看板, _id| Ok(())), TaskStatus::清理完成))
        }
    }
}

/// 解析阶段产出（带失败重试）：首次解析失败时，把 serde 错误原文回喂 LLM 修正重试，上限 2 次；
/// 重试仍失败才返回 Err（任务保持待承接可重试，不死等）。
fn 解析并构造带重试(
    角色: &AgentRole,
    答复: &str,
    阶段提示: &str,
    智能体: &智能体,
) -> Result<(Box<dyn FnOnce(&mut TaskBoard, u64) -> Result<()>>, TaskStatus)> {
    let mut 当前答复 = 答复.to_string();
    let mut 重试 = 0;
    loop {
        match 解析并构造(角色, &当前答复) {
            Ok(结果) => return Ok(结果),
            Err(错误) => {
                if 重试 >= 2 {
                    return Err(错误);
                }
                重试 += 1;
                let 修复提示 = format!(
                    "{}\n\n【修正要求】你上一轮输出的内容无法解析为合法 JSON，具体错误：\n{}\n请严格只输出一个 JSON 对象，不要输出任何解释文字，不要用 markdown 代码块包裹。重新输出。",
                    阶段提示, 错误
                );
                tracing::warn!("阶段产出解析失败，回喂错误重试（第 {}/2 次）：{}", 重试, 错误);
                当前答复 = 智能体.运行(修复提示)?;
            }
        }
    }
}

/// 提取答复中的 JSON 对象子串（括号配平，容忍前后解释文字与尾部杂质）
///
/// 逐字符扫描：首个 `{` 入栈，配平到栈空为止。忽略字符串内与转义序列中的括号，
/// 避免 LLM 在 JSON 后追加解释/代码块标记导致 rfind 取错闭合符。
/// 剥离 LLM 输出中的杂质标签（...、```json...```），
/// 避免 提取json 误抓 think 内部的碎片 JSON。
/// 支持大小写不敏感匹配、多段出现；保留标签外的正文内容。
/// 注意：<think>/``` 均为纯 ASCII，可直接在 UTF-8 字节流上匹配，
/// 避免 to_lowercase 改变多字节字符长度导致的偏移错位。
fn 剥离杂质标签(文本: &str) -> String {
    let mut 结果 = String::with_capacity(文本.len());
    let bytes = 文本.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // 检测 <think> 开始标签（7 字节纯 ASCII，大小写不敏感）
        if i + 7 <= len && bytes[i] == b'<' {
            let 候选 = &bytes[i..i + 7];
            if 候选.eq_ignore_ascii_case(b"<think>") {
                // 找 </think> 结束标签（8 字节纯 ASCII）
                if let Some(相对偏移) = 文本[i + 7..].find("</think>") {
                    // 跳过整个 <think>...</think> 块
                    i += 7 + 相对偏移 + 8;
                    continue;
                }
                // 无闭合标签：跳过 <think> 标记本身
                i += 7;
                continue;
            }
        }
        // 检测 ``` 代码块围栏（3 字节纯 ASCII）
        if i + 3 <= len && &bytes[i..i + 3] == b"```" {
            if let Some(相对偏移) = 文本[i + 3..].find("```") {
                let 块内容 = &文本[i + 3..i + 3 + 相对偏移];
                // 去掉首行可能的语言标识（如 "json\n"）
                let 内容起始 = if let Some(换行位置) = 块内容.find('\n') {
                    let 首行 = &块内容[..换行位置];
                    if 首行.trim().chars().all(|c| c.is_ascii_alphabetic()) {
                        换行位置 + 1
                    } else {
                        0
                    }
                } else {
                    0
                };
                结果.push_str(&块内容[内容起始..]);
                i += 3 + 相对偏移 + 3;
                continue;
            }
        }
        // 安全推进：UTF-8 首字节决定字符宽度，避免截断多字节字符
        let 字符宽度 = utf8_char_width(bytes[i]);
        let end = (i + 字符宽度).min(len);
        结果.push_str(&文本[i..end]);
        i = end;
    }
    结果
}

/// UTF-8 首字节 → 字符字节宽度（无效前缀当 1 字节处理）
fn utf8_char_width(首字节: u8) -> usize {
    match 首字节 {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1, // 无效续字节或孤立字节，安全跳过 1
    }
}

fn 提取json(文本: &str) -> Option<String> {
    let 开始 = 文本.find('{')?;
    let mut 深度 = 0i32;
    let mut 在字符串 = false;
    let mut 转义 = false;
    for (i, ch) in 文本[开始..].char_indices() {
        if 在字符串 {
            if 转义 {
                转义 = false;
            } else if ch == '\\' {
                转义 = true;
            } else if ch == '"' {
                在字符串 = false;
            }
            continue;
        }
        match ch {
            '"' => 在字符串 = true,
            '{' => 深度 += 1,
            '}' => {
                深度 -= 1;
                if 深度 == 0 {
                    return Some(文本[开始..=开始 + i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn 截断(文本: &str, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        文本.to_string()
    } else {
        let 头部: String = 文本.chars().take(上限).collect();
        format!("{头部}…")
    }
}

#[cfg(test)]
mod tests {
    use super::{阶段提示, 任务快照, 提取json, 剥离杂质标签};
    use hm_cognition::AgentRole;
    use tc_task::Task;

    #[test]
    fn 阶段提示_太乙金仙_含清理纪律() {
        let 任务 = Task::新建(1, "清理纪律任务".to_string(), "描述".to_string(), 0);
        let 快照 = 任务快照(&任务);
        let 提示 = 阶段提示(&AgentRole::太乙金仙, &快照);
        assert!(提示.contains("按名找文件"), "太乙金仙提示应先扫描残留，实际: {提示}");
        assert!(提示.contains("删除文件"), "太乙金仙提示应要求用删除文件工具清理，实际: {提示}");
        assert!(提示.contains("{\"模式\":\"**/*.bak\"}"), "太乙金仙提示应给出显式模式参数示例（防空参连败），实际: {提示}");
        assert!(提示.contains("重新扫描"), "太乙金仙提示应要求删除后复核为空，实际: {提示}");
        assert!(提示.contains("不得在工具失败时直接宣告清理完成"), "太乙金仙提示应禁止失败即宣告，实际: {提示}");
    }

    #[test]
    fn 提取json_无杂质_原样返回() {
        let 输入 = r#"{"a":1}"#;
        assert_eq!(提取json(输入).as_deref(), Some(r#"{"a":1}"#));
    }

    #[test]
    fn 提取json_后附代码块标记_正确截断() {
        let 输入 = "以下是设计文档：\n```json\n{\"边界定义\":{\"输入\":\"n\"},\"输出\":{}}\n```\n完毕";
        let 提取 = 提取json(输入).expect("应提取到 JSON");
        // 截取到配平的 }，不含尾随 ``` 标记
        assert_eq!(提取, r#"{"边界定义":{"输入":"n"},"输出":{}}"#);
    }

    #[test]
    fn 提取json_字符串内含括号_不误判() {
        // 字符串值里含 { 与 }，括号配平应跳过字符串内容
        let 输入 = r#"{"描述":"斐波那契 F(n) 用 {} 表示边界","值":1}"#;
        assert_eq!(
            提取json(输入).as_deref(),
            Some(r#"{"描述":"斐波那契 F(n) 用 {} 表示边界","值":1}"#)
        );
    }

    #[test]
    fn 提取json_嵌套对象_取最外层() {
        let 输入 = r#"{"轮次":[{"通过":true,"问题":["a","b"]}],"最终结果":true}"#;
        let 提取 = 提取json(输入).expect("应提取到嵌套 JSON");
        assert_eq!(提取, r#"{"轮次":[{"通过":true,"问题":["a","b"]}],"最终结果":true}"#);
    }

    #[test]
    fn 提取json_无括号_返回空() {
        assert!(提取json("没有 JSON 内容").is_none());
    }

    #[test]
    fn 提取json_只有左括号未闭合_返回空() {
        assert!(提取json("内容 { 未闭合").is_none());
    }

    #[test]
    fn 剥离杂质_think标签_完整剥离() {
        let 输入 = r#"<think>内部推理{"key":"val"}</think>

{"代码变更":[],"自检":{}}"#;
        let 净化 = 剥离杂质标签(输入);
        assert!(!净化.contains("内部推理"), "think 内容应被剥离");
        assert!(!净化.contains("<think>"), "think 标签应被剥离");
        let json = 提取json(&净化).expect("应提取到正文 JSON");
        assert!(json.contains("代码变更"), "应提取到正文 JSON 而非 think 内碎片");
    }

    #[test]
    fn 剥离杂质_think大小写不敏感() {
        let 输入 = r#"<Think>思考过程</Think>{"结果":true}"#;
        let 净化 = 剥离杂质标签(输入);
        let json = 提取json(&净化).expect("大写 Think 也应剥离");
        assert_eq!(json, r#"{"结果":true}"#);
    }

    #[test]
    fn 剥离杂质_代码块围栏_提取内容() {
        let 输入 = "以下是文档：\n```json\n{\"边界定义\":{\"输入\":\"n\"}}\n```\n完毕";
        let 净化 = 剥离杂质标签(输入);
        let json = 提取json(&净化).expect("应从代码块中提取 JSON");
        assert_eq!(json, r#"{"边界定义":{"输入":"n"}}"#);
    }

    #[test]
    fn 剥离杂质_无杂质_原样保留() {
        let 输入 = r#"{"直接":"json","值":42}"#;
        assert_eq!(剥离杂质标签(输入), 输入);
    }

    #[test]
    fn 剥离杂质_think未闭合_跳过标签保留后续() {
        let 输入 = r#"<think>未闭合的思考{"正文":true}"#;
        let 净化 = 剥离杂质标签(输入);
        // 未闭合时跳过  标记本身，后续内容当正文
        let json = 提取json(&净化).expect("未闭合 think 后仍应提取 JSON");
        assert_eq!(json, r#"{"正文":true}"#);
    }
}

/// 验收不通过时的错误根源追溯（纯规则，不用 LLM）：
/// 用结构化设计/实现文档 + 验收答复判定根源层级，返回（根源层级, 错误描述）
fn 追溯根源(快照: &任务快照, 验收答复: &str) -> (五行层级, String) {
    let 现象 = 提取错误现象(验收答复);
    let 追溯器 = 追溯器::新();
    let 包 = 追溯器.追溯(
        快照.uuid,
        &现象,
        快照.设计对象.as_ref(),
        快照.实现对象.as_ref(),
        &快照.描述,
    );
    let 描述 = if 包.错误描述.is_empty() { 现象 } else { 包.错误描述 };
    (包.根源层级, 描述)
}

/// 从验收答复 JSON 提取错误现象（轮次问题/建议 文本）
fn 提取错误现象(答复: &str) -> String {
    let Some(json) = 提取json(答复) else {
        return 截断(答复, 200).to_string();
    };
    let Ok(值) = serde_json::from_str::<serde_json::Value>(&json) else {
        return 截断(答复, 200).to_string();
    };
    let mut 片段 = Vec::new();
    if let Some(轮次) = 值.get("轮次").and_then(|v| v.as_array()) {
        for 轮 in 轮次 {
            if let Some(问题) = 轮.get("问题").and_then(|v| v.as_array()) {
                for q in 问题 {
                    if let Some(s) = q.as_str() {
                        片段.push(s.to_string());
                    }
                }
            }
            if let Some(建议) = 轮.get("建议").and_then(|v| v.as_str()) {
                片段.push(建议.to_string());
            }
        }
    }
    if 片段.is_empty() {
        截断(答复, 200).to_string()
    } else {
        片段.join("；")
    }
}
