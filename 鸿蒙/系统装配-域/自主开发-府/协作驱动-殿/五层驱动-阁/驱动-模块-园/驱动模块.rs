use std::sync::{Arc, Mutex};
use hm_cognition::{AgentRole, ContextManager, 语境消息, 消息角色};
use hm_content_contract::工具对话器;
use hm_contract::当前时间戳;
use hm_error::{Error, Result};
use hm_execute_contract::执行器;
use tc_task::{
    Task, TaskBoard, TaskStatus, DesignDoc, ImplementationDoc, VerificationDoc,
    FinalAcceptanceDoc,
};
use crate::循环驱动_殿::{智能体, 认知注入};

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
}

impl 五层协作驱动器 {
    pub fn 新(
        看板: Arc<Mutex<TaskBoard>>,
        上下文: Arc<Mutex<ContextManager>>,
        对话器: Arc<dyn 工具对话器>,
        执行器: Arc<dyn 执行器>,
        最大轮数: usize,
    ) -> Self {
        五层协作驱动器 { 看板, 上下文, 对话器, 执行器, 最大轮数, 认知: None }
    }

    /// 链式装配三态认知注入：未装配时行为与旧版完全一致
    pub fn 装配认知(mut self, 认知: 认知注入) -> Self {
        self.认知 = Some(认知);
        self
    }

    /// 执行一轮：返回 阶段完成 或 空闲
    pub fn 执行一轮(&self) -> Result<驱动结果> {
        // 1. 扫描看板找待承接任务（短锁）
        let 候选 = {
            let 看板 = self.看板.lock().expect("看板锁中毒");
            看板.全部()
                .into_iter()
                .filter(|t| 可承接(t.status))
                .min_by_key(|t| t.id)
                .map(|t| 任务快照(t))
        };
        let Some(快照) = 候选 else {
            return Ok(驱动结果::空闲);
        };

        // 2. 判定角色
        let 角色 = 状态归属角色(快照.状态)
            .ok_or_else(|| Error::状态流转非法(format!("{:?} 无可承接角色", 快照.状态)))?;

        // 3. 创建角色上下文
        let 上下文id = {
            let mut 上下文 = self.上下文.lock().expect("上下文锁中毒");
            上下文.创建上下文(角色, Some(快照.id))
        };

        // 4. 组装阶段提示并执行（智能体循环，LLM 可用工具实际干活）
        let 提示 = 阶段提示(&角色, &快照);
        let mut 智能体 = 智能体::new(self.对话器.clone(), self.执行器.clone(), self.最大轮数);
        if let Some(认知) = &self.认知 {
            智能体 = 智能体.装配认知(认知.clone());
        }
        let 答复 = 智能体.运行(提示.clone())?;

        // 5. 解析阶段产出为文档（失败 → 任务保持待承接，可重试）
        let (写文档, 下一状态) = 解析并构造(&角色, &答复)?;

        // 6. 承接 + 写文档 + 提交（锁内原子）
        {
            let mut 看板 = self.看板.lock().expect("看板锁中毒");
            看板.承接任务(快照.id, 角色)?;
            写文档(&mut 看板, 快照.id)?;
            看板.提交任务(快照.id, 角色, 下一状态)?;
        }

        // 7. 上下文记录后清理
        {
            let mut 上下文 = self.上下文.lock().expect("上下文锁中毒");
            let _ = 上下文.添加消息(&上下文id, 语境消息 { 角色: 消息角色::用户, 内容: 提示.clone() });
            let _ = 上下文.添加消息(&上下文id, 语境消息 { 角色: 消息角色::助手, 内容: 答复.clone() });
            let _ = 上下文.清理上下文(&上下文id);
        }

        Ok(驱动结果::阶段完成 { 任务id: 快照.id, 角色, 新状态: 下一状态 })
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
}

/// 任务快照：短锁内复制所需字段，避免执行期间持锁
struct 任务快照 {
    id: u64,
    标题: String,
    描述: String,
    状态: TaskStatus,
    需求: Option<String>,
    设计: Option<String>,
    实现: Option<String>,
    验收: Option<String>,
    终审: Option<String>,
}

fn 任务快照(t: &Task) -> 任务快照 {
    任务快照 {
        id: t.id,
        标题: t.title.clone(),
        描述: t.description.clone(),
        状态: t.status,
        需求: t.需求文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        设计: t.设计文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        实现: t.实现文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        验收: t.验收文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
        终审: t.终审文档.as_ref().map(|d| serde_json::to_string_pretty(d).unwrap_or_default()),
    }
}

/// 可承接状态集合（与流转逻辑-园的状态归属一致）
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

/// 状态 → 归属角色（与流转逻辑-园 状态归属角色 保持一致）
fn 状态归属角色(状态: TaskStatus) -> Option<AgentRole> {
    match 状态 {
        TaskStatus::待圣人设计 => Some(AgentRole::圣人),
        TaskStatus::待大罗金仙实现 | TaskStatus::待修复 => Some(AgentRole::大罗金仙),
        TaskStatus::待准圣验收 => Some(AgentRole::准圣),
        TaskStatus::待道祖终审 => Some(AgentRole::道祖),
        TaskStatus::待清理 | TaskStatus::清理中 => Some(AgentRole::太乙金仙),
        _ => None,
    }
}

/// 组装阶段提示：角色使命 + 任务信息 + 已有文档 + 本阶段输出要求
fn 阶段提示(角色: &AgentRole, 快照: &任务快照) -> String {
    let 已有文档 = 拼接已有文档(快照);
    let 阶段要求 = match 角色 {
        AgentRole::圣人 => {
            "你的任务是【设计】。基于需求设计实现方案，输出设计文档 JSON：\
             {\"边界定义\":{},\"安全区域\":[],\"契约\":[{\"契约名\":\"\",\"方法\":[{\"名称\":\"\",\"签名\":\"\",\"描述\":\"\"}],\"描述\":\"\"}],\
             \"修改文件\":[],\"新建文件\":[],\"依赖\":[{\"来源模块\":\"\",\"目标模块\":\"\",\"描述\":\"\"}]}"
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
            "你的任务是【清理】。对已终审通过的任务做收尾清理：核对产物、归档、移除临时文件。\
             输出清理记录 JSON：\
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
        职责 = 角色职责(角色),
        id = 快照.id,
        标题 = 快照.标题,
        描述 = 快照.描述,
        已有文档 = 已有文档,
        阶段要求 = 阶段要求,
    )
}

fn 角色职责(角色: &AgentRole) -> &'static str {
    match 角色 {
        AgentRole::道祖 => "决策与终审",
        AgentRole::圣人 => "边界契约设计",
        AgentRole::大罗金仙 => "代码实现与自检",
        AgentRole::准圣 => "逐项验收",
        AgentRole::太乙金仙 => "清理与归档",
    }
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
    let json = 提取json(答复)
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

/// 提取答复中的 JSON 对象子串（第一个 { 到最后一个 }，容忍前后解释文字）
fn 提取json(文本: &str) -> Option<String> {
    let 开始 = 文本.find('{')?;
    let 结束 = 文本.rfind('}')?;
    if 结束 <= 开始 {
        return None;
    }
    Some(文本[开始..=结束].to_string())
}

fn 截断(文本: &str, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        文本.to_string()
    } else {
        let 头部: String = 文本.chars().take(上限).collect();
        format!("{头部}…")
    }
}
