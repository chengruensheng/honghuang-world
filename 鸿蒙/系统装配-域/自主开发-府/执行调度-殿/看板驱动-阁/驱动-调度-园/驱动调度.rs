use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::{五层协作驱动器, 驱动结果};
use tc_task::状态层级标签;
use crate::{驱动会话存储, 驱动阶段事件, 驱动阶段摘要, 驱动过程事件, 看板驱动状态};
use super::驱动会话::状态名;

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
    pub(crate) 驱动器: Mutex<Option<Arc<五层协作驱动器>>>,
    pub(crate) 运行中: AtomicBool,
    最近阶段: Mutex<Option<驱动阶段摘要>>,
    pub(crate) 最近结果: Mutex<Option<String>>,
    pub(crate) 事件流: Mutex<Vec<驱动阶段事件>>,
    pub(crate) 事件序号: AtomicU64,
    pub(crate) 过程事件流: Mutex<Vec<驱动过程事件>>,
    pub(crate) 过程事件序号: AtomicU64,
    pub(crate) 会话存储: 驱动会话存储,
    pub(crate) 当前会话id: Mutex<Option<u64>>,
}

impl 看板驱动台 {
    pub fn 新() -> Self {
        看板驱动台 {
            驱动器: Mutex::new(None),
            运行中: AtomicBool::new(false),
            最近阶段: Mutex::new(None),
            最近结果: Mutex::new(None),
            事件流: Mutex::new(Vec::new()),
            事件序号: AtomicU64::new(0),
            过程事件流: Mutex::new(Vec::new()),
            过程事件序号: AtomicU64::new(0),
            会话存储: 驱动会话存储::新(None),
            当前会话id: Mutex::new(None),
        }
    }

    /// 装配五层协作驱动器；装配后驱动接口就绪
    pub fn 装配(&self, 驱动器: Arc<五层协作驱动器>) {
        *self.驱动器.lock().expect("看板驱动台装配锁中毒") = Some(驱动器);
    }

    /// 装配会话存储目录：None=纯内存（重启丢失），Some=落盘持久化（重启可回看）
    pub fn 装配会话存储(&self, 目录: &str) {
        let 目录 = if 目录.trim().is_empty() {
            None
        } else {
            Some(目录.to_string())
        };
        self.会话存储.设置目录(目录);
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
        self.事件流.lock().expect("事件流清空锁中毒").clear();
        self.过程事件流.lock().expect("过程事件流清空锁中毒").clear();
        // 序号不归零：序号是客户端的增量游标（since），必须在本进程内全局单调。
        // 清空缓冲只为「新连接只看当前轮」；若连序号一起归零，已经连着的老客户端
        // （游标停在上轮末号）会把新轮的 1..N 全部判成「已读过」而永远收不到新事件——
        // 表现为看板已推进、过程流却一片空白。缓冲可以丢，游标刻度不能倒拨。
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
        self.启动执行一轮("发布自动");
        true
    }

    /// 启动后台驱动一轮；结束（含 panic）后复位运行中并写入摘要。
    /// 开头强制置位运行中：即使调用方未先预留（如测试直调），等待完成/并发互斥依然成立。
    pub fn 启动执行一轮(self: &Arc<Self>, 发起方式: &str) {
        self.启动驱动(驱动模式::一轮, 发起方式);
    }

    /// 启动后台驱动到空闲：循环执行一轮直到 无可驱动任务 / 达到上限 / 出错，
    /// 已推进的轮次结果保留（每轮推进更新最近摘要）。结束（含 panic）后复位运行中。
    pub fn 启动执行到空闲(self: &Arc<Self>, 上限: usize, 发起方式: &str) {
        self.启动驱动(驱动模式::到空闲(上限), 发起方式);
    }

    /// 统一驱动启动：一轮 = 上限 1；到空闲 = 上限 N。失败保留已推进轮次计数。
    fn 启动驱动(self: &Arc<Self>, 模式: 驱动模式, 发起方式: &str) {
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
        // 创建会话：本次「发布→驱动」固化为可回放的会话（发起方式：发布自动/手动驱动/手动到空闲）
        let 会话id = self.会话存储.创建会话(发起方式);
        *self.当前会话id.lock().expect("当前会话id锁中毒") = Some(会话id);
        let 驱动台 = self.clone();
        std::thread::spawn(move || {
            let 上限 = match 模式 {
                驱动模式::一轮 => 1,
                驱动模式::到空闲(上限) => 上限,
            };
            let mut 轮次数: usize = 0;
            let mut 失败: Option<hm_error::Error> = None;
            let mut 空闲: bool = false;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                for _ in 0..上限 {
                    match 驱动器.执行一轮() {
                        Ok(驱动结果::空闲) => {
                            空闲 = true;
                            break;
                        }
                        Ok(驱动结果::阶段完成 { 任务id, 角色, 新状态 }) => {
                            轮次数 += 1;
                            let 角色名 = 角色.名称().to_string();
                            let 新状态名 = 状态名(&新状态).to_string();
                            let 层级 = 状态层级标签(新状态).名().to_string();
                            驱动台.写最近阶段(驱动阶段摘要 {
                                类型: "阶段完成".into(),
                                任务id: Some(任务id),
                                角色: Some(角色名.clone()),
                                新状态: Some(新状态名.clone()),
                                层级: Some(层级.clone()),
                                消息: None,
                            });
                            驱动台.记录驱动事件(&驱动阶段事件 {
                                序号: 0,
                                类型: "阶段完成".into(),
                                任务id: Some(任务id),
                                角色: Some(角色名),
                                新状态: Some(新状态名),
                                层级: Some(层级),
                                消息: None,
                                时间: hm_contract::当前时间戳(),
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
                        层级: None,
                        消息: Some(e.to_string()),
                    });
                    *驱动台.最近结果.lock().expect("最近结果锁中毒") = Some(if 轮次数 > 0 {
                        format!("驱动失败: {e}（已推进 {轮次数} 轮）")
                    } else {
                        format!("驱动失败: {e}")
                    });
                    驱动台.记录驱动事件(&驱动阶段事件 {
                        序号: 0,
                        类型: "错误".into(),
                        任务id: None,
                        角色: None,
                        新状态: None,
                        层级: None,
                        消息: Some(e.to_string()),
                        时间: hm_contract::当前时间戳(),
                    });
                }
                None if 轮次数 == 0 => {
                    *驱动台.最近阶段.lock().expect("最近阶段锁中毒") = Some(驱动阶段摘要 {
                        类型: "空闲".into(),
                        任务id: None,
                        角色: None,
                        新状态: None,
                        层级: None,
                        消息: None,
                    });
                    *驱动台.最近结果.lock().expect("最近结果锁中毒") =
                        Some("看板无可驱动任务（空闲）".into());
                    驱动台.记录驱动事件(&驱动阶段事件 {
                        序号: 0,
                        类型: "空闲".into(),
                        任务id: None,
                        角色: None,
                        新状态: None,
                        层级: None,
                        消息: None,
                        时间: hm_contract::当前时间戳(),
                    });
                }
                None if 空闲 => {
                    // 多轮后看板清空：空闲作为会话结束事件记录，最近阶段保留最后推进、最近结果写汇总
                    驱动台.记录驱动事件(&驱动阶段事件 {
                        序号: 0,
                        类型: "空闲".into(),
                        任务id: None,
                        角色: None,
                        新状态: None,
                        层级: None,
                        消息: None,
                        时间: hm_contract::当前时间戳(),
                    });
                    *驱动台.最近结果.lock().expect("最近结果锁中毒") =
                        Some(format!("共推进 {轮次数} 轮"));
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
            // 会话收尾：以最近阶段/最近结果 决定状态（错误=失败 / 空闲=空闲 / 其余=已完成）与摘要
            if let Some(会话id) = 驱动台.当前会话id.lock().expect("当前会话id锁中毒").clone() {
                let 阶段 = 驱动台.最近阶段.lock().expect("最近阶段锁中毒").clone();
                let 结果 = 驱动台.最近结果.lock().expect("最近结果锁中毒").clone();
                let 状态 = match &阶段 {
                    Some(驱动阶段摘要 { 类型, .. }) if 类型 == "错误" => "失败",
                    Some(驱动阶段摘要 { 类型, .. }) if 类型 == "空闲" => "空闲",
                    _ => "已完成",
                };
                驱动台.会话存储.结束会话(会话id, 状态, 结果);
            }
        });
    }

    /// 写入最近阶段摘要（驱动线程内使用）
    pub(crate) fn 写最近阶段(&self, 摘要: 驱动阶段摘要) {
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

    /// 当前运行会话 id（无运行中会话时为 None；供协议适配器生成 runId/threadId）
    pub fn 当前会话id(&self) -> Option<u64> {
        *self.当前会话id.lock().expect("当前会话id锁中毒")
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
