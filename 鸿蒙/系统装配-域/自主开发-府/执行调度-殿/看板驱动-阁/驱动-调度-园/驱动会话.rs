use std::sync::atomic::Ordering;
use std::sync::Arc;
use crate::驱动结果;
use hm_content_contract::对话消息;
use tc_task::{TaskStatus, 状态层级标签};
use crate::{驱动会话详情, 运行检查点, 驱动阶段事件, 驱动阶段摘要, 驱动过程事件, 任务项};
use super::驱动调度::看板驱动台;

/// 驱动事件流环形上限：超限丢弃最旧记录，序号仍单调递增
const 驱动事件流上限: usize = 500;
/// 过程事件流环形上限：智能体循环每步事件，超限丢弃最旧
const 过程事件流上限: usize = 1000;

impl 看板驱动台 {
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
pub(crate) fn 状态名(状态: &TaskStatus) -> &'static str {
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
        TaskStatus::待人工验收 => "待人工验收",
        TaskStatus::人工验收中 => "人工验收中",
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
