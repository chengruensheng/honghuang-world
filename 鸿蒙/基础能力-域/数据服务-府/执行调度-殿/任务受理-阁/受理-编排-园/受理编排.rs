use hm_agent::需求摘要;
use tc_task::{TaskPriority, TaskScene};
use crate::数据服务状态;


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
pub fn 受理开发任务(状态: &数据服务状态, 任务: String) -> std::result::Result<u64, 受理失败> {
    let 文案 = 任务.trim().to_string();
    if 文案.is_empty() {
        return Err(受理失败::任务为空);
    }
    if !状态.看板驱动台.就绪() {
        return Err(受理失败::未上线);
    }
    if !状态.看板驱动台.预留() {
        return Err(受理失败::运行中);
    }

    let id = match 发布看板任务(状态, &文案) {
        Ok(id) => id,
        Err(e) => {
            状态.看板驱动台.释放();
            tracing::warn!("受理发布看板失败: {e}");
            return Err(受理失败::创建受阻(e.to_string()));
        }
    };

    // 自动驱动一轮：看板自主流转开始（预占已成功，直接启动；自动驱动一轮 会二次预留失败）
    状态.看板驱动台.启动执行一轮();
    Ok(id)
}

/// 发布看板任务：标题=需求截断（50 字），描述=完整需求，状态=待圣人设计，发起人=道祖。
fn 发布看板任务(状态: &数据服务状态, 文案: &str) -> std::result::Result<u64, hm_error::Error> {
    let 标题 = 截断(文案, 标题上限);
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    let mut task = tc_task::Task::新建(0, 标题, 文案.to_string(), hm_contract::当前时间戳());
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

/// 道祖确认发布：取出对齐需求 → 发布看板（含场景/优先级）→ 写记忆 → 自动驱动一轮。
pub fn 确认发布对齐需求(状态: &数据服务状态) -> std::result::Result<u64, 受理失败> {
    let 需求 = 取出待确认需求(状态)?;
    if !状态.看板驱动台.预留() {
        return Err(受理失败::运行中);
    }
    let id = match 发布对齐任务(状态, &需求) {
        Ok(id) => id,
        Err(e) => {
            状态.看板驱动台.释放();
            tracing::warn!("发布对齐需求失败: {e}");
            return Err(受理失败::创建受阻(e.to_string()));
        }
    };
    if let Err(e) = 写需求记忆(状态, &需求) {
        tracing::warn!("需求记忆写入失败: {e}");
    }
    状态.看板驱动台.启动执行一轮();
    Ok(id)
}

/// 取出道祖接待器的待确认需求（确认发布并清空会话）
fn 取出待确认需求(状态: &数据服务状态) -> std::result::Result<需求摘要, 受理失败> {
    let 接待 = 状态.道祖接待.as_ref().ok_or(受理失败::未上线)?;
    let 接待 = 接待.lock().expect("道祖接待锁中毒");
    接待.确认发布().ok_or(受理失败::无待确认)
}

/// 发布对齐任务：标题=需求标题，描述=需求描述，场景/优先级从摘要解析
fn 发布对齐任务(状态: &数据服务状态, 需求: &需求摘要) -> std::result::Result<u64, hm_error::Error> {
    let 标题 = 截断(&需求.标题, 标题上限);
    let mut board = 状态.任务看板.lock().expect("看板锁中毒");
    let mut task = tc_task::Task::新建(0, 标题, 需求.描述.clone(), hm_contract::当前时间戳());
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
fn 写需求记忆(状态: &数据服务状态, 需求: &需求摘要) -> std::result::Result<u64, hm_error::Error> {
    let 内容 = format!("【需求】{}：{}", 需求.标题, 需求.描述);
    let mut 记忆库 = 状态.记忆库.lock().expect("记忆库锁中毒");
    记忆库.写入(内容, "道祖对齐".into())
}
