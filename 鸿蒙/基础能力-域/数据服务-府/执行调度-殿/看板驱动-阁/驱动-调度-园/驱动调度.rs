use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::Serialize;
use hm_agent::{五层协作驱动器, 驱动结果};
use tc_task::TaskStatus;

/// 一次驱动的结果摘要（状态接口可查询）
#[derive(Debug, Clone, Serialize)]
pub struct 驱动阶段摘要 {
    /// 空闲 / 阶段完成 / 错误
    pub 类型: String,
    /// 被推进的任务 id（空闲/错误为 None）
    pub 任务id: Option<u64>,
    /// 承接角色显示名（道祖/圣人/大罗金仙/准圣）
    pub 角色: Option<String>,
    /// 提交后的新状态显示名
    pub 新状态: Option<String>,
    /// 补充消息（错误详情等）
    pub 消息: Option<String>,
}

/// 看板驱动台状态汇总（状态接口返回体）
#[derive(Debug, Serialize)]
pub struct 看板驱动状态 {
    /// 五层协作驱动器是否已装配
    pub 就绪: bool,
    /// 是否有一轮驱动执行中
    pub 运行中: bool,
    /// 最近一次驱动结果摘要
    pub 最近阶段: Option<驱动阶段摘要>,
    /// 最近一次驱动的结果说明
    pub 最近结果: Option<String>,
}

/// 驱动模式：一轮（发布/受理联动）或 到空闲（循环到无可驱动任务/上限/错误）
enum 驱动模式 {
    一轮,
    到空闲(usize),
}

/// 看板驱动台：单轮看板驱动的调度台。
///
/// 参照开发执行台模式：装配后对外就绪；预留互斥防并发；后台线程执行；
/// catch_unwind 兜底保证运行中标志复位；最近阶段/最近结果供状态接口查询。
pub struct 看板驱动台 {
    驱动器: Mutex<Option<Arc<五层协作驱动器>>>,
    运行中: AtomicBool,
    最近阶段: Mutex<Option<驱动阶段摘要>>,
    最近结果: Mutex<Option<String>>,
}

impl 看板驱动台 {
    pub fn 新() -> Self {
        看板驱动台 {
            驱动器: Mutex::new(None),
            运行中: AtomicBool::new(false),
            最近阶段: Mutex::new(None),
            最近结果: Mutex::new(None),
        }
    }

    /// 装配五层协作驱动器；装配后驱动接口就绪
    pub fn 装配(&self, 驱动器: Arc<五层协作驱动器>) {
        *self.驱动器.lock().expect("看板驱动台装配锁中毒") = Some(驱动器);
    }

    /// 驱动器是否已装配
    pub fn 就绪(&self) -> bool {
        self.驱动器.lock().expect("就绪检查锁中毒").is_some()
    }

    /// 预占驱动通道；false 表示已有驱动运行中
    pub fn 预留(&self) -> bool {
        if self
            .运行中
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return false;
        }
        *self.最近阶段.lock().expect("最近阶段清空锁中毒") = None;
        *self.最近结果.lock().expect("最近结果清空锁中毒") = None;
        true
    }

    /// 释放预占（未启动执行时回滚）
    pub fn 释放(&self) {
        self.运行中.store(false, Ordering::SeqCst);
    }

    /// 发布后自动驱动（尽力而为）：就绪且空闲才启动；未就绪/运行中静默返回 false。
    /// 供 看板发布 handler 与 受理编排 联动调用，不阻塞发布结果。
    pub fn 自动驱动一轮(self: &Arc<Self>) -> bool {
        if !self.就绪() {
            return false;
        }
        if !self.预留() {
            return false;
        }
        self.启动执行一轮();
        true
    }

    /// 启动后台驱动一轮；结束（含 panic）后复位运行中并写入摘要。
    /// 开头强制置位运行中：即使调用方未先预留（如测试直调），等待完成/并发互斥依然成立。
    pub fn 启动执行一轮(self: &Arc<Self>) {
        self.启动驱动(驱动模式::一轮);
    }

    /// 启动后台驱动到空闲：循环执行一轮直到 无可驱动任务 / 达到上限 / 出错，
    /// 已推进的轮次结果保留（每轮推进更新最近摘要）。结束（含 panic）后复位运行中。
    pub fn 启动执行到空闲(self: &Arc<Self>, 上限: usize) {
        self.启动驱动(驱动模式::到空闲(上限));
    }

    /// 统一驱动启动：一轮 = 上限 1；到空闲 = 上限 N。失败保留已推进轮次计数。
    fn 启动驱动(self: &Arc<Self>, 模式: 驱动模式) {
        self.运行中.store(true, Ordering::SeqCst);
        let 驱动器 = {
            let 守卫 = self.驱动器.lock().expect("看板驱动台锁中毒");
            match 守卫.as_ref() {
                Some(器) => 器.clone(),
                None => {
                    self.释放();
                    *self.最近结果.lock().expect("最近结果锁中毒") = Some("驱动器未装配".into());
                    return;
                }
            }
        };
        let 驱动台 = self.clone();
        std::thread::spawn(move || {
            let 上限 = match 模式 {
                驱动模式::一轮 => 1,
                驱动模式::到空闲(上限) => 上限,
            };
            let mut 轮次数: usize = 0;
            let mut 失败: Option<hm_error::Error> = None;
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                for _ in 0..上限 {
                    match 驱动器.执行一轮() {
                        Ok(驱动结果::空闲) => break,
                        Ok(驱动结果::阶段完成 { 任务id, 角色, 新状态 }) => {
                            轮次数 += 1;
                            驱动台.写最近阶段(驱动阶段摘要 {
                                类型: "阶段完成".into(),
                                任务id: Some(任务id),
                                角色: Some(角色.名称().to_string()),
                                新状态: Some(状态名(&新状态).to_string()),
                                消息: None,
                            });
                        }
                        Err(e) => {
                            失败 = Some(e);
                            break;
                        }
                    }
                }
            }))
            .unwrap_or_else(|_| {
                失败 = Some(hm_error::Error::Other("看板驱动线程异常终止".into()));
            });
            // 先写摘要、后复位运行中：等待完成() 依赖 运行中 复位信号，必须保证摘要先行可见
            match 失败 {
                Some(e) => {
                    *驱动台.最近阶段.lock().expect("最近阶段锁中毒") = Some(驱动阶段摘要 {
                        类型: "错误".into(),
                        任务id: None,
                        角色: None,
                        新状态: None,
                        消息: Some(e.to_string()),
                    });
                    *驱动台.最近结果.lock().expect("最近结果锁中毒") = Some(if 轮次数 > 0 {
                        format!("驱动失败: {e}（已推进 {轮次数} 轮）")
                    } else {
                        format!("驱动失败: {e}")
                    });
                }
                None if 轮次数 == 0 => {
                    *驱动台.最近阶段.lock().expect("最近阶段锁中毒") = Some(驱动阶段摘要 {
                        类型: "空闲".into(),
                        任务id: None,
                        角色: None,
                        新状态: None,
                        消息: None,
                    });
                    *驱动台.最近结果.lock().expect("最近结果锁中毒") =
                        Some("看板无可驱动任务（空闲）".into());
                }
                None => {
                    // 单轮保持 v1.36 文案兼容；多轮给出汇总
                    let 摘要 = 驱动台.最近阶段.lock().expect("最近阶段锁中毒").clone();
                    *驱动台.最近结果.lock().expect("最近结果锁中毒") = Some(match 摘要 {
                        Some(驱动阶段摘要 { 任务id: Some(id), 角色: Some(角色), .. }) if 轮次数 == 1 => {
                            format!("任务 {id} 已推进（{角色}）")
                        }
                        _ => format!("共推进 {轮次数} 轮"),
                    });
                }
            }
            驱动台.运行中.store(false, Ordering::SeqCst);
        });
    }

    /// 写入最近阶段摘要（驱动线程内使用）
    fn 写最近阶段(&self, 摘要: 驱动阶段摘要) {
        *self.最近阶段.lock().expect("最近阶段锁中毒") = Some(摘要);
    }

    /// 是否有驱动运行中
    pub fn 运行中(&self) -> bool {
        self.运行中.load(Ordering::SeqCst)
    }

    /// 状态汇总（状态接口）
    pub fn 当前状态(&self) -> 看板驱动状态 {
        看板驱动状态 {
            就绪: self.就绪(),
            运行中: self.运行中(),
            最近阶段: self.最近阶段.lock().expect("最近阶段锁中毒").clone(),
            最近结果: self.最近结果.lock().expect("最近结果锁中毒").clone(),
        }
    }

    /// 轮询等待驱动完成（测试用）
    pub fn 等待完成(&self, 超时毫秒: u64) -> bool {
        let 开始 = Instant::now();
        while self.运行中() {
            if 开始.elapsed() > Duration::from_millis(超时毫秒) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        true
    }
}

/// TaskStatus 显示名（驱动摘要用）
fn 状态名(状态: &TaskStatus) -> &'static str {
    match 状态 {
        TaskStatus::待受理 => "待受理",
        TaskStatus::进行中 => "进行中",
        TaskStatus::已完成 => "已完成",
        TaskStatus::已取消 => "已取消",
        TaskStatus::待圣人设计 => "待圣人设计",
        TaskStatus::圣人设计中 => "圣人设计中",
        TaskStatus::待大罗金仙实现 => "待大罗金仙实现",
        TaskStatus::大罗金仙实现中 => "大罗金仙实现中",
        TaskStatus::待准圣验收 => "待准圣验收",
        TaskStatus::准圣验收中 => "准圣验收中",
        TaskStatus::待修复 => "待修复",
        TaskStatus::待道祖终审 => "待道祖终审",
        TaskStatus::道祖终审中 => "道祖终审中",
    }
}
