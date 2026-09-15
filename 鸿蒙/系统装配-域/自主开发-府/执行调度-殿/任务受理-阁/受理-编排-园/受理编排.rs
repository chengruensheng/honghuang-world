use std::sync::{Arc, Mutex};
use hm_contract::当前时间戳;
use hm_domain_contract::记忆库契约;
use tc_task::{TaskBoard, TaskPriority, TaskScene};
use crate::{看板驱动台, 道祖接待, 需求摘要};

/// 受理依赖：由调用方（数据服务层）从装配状态中取出并注入，
/// 使受理编排不反向依赖 HTTP 服务层，避免 crate 循环依赖。
pub struct 受理依赖<'a, M> {
    /// 看板驱动台（受理前校验就绪/预占执行通道）
    pub 看板驱动台: &'a Arc<看板驱动台>,
    /// 任务看板（发布待圣人设计的看板任务）
    pub 任务看板: &'a Arc<Mutex<TaskBoard>>,
    /// 道祖接待器（对齐需求确认发布；未装配时 None）
    pub 道祖接待: Option<&'a Arc<Mutex<道祖接待>>>,
    /// 记忆库（对齐需求沉淀记忆）
    pub 记忆库: &'a Arc<Mutex<dyn 记忆库契约<M>>>,
}

/// 受理失败原因（HTTP 层映射状态码，启动入口映射日志）
#[derive(Debug)]
pub enum 受理失败 {
    /// 看板驱动器未装配上线（如 LLM key 缺失）
    未上线,
    /// 已有驱动执行中
    运行中,
    /// 任务文本为空
    任务为空,
    /// 道祖未给出待确认需求（未对齐即确认）
    无待确认,
    /// 看板发布受阻
    创建受阻(String),
}

/// 受理任务标题截断上限（字符数）
const 标题上限: usize = 50;

/// 受理编排：校验 → 看板驱动台就绪/预占 → 发布看板任务（待圣人设计，发起人道祖）→ 自动驱动一轮。
///
/// 需求经此落为看板任务，由五层协作驱动器自主流转（圣人设计→大罗金仙实现→准圣验收→道祖终审）；
/// 返回看板任务 id；失败时已预占的驱动通道回滚释放。
pub fn 受理开发任务<M>(依赖: 受理依赖<'_, M>, 任务: String) -> std::result::Result<u64, 受理失败> {
    let 文案 = 任务.trim().to_string();
    if 文案.is_empty() {
        return Err(受理失败::任务为空);
    }
    if !依赖.看板驱动台.就绪() {
        return Err(受理失败::未上线);
    }
    if !依赖.看板驱动台.预留() {
        return Err(受理失败::运行中);
    }

    let id = match 发布看板任务(&依赖, &文案) {
        Ok(id) => id,
        Err(e) => {
            依赖.看板驱动台.释放();
            tracing::warn!("受理发布看板失败: {e}");
            return Err(受理失败::创建受阻(e.to_string()));
        }
    };

    // 自动驱动到空闲：看板自主流转开始（预占已成功，直接启动；自动驱动一轮 会二次预留失败）
    // 上限20轮：五层协作最多5个阶段，每阶段最多4轮重试，20轮足够覆盖正常+异常场景
    依赖.看板驱动台.启动执行到空闲(20, "发布自动");
    Ok(id)
}

/// 发布看板任务：标题=需求截断（50 字），描述=完整需求，状态=待圣人设计，发起人=道祖。
fn 发布看板任务<M>(依赖: &受理依赖<'_, M>, 文案: &str) -> std::result::Result<u64, hm_error::Error> {
    let 标题 = 截断(文案, 标题上限);
    let mut board = 依赖.任务看板.lock().expect("看板锁中毒");
    let mut task = tc_task::Task::新建(0, 标题, 文案.to_string(), 当前时间戳());
    task.status = tc_task::TaskStatus::待圣人设计;
    task.发起人 = tc_task::AgentRole::道祖;
    board.发布任务(task)
}

/// 按字符数截断文本（任务标题用），超长部分以省略号结尾
fn 截断(文本: &str, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        文本.to_string()
    } else {
        let 头部: String = 文本.chars().take(上限).collect();
        format!("{头部}…")
    }
}

/// 道祖确认发布：预占驱动通道 → 取出对齐需求 → 发布看板（含场景/优先级）→ 写记忆 → 自动驱动一轮。
///
/// 顺序契约：**先预占、后取需求**。反序会在驱动忙时把已对齐需求白白消费掉——
/// 用户拿到「运行中」再重试，只能得到「无待确认需求」，对齐成果丢失、必须重新澄清。
pub fn 确认发布对齐需求<M>(依赖: 受理依赖<'_, M>) -> std::result::Result<u64, 受理失败> {
    if !依赖.看板驱动台.预留() {
        return Err(受理失败::运行中);
    }
    let 需求 = match 取出待确认需求(&依赖) {
        Ok(需求) => 需求,
        Err(e) => {
            依赖.看板驱动台.释放();
            return Err(e);
        }
    };
    let id = match 发布对齐任务(&依赖, &需求) {
        Ok(id) => id,
        Err(e) => {
            依赖.看板驱动台.释放();
            tracing::warn!("发布对齐需求失败: {e}");
            return Err(受理失败::创建受阻(e.to_string()));
        }
    };
    if let Err(e) = 写需求记忆(&依赖, &需求) {
        tracing::warn!("需求记忆写入失败: {e}");
    }
    依赖.看板驱动台.启动执行到空闲(20, "发布自动");
    Ok(id)
}

/// 取出道祖接待器的待确认需求（确认发布并清空会话）
fn 取出待确认需求<M>(依赖: &受理依赖<'_, M>) -> std::result::Result<需求摘要, 受理失败> {
    let 接待 = 依赖.道祖接待.ok_or(受理失败::未上线)?;
    let 接待 = 接待.lock().expect("道祖接待锁中毒");
    接待.确认发布().ok_or(受理失败::无待确认)
}

/// 发布对齐任务：标题=需求标题，描述=需求描述，场景/优先级从摘要解析
fn 发布对齐任务<M>(依赖: &受理依赖<'_, M>, 需求: &需求摘要) -> std::result::Result<u64, hm_error::Error> {
    let 标题 = 截断(&需求.标题, 标题上限);
    let mut board = 依赖.任务看板.lock().expect("看板锁中毒");
    let mut task = tc_task::Task::新建(0, 标题, 需求.描述.clone(), 当前时间戳());
    task.status = tc_task::TaskStatus::待圣人设计;
    task.发起人 = tc_task::AgentRole::道祖;
    if let Some(场景) = 需求.场景.as_deref().and_then(TaskScene::解析) {
        task.场景 = 场景;
    }
    if let Some(优先级) = 需求.优先级.as_deref().and_then(TaskPriority::解析) {
        task.优先级 = 优先级;
    }
    board.发布任务(task)
}

/// 写需求记忆：对齐后的需求摘要落记忆库（闭环五行记忆）
fn 写需求记忆<M>(依赖: &受理依赖<'_, M>, 需求: &需求摘要) -> std::result::Result<u64, hm_error::Error> {
    let 内容 = format!("【需求】{}：{}", 需求.标题, 需求.描述);
    let mut 记忆库 = 依赖.记忆库.lock().expect("记忆库锁中毒");
    记忆库.写入(内容, "道祖对齐".into())
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use hm_content_contract::{工具对话器, 对话消息, 模型响应};
    use hm_contract::Component;

    /// 空对话器：确认发布路径不触达 LLM（只在真接待用户消息时才会用到），
    /// 仅为把 道祖接待 装配起来而存在。
    struct 空对话器;
    impl Component for 空对话器 {
        fn name(&self) -> &'static str {
            "空对话器"
        }
    }
    impl 工具对话器 for 空对话器 {
        fn 对话(&self, _消息: Vec<对话消息>, _工具: Vec<serde_json::Value>) -> hm_error::Result<模型响应> {
            Ok(模型响应 {
                内容: None,
                工具调用: Vec::new(),
                思考: None,
            })
        }
    }

    /// 空记忆库：确认发布的失败路径不写记忆，用最小实现顶住依赖
    struct 空记忆库;
    impl Component for 空记忆库 {
        fn name(&self) -> &'static str {
            "空记忆库"
        }
    }
    impl 记忆库契约<()> for 空记忆库 {
        fn 写入(&mut self, _内容: String, _标签: String) -> hm_error::Result<u64> {
            Ok(0)
        }
        fn 查询(&self, _id: u64) -> Option<&()> {
            None
        }
        fn 按标签(&self, _标签: &str) -> Vec<&()> {
            Vec::new()
        }
        fn 全部(&self) -> Vec<&()> {
            Vec::new()
        }
    }

    fn 临时看板路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("hm_受理编排_{名}.jsonl"))
    }

    /// 顺序契约：驱动忙时必须**先判忙、后取需求**。
    /// 反序（旧实现）会把已对齐需求 take 走再返回「运行中」，用户重试只能得到
    /// 「无待确认需求」：对齐成果丢失、须重新澄清。
    /// 用「无需求的接待器」区分两种实现：先判忙 → 运行中；先取需求 → 无待确认。
    #[test]
    fn 确认发布_驱动忙_判忙先于取需求() {
        let 台 = Arc::new(看板驱动台::新());
        assert!(台.预留(), "预置忙态：预留应成功");
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(临时看板路径("忙"))));
        let 接待 = Arc::new(Mutex::new(道祖接待::新(Arc::new(空对话器))));
        let 记忆: Arc<Mutex<dyn 记忆库契约<()>>> = Arc::new(Mutex::new(空记忆库));
        let 依赖 = 受理依赖 {
            看板驱动台: &台,
            任务看板: &看板,
            道祖接待: Some(&接待),
            记忆库: &记忆,
        };
        let 结果 = 确认发布对齐需求(依赖);
        assert!(
            matches!(结果, Err(受理失败::运行中)),
            "驱动忙应先判忙返回「运行中」，实际: {结果:?}"
        );
    }

    /// 取需求失败必须回滚预占：否则驱动通道被占死，此后一切受理都判「运行中」。
    #[test]
    fn 确认发布_无待确认_回滚预占() {
        let 台 = Arc::new(看板驱动台::新());
        let 看板 = Arc::new(Mutex::new(TaskBoard::新建(临时看板路径("空"))));
        let 接待 = Arc::new(Mutex::new(道祖接待::新(Arc::new(空对话器))));
        let 记忆: Arc<Mutex<dyn 记忆库契约<()>>> = Arc::new(Mutex::new(空记忆库));
        let 依赖 = 受理依赖 {
            看板驱动台: &台,
            任务看板: &看板,
            道祖接待: Some(&接待),
            记忆库: &记忆,
        };
        let 结果 = 确认发布对齐需求(依赖);
        assert!(
            matches!(结果, Err(受理失败::无待确认)),
            "无对齐需求应判「无待确认」，实际: {结果:?}"
        );
        assert!(台.预留(), "取需求失败须回滚预占，否则驱动通道被占死");
    }
}
