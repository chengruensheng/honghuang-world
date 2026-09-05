use std::sync::Arc;
use hm_error::Result;
use crate::数据服务状态;


/// 受理失败原因（HTTP 层映射状态码，启动入口映射日志）
#[derive(Debug)]
pub enum 受理失败 {
    /// 智能体未装配上线（如 LLM key 缺失）
    未上线,
    /// 已有任务执行中
    运行中,
    /// 任务文本为空
    任务为空,
    /// 任务创建受阻（如待受理容量超限）
    创建受阻(String),
}

/// 受理任务标题前缀
const 任务标题前缀: &str = "对话任务: ";
/// 任务标题截断上限（字符数）
const 标题上限: usize = 50;
/// 任务描述固定文案
const 任务描述: &str = "由世界入口对话视图下达，经智能体执行";

/// 受理编排：校验 → 预占执行通道 → 创建任务并推进进行中 → 启动后台执行。
///
/// 返回任务 id；失败时已预占的通道回滚释放。结束通知只做任务状态推进，
/// 最近结果由执行台自身在执行结束时写入。
pub fn 受理开发任务(状态: &数据服务状态, 任务: String) -> std::result::Result<u64, 受理失败> {
    let 文案 = 任务.trim().to_string();
    if 文案.is_empty() {
        return Err(受理失败::任务为空);
    }
    if !状态.开发执行台.就绪() {
        return Err(受理失败::未上线);
    }
    if !状态.开发执行台.预留() {
        return Err(受理失败::运行中);
    }

    let 标题 = format!("{任务标题前缀}{}", 截断(&文案, 标题上限));
    let id = match 状态.任务仓库.lock().expect("引擎锁中毒").创建(标题, 任务描述.to_string()) {
        Ok(id) => id,
        Err(e) => {
            状态.开发执行台.释放();
            tracing::warn!("受理建任务失败: {e}");
            return Err(受理失败::创建受阻(e.to_string()));
        }
    };
    if let Err(e) = 状态.任务仓库.lock().expect("引擎锁中毒").推进(id, tc_task::TaskStatus::进行中) {
        tracing::warn!("任务 {id} 推进为进行中失败: {e}");
    }

    let 结束通知 = {
        let 仓库 = 状态.任务仓库.clone();
        Arc::new(move |结果: &Result<String>| {
            推进结束状态(&仓库, id, 结果);
        })
    };
    状态.开发执行台.启动执行(文案, 结束通知);
    Ok(id)
}

/// 执行结束的任务状态推进：成功→已完成，失败/中断→已取消
fn 推进结束状态(
    仓库: &Arc<std::sync::Mutex<dyn hm_domain_contract::任务仓库契约<tc_task::Task, tc_task::TaskStatus>>>,
    id: u64,
    结果: &Result<String>,
) {
    let 目标 = match 结果 {
        Ok(_) => tc_task::TaskStatus::已完成,
        Err(_) => tc_task::TaskStatus::已取消,
    };
    if let Err(e) = 仓库.lock().expect("引擎锁中毒").推进(id, 目标) {
        tracing::warn!("任务 {id} 结束状态推进失败: {e}");
    }
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
