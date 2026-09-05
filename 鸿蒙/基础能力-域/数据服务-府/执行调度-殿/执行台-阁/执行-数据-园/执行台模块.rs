use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;
use hm_error::Result;
use hm_execute_contract::{开发事件, 开发事件类型, 开发执行契约};

/// 事件流环形上限：超限丢弃最旧记录，序号仍单调递增
const 事件流上限: usize = 500;

/// 一条对外可查询的执行事件记录
#[derive(Debug, Clone, Serialize)]
pub struct 事件记录 {
    /// 本次执行会话内的单调序号（从 1 起，受理新任务时重置）
    pub 序号: u64,
    /// 智能体循环轮次（从 0 起）
    pub 轮次: usize,
    /// 事件类型名：思考/工具调用/工具结果/任务答复
    pub 类型: String,
    /// 工具名（思考/答复类为空串）
    pub 工具名: String,
    /// 事件内容摘要（智能体侧已截断）
    pub 内容: String,
    /// 记录时间戳（秒）
    pub 时间: u64,
}

/// 执行台状态汇总（事件查询接口的元信息部分）
#[derive(Debug, Serialize)]
pub struct 事件流状态 {
    /// 智能体是否已装配上线
    pub 就绪: bool,
    /// 是否有任务执行中
    pub 运行中: bool,
    /// 智能体工作区（沙箱）
    pub 工作区: String,
    /// 最近一次执行结果（答复文本或失败摘要）
    pub 最近结果: Option<String>,
}

/// 开发执行台：智能体上线受理的核心调度。
///
/// 持有开发执行契约实现（生产为 hm-agent 智能体，测试注入 mock），
/// 管理执行通道互斥、事件环形流与最近结果；同步契约执行放入后台线程。
pub struct 开发执行台 {
    执行器: Mutex<Option<Arc<dyn 开发执行契约>>>,
    工作区: Mutex<String>,
    运行中: AtomicBool,
    事件流: Mutex<Vec<事件记录>>,
    下一序号: AtomicU64,
    最近结果: Mutex<Option<String>>,
}

impl 开发执行台 {
    pub fn 新() -> Self {
        开发执行台 {
            执行器: Mutex::new(None),
            工作区: Mutex::new(String::new()),
            运行中: AtomicBool::new(false),
            事件流: Mutex::new(Vec::new()),
            下一序号: AtomicU64::new(0),
            最近结果: Mutex::new(None),
        }
    }

    /// 装配智能体实现与工作区元信息；装配后受理接口就绪
    pub fn 装配(&self, 执行器: Arc<dyn 开发执行契约>, 工作区: &str) {
        *self.执行器.lock().expect("执行台装配锁中毒") = Some(执行器);
        *self.工作区.lock().expect("工作区锁中毒") = 工作区.to_string();
    }

    /// 智能体是否已装配上线
    pub fn 就绪(&self) -> bool {
        self.执行器.lock().expect("就绪检查锁中毒").is_some()
    }

    /// 预占执行通道：成功时清空上一次会话的事件流与结果；false 表示已有任务执行中
    pub fn 预留(&self) -> bool {
        if self
            .运行中
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return false;
        }
        self.事件流.lock().expect("事件流清空锁中毒").clear();
        self.下一序号.store(0, Ordering::SeqCst);
        *self.最近结果.lock().expect("最近结果清空锁中毒") = None;
        true
    }

    /// 释放预占（创建任务失败等需要回滚时），不产生任何通知
    pub fn 释放(&self) {
        self.运行中.store(false, Ordering::SeqCst);
    }

    /// 启动后台执行线程：结束（含 panic）后释放通道、写入最近结果并通知编排方
    pub fn 启动执行(self: &Arc<Self>, 任务: String, 结束通知: Arc<dyn Fn(&Result<String>) + Send + Sync>) {
        let 执行器 = {
            let 守卫 = self.执行器.lock().expect("执行台锁中毒");
            match 守卫.as_ref() {
                Some(器) => 器.clone(),
                None => {
                    self.释放();
                    (结束通知)(&Err(hm_error::Error::Other("智能体未装配".into())));
                    return;
                }
            }
        };
        let 执行台 = self.clone();
        std::thread::spawn(move || {
            let 结果 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                执行器.执行开发任务(任务)
            }))
            .unwrap_or_else(|_| Err(hm_error::Error::Other("智能体执行线程异常终止".into())));
            执行台.运行中.store(false, Ordering::SeqCst);
            let 摘要 = match &结果 {
                Ok(答复) => Some(答复.clone()),
                Err(错误) => Some(format!("执行失败: {错误}")),
            };
            *执行台.最近结果.lock().expect("最近结果锁中毒") = 摘要;
            (结束通知)(&结果);
        });
    }

    /// 置位中断句柄请求停止循环；未装配时返回 false
    pub fn 中断(&self) -> bool {
        let 守卫 = self.执行器.lock().expect("中断锁中毒");
        match 守卫.as_ref() {
            Some(器) => {
                器.中断句柄().store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    /// 是否有任务执行中
    pub fn 运行中(&self) -> bool {
        self.运行中.load(Ordering::SeqCst)
    }

    /// 记录一条智能体事件（环形上限，序号单调递增）
    pub fn 记录事件(&self, 事件: &开发事件) {
        let 序号 = self.下一序号.fetch_add(1, Ordering::SeqCst) + 1;
        let 记录 = 事件记录 {
            序号,
            轮次: 事件.轮次,
            类型: 事件类型名(&事件.类型).to_string(),
            工具名: 事件.工具名.clone(),
            内容: 事件.内容.clone(),
            时间: 当前秒(),
        };
        let mut 流 = self.事件流.lock().expect("事件流锁中毒");
        if 流.len() >= 事件流上限 {
            流.remove(0);
        }
        流.push(记录);
    }

    /// 返回序号大于 since 的事件增量
    pub fn 事件增量(&self, since: u64) -> Vec<事件记录> {
        let 流 = self.事件流.lock().expect("事件增量锁中毒");
        流.iter().filter(|记录| 记录.序号 > since).cloned().collect()
    }

    /// 汇总状态信息（事件查询接口）
    pub fn 当前状态(&self) -> 事件流状态 {
        事件流状态 {
            就绪: self.就绪(),
            运行中: self.运行中(),
            工作区: self.工作区.lock().expect("状态工作区锁中毒").clone(),
            最近结果: self.最近结果.lock().expect("状态结果锁中毒").clone(),
        }
    }
}

/// 开发事件类型转显示名
fn 事件类型名(类型: &开发事件类型) -> &'static str {
    match 类型 {
        开发事件类型::思考 => "思考",
        开发事件类型::工具调用 => "工具调用",
        开发事件类型::工具结果 => "工具结果",
        开发事件类型::任务答复 => "任务答复",
    }
}

/// 当前 UNIX 时间戳（秒）
fn 当前秒() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
