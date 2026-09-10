//! 长河事件词表 v1 —— 水镜映真-域的唯一显示契约（Rust 真理源）。
//! 规约（设计稿 §四）：
//! 1. wire 信封 = {"类型": ..., ...载荷}，全中文字段，单语言、无双名回退；
//! 2. 界面 = 对长河事件序列的纯归约，归约不产生副作用；
//! 3. 错误是一等事件（河错），严禁哨兵字符串入正文。
//! JS 侧 长河.js 为镜像同构实现，验收标准：同一事件序列产出同一会话视图。

use serde::{Deserialize, Serialize};

/// 接待流固定消息id（旧方言适配层由此归并增量）
pub const 接待消息ID: &str = "接待";

/// 阶段词表（后端语义词的唯一出口；旧接口阶段串由适配层映射到此）
pub mod 阶段 {
    pub const 接待中: &str = "接待中";
    pub const 待确认: &str = "待确认";
}

/// 长河事件：镜面上发生的一切皆为一事件
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "类型")]
pub enum 长河事件 {
    /// 一条流式回复开始
    回复开始 { 消息id: String, 角色: String },
    /// 回复正文增量（逐块）
    回复增量 { 消息id: String, 文: String },
    /// 回复收尾
    回复结束 { 消息id: String },
    /// 思考流开始（折叠呈现）
    思考开始 { 消息id: String },
    /// 思考正文增量
    思考增量 { 消息id: String, 文: String },
    /// 思考收尾
    思考结束 { 消息id: String },
    /// 工具调用开始
    工具开始 { 调用id: String, 名称: String },
    /// 工具参数增量
    工具参数 { 调用id: String, 片段: String },
    /// 工具调用收尾
    工具结束 { 调用id: String },
    /// 工具结果（终值）
    工具结果 { 调用id: String, 文: String },
    /// 一轮运行收尾（接待/驱动共用）
    运行结束 { 阶段: String, 任务id: Option<u64> },
    /// 任务卡状态迁移（状态词 = 后端唯一词表，前端禁做字符串猜谜）
    任务状态 { 任务id: u64, 状态: String },
    /// 任务执行过程事件（序号单调，归约按游标去重；字段名"类别"避让 serde tag"类型"）
    过程事件 {
        任务id: u64,
        序号: u64,
        类别: String,
        角色: String,
        工具名: Option<String>,
        内容: String,
    },
    /// 类型化错误（替代哨兵字符串）
    河错 { 来源: String, 文: String },
    /// 清河：整河重置（用户清屏/重挂回放）
    清河 {},
}

/// 消息方向
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum 方向 {
    己方,
    对方,
}

/// 消息内容段：一条消息由若干段构成（正文/思考/工具）
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "类")]
pub enum 段视图 {
    正文 { 文: String },
    思考 { 文: String },
    工具 { 名称: String, 参数: String, 结果: String },
}

/// 消息视图
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct 消息视图 {
    pub 消息id: String,
    pub 方向: 方向,
    pub 角色: String,
    pub 段: Vec<段视图>,
    /// 是否仍在接收增量（回复/思考未收尾）
    pub 在流: bool,
}

/// 执行过程条目
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct 过程条 {
    pub 序号: u64,
    pub 类别: String,
    pub 角色: String,
    pub 工具名: Option<String>,
    pub 内容: String,
}

/// 任务视图
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct 任务视图 {
    pub 状态: String,
    pub 过程: Vec<过程条>,
}

/// 会话视图：对长河事件归约出的唯一真相（回放/快照/统一通道复用此结构）
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct 会话视图 {
    pub 阶段: String,
    pub 消息: Vec<消息视图>,
    pub 任务: std::collections::BTreeMap<u64, 任务视图>,
    pub 过程游标: u64,
}

/// 归约：视图 × 事件 → 新视图（纯函数；克隆进、克隆出，无副作用）
pub fn 归约(前: &会话视图, 事: &长河事件) -> 会话视图 {
    let mut 后 = 前.clone();
    match 事 {
        长河事件::回复开始 { 消息id, 角色 } => {
            let 消息 = 开消息(&mut 后, 消息id, 方向::对方, 角色);
            消息.在流 = true;
        }
        长河事件::回复增量 { 消息id, 文 } => {
            追段文(&mut 后, 消息id, &段择::正文, 文);
        }
        长河事件::回复结束 { 消息id } => {
            if let Some(消息) = 找消息(&mut 后, 消息id) {
                消息.在流 = false;
            }
        }
        长河事件::思考开始 { 消息id } => {
            let 消息 = 开消息(&mut 后, 消息id, 方向::对方, "道祖");
            消息.在流 = true;
            if !消息.段.iter().any(|s| matches!(s, 段视图::思考 { .. })) {
                消息.段.push(段视图::思考 { 文: String::new() });
            }
        }
        长河事件::思考增量 { 消息id, 文 } => {
            追段文(&mut 后, 消息id, &段择::思考, 文);
        }
        长河事件::思考结束 { 消息id } => {
            if let Some(消息) = 找消息(&mut 后, 消息id) {
                消息.在流 = false;
            }
        }
        长河事件::工具开始 { 调用id, 名称 } => {
            // 调用id 即消息id，段与调用一一对应：每次开始恒新开工具段
            let 消息 = 开消息(&mut 后, 调用id, 方向::对方, "道祖");
            消息.在流 = true;
            消息.段.push(段视图::工具 {
                名称: 名称.clone(),
                参数: String::new(),
                结果: String::new(),
            });
        }
        长河事件::工具参数 { 调用id, 片段 } => {
            if let Some(段视图::工具 { 参数, .. }) = 找工具段(&mut 后, 调用id) {
                参数.push_str(片段);
            }
        }
        长河事件::工具结束 { .. } => {}
        长河事件::工具结果 { 调用id, 文 } => {
            if let Some(段视图::工具 { 结果, .. }) = 找工具段(&mut 后, 调用id) {
                *结果 = 文.clone();
            }
        }
        长河事件::运行结束 { 阶段, 任务id } => {
            后.阶段 = 阶段.clone();
            if let Some(id) = 任务id {
                后.任务.entry(*id).or_default();
            }
        }
        长河事件::任务状态 { 任务id, 状态 } => {
            后.任务.entry(*任务id).or_default().状态 = 状态.clone();
        }
        长河事件::过程事件 { 任务id, 序号, 类别, 角色, 工具名, 内容 } => {
            if *序号 > 后.过程游标 {
                后.过程游标 = *序号;
                后.任务.entry(*任务id).or_default().过程.push(过程条 {
                    序号: *序号,
                    类别: 类别.clone(),
                    角色: 角色.clone(),
                    工具名: 工具名.clone(),
                    内容: 内容.clone(),
                });
            }
        }
        长河事件::河错 { .. } => {
            // 错误事件由显影层呈现；归约不改结构（避免错误污染视图形状）
        }
        长河事件::清河 {} => {
            后 = 会话视图::default();
        }
    }
    后
}

/// 段选择器（内部）
enum 段择 {
    正文,
    思考,
}

fn 找消息<'a>(后: &'a mut 会话视图, 消息id: &str) -> Option<&'a mut 消息视图> {
    后.消息.iter_mut().find(|m| m.消息id == 消息id)
}

fn 开消息<'a>(后: &'a mut 会话视图, 消息id: &str, 向: 方向, 角色: &str) -> &'a mut 消息视图 {
    if 找消息(后, 消息id).is_none() {
        后.消息.push(消息视图 {
            消息id: 消息id.to_string(),
            方向: 向,
            角色: 角色.to_string(),
            段: Vec::new(),
            在流: false,
        });
    }
    找消息(后, 消息id).expect("消息已确保存在")
}

/// 追加正文/思考段文本：有对应在流段则续写，否则新开段
fn 追段文(后: &mut 会话视图, 消息id: &str, 择: &段择, 文: &str) {
    let Some(消息) = 找消息(后, 消息id) else { return };
    match 择 {
        段择::正文 => match 消息.段.last_mut() {
            Some(段视图::正文 { 文: 累 }) if 消息.在流 => 累.push_str(文),
            _ => 消息.段.push(段视图::正文 { 文: 文.to_string() }),
        },
        段择::思考 => match 消息.段.last_mut() {
            Some(段视图::思考 { 文: 累 }) => 累.push_str(文),
            _ => 消息.段.push(段视图::思考 { 文: 文.to_string() }),
        },
    }
}

fn 找工具段<'a>(后: &'a mut 会话视图, 调用id: &str) -> Option<&'a mut 段视图> {
    找消息(后, 调用id).and_then(|m| {
        m.段.iter_mut().rev().find(|s| matches!(s, 段视图::工具 { .. }))
    })
}

#[cfg(test)]
mod 归约验证 {
    use super::*;

    /// 逐事件折叠（模拟长河：视图 × 事件 → 视图）
    fn 逐条归约(事们: &[长河事件]) -> 会话视图 {
        let mut 视 = 会话视图::default();
        for 事 in 事们 {
            视 = 归约(&视, 事);
        }
        视
    }

    #[test]
    fn 回复增量拼接与收尾() {
        let 视 = 逐条归约(&[
            长河事件::回复开始 { 消息id: "m1".into(), 角色: "道祖".into() },
            长河事件::回复增量 { 消息id: "m1".into(), 文: "你好".into() },
            长河事件::回复增量 { 消息id: "m1".into(), 文: "，界主".into() },
            长河事件::回复结束 { 消息id: "m1".into() },
        ]);
        assert_eq!(视.消息.len(), 1);
        assert!(!视.消息[0].在流);
        assert_eq!(视.消息[0].段, vec![段视图::正文 { 文: "你好，界主".into() }]);
    }

    #[test]
    fn 过程事件按游标去重_乱序旧包丢弃() {
        let 视 = 逐条归约(&[
            长河事件::过程事件 { 任务id: 1, 序号: 2, 类别: "执行".into(), 角色: "大罗".into(), 工具名: None, 内容: "乙".into() },
            长河事件::过程事件 { 任务id: 1, 序号: 1, 类别: "执行".into(), 角色: "大罗".into(), 工具名: None, 内容: "甲".into() },
            长河事件::过程事件 { 任务id: 1, 序号: 3, 类别: "执行".into(), 角色: "大罗".into(), 工具名: None, 内容: "丙".into() },
        ]);
        let 过程 = &视.任务[&1].过程;
        assert_eq!(过程.len(), 2);
        assert_eq!(过程[0].内容, "乙");
        assert_eq!(视.过程游标, 3);
    }

    #[test]
    fn 河错不改视图结构_清河重置视图() {
        let 前 = 逐条归约(&[
            长河事件::河错 { 来源: "道祖".into(), 文: "x".into() },
            长河事件::回复开始 { 消息id: "m1".into(), 角色: "道祖".into() },
            长河事件::任务状态 { 任务id: 7, 状态: "已完成".into() },
        ]);
        assert_eq!(前.消息.len(), 1);
        let 后 = 归约(&前, &长河事件::清河 {});
        assert_eq!(后, 会话视图::default());
    }

    #[test]
    fn 工具段与调用一一对应() {
        let 视 = 逐条归约(&[
            长河事件::工具开始 { 调用id: "c1".into(), 名称: "读文件".into() },
            长河事件::工具参数 { 调用id: "c1".into(), 片段: "{\"路".into() },
            长河事件::工具参数 { 调用id: "c1".into(), 片段: "径\":1}".into() },
            长河事件::工具结果 { 调用id: "c1".into(), 文: "ok".into() },
        ]);
        assert_eq!(
            视.消息[0].段,
            vec![段视图::工具 { 名称: "读文件".into(), 参数: "{\"路径\":1}".into(), 结果: "ok".into() }]
        );
    }
}
