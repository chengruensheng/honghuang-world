use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use hm_agent::{五层协作驱动器, 驱动结果, 任务项};
use hm_content_contract::对话消息;
use tc_task::{TaskStatus, 状态层级标签};
use crate::{驱动会话存储, 驱动会话详情, 运行检查点, 驱动阶段事件, 驱动阶段摘要, 驱动过程事件, 看板驱动状态};
/// 驱动事件流环形上限：超限丢弃最旧记录，序号仍单调递增
const 驱动事件流上限: usize = 500;
/// 过程事件流环形上限：智能体循环每步事件，超限丢弃最旧
const 过程事件流上限: usize = 1000;

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
    事件流: Mutex<Vec<驱动阶段事件>>,
    事件序号: AtomicU64,
    过程事件流: Mutex<Vec<驱动过程事件>>,
    过程事件序号: AtomicU64,
    会话存储: 驱动会话存储,
    当前会话id: Mutex<Option<u64>>,
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

    /// 记录一条驱动阶段事件（环形上限，序号单调递增）
    pub fn 记录驱动事件(&self, 事件: &驱动阶段事件) {
        let 序号 = self.事件序号.fetch_add(1, Ordering::SeqCst) + 1;
        let 记录 = 驱动阶段事件 { 序号, ..事件.clone() };
        let mut 流 = self.事件流.lock().expect("事件流锁中毒");
        if 流.len() >= 驱动事件流上限 {
            流.remove(0);
        }
        流.push(记录);
    }

    /// 返回序号大于 since 的驱动事件增量
    pub fn 驱动事件增量(&self, since: u64) -> Vec<驱动阶段事件> {
        let 流 = self.事件流.lock().expect("驱动事件增量锁中毒");
        流.iter().filter(|记录| 记录.序号 > since).cloned().collect()
    }

    /// 记录一条驱动过程事件（环形上限，序号单调递增；并转录到当前会话落盘供历史回放）
    pub fn 记录过程事件(&self, 事件: &驱动过程事件) {
        let 序号 = self.过程事件序号.fetch_add(1, Ordering::SeqCst) + 1;
        let 记录 = 驱动过程事件 { 序号, ..事件.clone() };
        {
            let mut 流 = self.过程事件流.lock().expect("过程事件流锁中毒");
            if 流.len() >= 过程事件流上限 {
                流.remove(0);
            }
            流.push(记录.clone());
        }
        // 会话双写：把事件转录到当前会话（落盘），供 /api/dev/sessions/{id} 回放
        if let Some(会话id) = self.当前会话id.lock().expect("当前会话id锁中毒").clone() {
            self.会话存储.追加事件(会话id, &记录);
        }
    }

    /// 返回序号大于 since 的过程事件增量
    pub fn 过程事件增量(&self, since: u64) -> Vec<驱动过程事件> {
        let 流 = self.过程事件流.lock().expect("过程事件增量锁中毒");
        流.iter().filter(|记录| 记录.序号 > since).cloned().collect()
    }

    /// 会话清单（历史回放入口，按创建时间倒序）
    pub fn 会话清单(&self) -> Vec<crate::驱动会话摘要> {
        self.会话存储.清单()
    }

    /// 会话回放：返回某会话元数据 + 全程事件；会话不存在时返回 None
    pub fn 会话回放(&self, 会话id: u64) -> Option<驱动会话详情> {
        self.会话存储.回放(会话id)
    }

    /// 写入当前会话下某任务某阶段的运行检查点（结合当前会话id；供 Resume/Fork 断点续跑）
    pub fn 写入检查点(&self, 任务id: u64, 角色名: &str, 轮次: usize, 阶段提示: &str, 消息: Vec<对话消息>, 清单: Vec<任务项>) {
        if let Some(会话id) = self.当前会话id.lock().expect("当前会话id锁中毒").clone() {
            let 检查点 = 运行检查点 {
                会话id,
                任务id,
                角色: 角色名.to_string(),
                阶段提示: 阶段提示.to_string(),
                消息,
                任务清单: 清单,
                轮次,
                时间: hm_contract::当前时间戳(),
            };
            self.会话存储.保存检查点(&检查点);
        }
    }

    /// 读取某会话某任务的运行检查点（供 Resume/Fork 恢复）
    pub fn 读取检查点(&self, 会话id: u64, 任务id: u64) -> Option<运行检查点> {
        self.会话存储.读取检查点(会话id, 任务id)
    }

    /// 定向恢复/分叉驱动：基于源会话检查点消息推进指定任务（`继承消息`=true 时携带源断点上下文）。
    /// 返回 Ok(新会话id) 受理成功；Err(原因) 未受理（未就绪/互斥）。
    pub fn 启动定向驱动(self: &Arc<Self>, 源会话id: u64, 任务id: u64, 继承消息: bool, 发起方式: &str) -> Result<u64, String> {
        if !self.就绪() {
            return Err("看板驱动未就绪：需配置 LLM_API_KEY 并开启 run_dev_agent 后重启".into());
        }
        // Resume 前置：读检查点做两件事——404（无断点）与闸门（该阶段已推进则 400 无需恢复）
        let 检查点 = self.会话存储.读取检查点(源会话id, 任务id);
        if 发起方式 == "恢复" {
            let 断点 = 检查点
                .as_ref()
                .ok_or_else(|| "无可恢复检查点（该会话未落盘该任务的运行断点）".to_string())?;
            if let Some(器) = self.驱动器.lock().expect("驱动器锁中毒").as_ref() {
                器.恢复闸门(任务id, &断点.角色)?;
            }
        }
        if !self.预留() {
            return Err("已有驱动执行中，请等待完成后再驱动".into());
        }
        let 已存消息 = if 继承消息 {
            检查点.map(|c| c.消息).unwrap_or_default()
        } else {
            Vec::new()
        };
        let 发起方式标记 = format!("{发起方式}:{源会话id}");
        let 会话id = self.会话存储.创建会话(&发起方式标记);
        *self.当前会话id.lock().expect("当前会话id锁中毒") = Some(会话id);
        let 驱动台 = self.clone();
        std::thread::spawn(move || {
            let 驱动器 = match 驱动台.驱动器.lock().expect("驱动器锁中毒").as_ref() {
                Some(器) => 器.clone(),
                None => {
                    *驱动台.最近结果.lock().expect("最近结果锁中毒") = Some("驱动器未装配".into());
                    驱动台.运行中.store(false, Ordering::SeqCst);
                    驱动台.会话存储.结束会话(会话id, "失败", Some("驱动器未装配".into()));
                    return;
                }
            };
            let 结果 = 驱动器.执行一轮_恢复(Some((任务id, 已存消息)));
            驱动台.收尾定向驱动(会话id, 结果);
        });
        Ok(会话id)
    }

    /// 定向驱动结果收尾（失败/空闲/阶段完成 → 摘要+事件+会话状态）；复用与 启动驱动 相同口径
    fn 收尾定向驱动(&self, 会话id: u64, 结果: hm_error::Result<驱动结果>) {
        let (状态, 结果说明): (&str, String) = match &结果 {
            Ok(驱动结果::阶段完成 { 任务id, 角色, 新状态 }) => {
                let 角色名 = 角色.名称().to_string();
                let 新状态名 = 状态名(新状态).to_string();
                let 层级 = 状态层级标签(*新状态).名().to_string();
                self.写最近阶段(驱动阶段摘要 {
                    类型: "阶段完成".into(),
                    任务id: Some(*任务id),
                    角色: Some(角色名.clone()),
                    新状态: Some(新状态名.clone()),
                    层级: Some(层级.clone()),
                    消息: None,
                });
                self.记录驱动事件(&驱动阶段事件 {
                    序号: 0,
                    类型: "阶段完成".into(),
                    任务id: Some(*任务id),
                    角色: Some(角色名.clone()),
                    新状态: Some(新状态名),
                    层级: Some(层级),
                    消息: None,
                    时间: hm_contract::当前时间戳(),
                });
                ("已完成", format!("任务 {任务id} 已推进（{角色名}）"))
            }
            Ok(驱动结果::空闲) => {
                self.记录驱动事件(&驱动阶段事件 {
                    序号: 0,
                    类型: "空闲".into(),
                    任务id: None,
                    角色: None,
                    新状态: None,
                    层级: None,
                    消息: None,
                    时间: hm_contract::当前时间戳(),
                });
                ("空闲", "看板无可驱动任务（空闲）".into())
            }
            Err(e) => {
                self.写最近阶段(驱动阶段摘要 {
                    类型: "错误".into(),
                    任务id: None,
                    角色: None,
                    新状态: None,
                    层级: None,
                    消息: Some(e.to_string()),
                });
                self.记录驱动事件(&驱动阶段事件 {
                    序号: 0,
                    类型: "错误".into(),
                    任务id: None,
                    角色: None,
                    新状态: None,
                    层级: None,
                    消息: Some(e.to_string()),
                    时间: hm_contract::当前时间戳(),
                });
                ("失败", format!("驱动失败: {e}"))
            }
        };
        *self.最近结果.lock().expect("最近结果锁中毒") = Some(结果说明.clone());
        self.运行中.store(false, Ordering::SeqCst);
        self.会话存储.结束会话(会话id, 状态, Some(结果说明));
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
        TaskStatus::待清理 => "待清理",
        TaskStatus::清理中 => "清理中",
        TaskStatus::清理完成 => "清理完成",
        TaskStatus::待道祖澄清 => "待道祖澄清",
        TaskStatus::道祖澄清中 => "道祖澄清中",
        TaskStatus::待重新设计 => "待重新设计",
        TaskStatus::待重新实现 => "待重新实现",
        TaskStatus::待重新验收 => "待重新验收",
        TaskStatus::待重新清理 => "待重新清理",
    }
}
