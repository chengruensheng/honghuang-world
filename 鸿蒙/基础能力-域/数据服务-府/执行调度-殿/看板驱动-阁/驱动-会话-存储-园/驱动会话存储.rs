// 驱动会话存储：把每次「发布 → 看板五层驱动」固化为可回放的驱动会话。
//
// 落盘布局（目录非空时）：
//   目录/会话-{id}.json       —— 会话元数据（清单一项）
//   目录/会话-{id}-事件.jsonl —— 过程事件追加写入（每行一个 JSON）
// 目录为空时（persistence.dir 未配置）退化为纯内存暂存，重启丢失（与旧版一致）。
// 回放接口对单会话事件做最新 5000 条截断，避免超大会话拖垮前端。
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use hm_agent::任务项;
use hm_content_contract::对话消息;
use crate::驱动过程事件;

/// 驱动会话最多回放的事件条数（超限仅保留最新，与过程事件环形上限解耦）
pub const 会话事件回放上限: usize = 5000;

/// 运行检查点消息快照上限：超限仅保留最新，避免超大会话（省盘省流）
pub const 检查点消息上限: usize = 50;

/// 会话元数据/检查点文件后缀（JSON 格式）
const 会话文件后缀: &str = ".json";

/// 会话摘要（清单一项；元数据落盘形态，含反序列化以回读）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 驱动会话摘要 {
    pub 会话id: u64,
    /// 会话创建的秒级时间戳
    pub 创建时间: u64,
    /// 发起方式：发布自动 / 手动驱动 / 手动到空闲
    pub 发起方式: String,
    /// 会话内被推进的任务 id（去重，可能多个：到空闲可推进多任务）
    pub 任务id列表: Vec<u64>,
    /// 运行中 / 已完成 / 空闲 / 失败
    pub 状态: String,
    /// 结果摘要（最近结果；会话未结束时为 None）
    pub 结果摘要: Option<String>,
    /// 会话累计事件数
    pub 事件数: u64,
}

/// 会话详情（回放返回体：元数据 + 全程事件）
#[derive(Debug, Serialize)]
pub struct 驱动会话详情 {
    #[serde(flatten)]
    pub 摘要: 驱动会话摘要,
    pub 事件: Vec<驱动过程事件>,
}

/// 会话清单响应
#[derive(Debug, Serialize)]
pub struct 会话清单响应 {
    pub 会话: Vec<驱动会话摘要>,
}

/// 运行检查点：某会话某任务某阶段的智能体运行断点（Resume/Fork 恢复用）。
/// 落盘：`目录/会话-{会话id}-运行-{任务id}.json`（每任务一个，覆盖写最新）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 运行检查点 {
    pub 会话id: u64,
    /// 被推进的任务 id
    pub 任务id: u64,
    /// 承接角色显示名（道祖/圣人/大罗金仙/准圣/太乙金仙）
    pub 角色: String,
    /// 该阶段提示文本（定位阶段来源，审计用）
    pub 阶段提示: String,
    /// LLM 对话消息快照（截至最后一轮末，含系统提示/初始注入/历史轮次）
    pub 消息: Vec<对话消息>,
    /// 任务清单快照
    pub 任务清单: Vec<任务项>,
    /// 已完成轮数
    pub 轮次: usize,
    /// 写入秒级时间戳
    pub 时间: u64,
}

/// 驱动会话存储（被看板驱动台持有，串行单驱动，内部靠 Mutex 保证并发写安全）
pub struct 驱动会话存储 {
    目录: Mutex<Option<String>>,
    /// 纯内存模式（目录为空）时暂存会话摘要
    内存会话: Mutex<Vec<驱动会话摘要>>,
    /// 纯内存模式时的会话事件（(会话id, 事件)）
    内存事件: Mutex<Vec<(u64, 驱动过程事件)>>,
    /// 运行检查点（双写：内存随时可恢复 + 落盘重启可恢复）
    内存检查点: Mutex<Vec<运行检查点>>,
}

impl 驱动会话存储 {
    pub fn 新(目录: Option<String>) -> Self {
        if let Some(d) = &目录 {
            let _ = std::fs::create_dir_all(d);
        }
        驱动会话存储 {
            目录: Mutex::new(目录),
            内存会话: Mutex::new(Vec::new()),
            内存事件: Mutex::new(Vec::new()),
            内存检查点: Mutex::new(Vec::new()),
        }
    }

    /// 设置落盘目录（None=纯内存）；有目录则确保存在
    pub fn 设置目录(&self, 目录: Option<String>) {
        if let Some(d) = &目录 {
            let _ = std::fs::create_dir_all(d);
        }
        *self.目录.lock().expect("会话目录锁中毒") = 目录;
    }

    fn 当前目录(&self) -> Option<String> {
        self.目录.lock().expect("会话目录锁中毒").clone()
    }

    /// 创建会话（状态=运行中），返回会话id
    pub fn 创建会话(&self, 发起方式: &str) -> u64 {
        let id = 毫秒时间戳();
        let 会话 = 驱动会话摘要 {
            会话id: id,
            创建时间: hm_contract::当前时间戳(),
            发起方式: 发起方式.to_string(),
            任务id列表: Vec::new(),
            状态: "运行中".to_string(),
            结果摘要: None,
            事件数: 0,
        };
        self.内存会话.lock().expect("会话内存锁中毒").push(会话.clone());
        if let Some(d) = self.当前目录() {
            self.写元数据(&d, &会话);
            let _ = std::fs::write(self.事件文件(&d, id), "");
        }
        id
    }

    /// 追加一条过程事件（双写：内存 + 落盘）
    pub fn 追加事件(&self, 会话id: u64, 事件: &驱动过程事件) {
        {
            let mut 事件内存 = self.内存事件.lock().expect("会话事件锁中毒");
            事件内存.push((会话id, 事件.clone()));
        }
        let 快照 = {
            let mut 会话内存 = self.内存会话.lock().expect("会话内存锁中毒");
            match 会话内存.iter_mut().find(|s| s.会话id == 会话id) {
                Some(会话) => {
                    会话.事件数 += 1;
                    if let Some(任务id) = 事件.任务id {
                        if !会话.任务id列表.contains(&任务id) {
                            会话.任务id列表.push(任务id);
                        }
                    }
                    会话.clone()
                }
                None => return,
            }
        };
        if let Some(d) = self.当前目录() {
            let _ = 追加事件行(&self.事件文件(&d, 会话id), 事件);
            self.写元数据(&d, &快照);
        }
    }

    /// 结束会话：更新状态 + 结果摘要
    pub fn 结束会话(&self, 会话id: u64, 状态: &str, 结果摘要: Option<String>) {
        let 快照 = {
            let mut 会话内存 = self.内存会话.lock().expect("会话内存锁中毒");
            match 会话内存.iter_mut().find(|s| s.会话id == 会话id) {
                Some(会话) => {
                    会话.状态 = 状态.to_string();
                    会话.结果摘要 = 结果摘要.clone();
                    会话.clone()
                }
                None => return,
            }
        };
        if let Some(d) = self.当前目录() {
            self.写元数据(&d, &快照);
        }
    }

    /// 保存某会话某任务的运行检查点（每任务一个，覆盖写最新；内存+落盘双写）。
    /// 消息快照超限仅保留最新 `检查点消息上限` 条。
    pub fn 保存检查点(&self, 检查点: &运行检查点) {
        let mut 快照 = 检查点.clone();
        if 快照.消息.len() > 检查点消息上限 {
            let 起点 = 快照.消息.len() - 检查点消息上限;
            快照.消息 = 快照.消息.split_off(起点);
        }
        {
            let mut 内存 = self.内存检查点.lock().expect("检查点内存锁中毒");
            内存.retain(|c| !(c.会话id == 快照.会话id && c.任务id == 快照.任务id));
            内存.push(快照.clone());
        }
        if let Some(d) = self.当前目录() {
            if let Ok(文本) = serde_json::to_string_pretty(&快照) {
                let _ = std::fs::write(self.检查点文件(&d, 快照.会话id, 快照.任务id), 文本);
            }
        }
    }

    /// 读取某会话某任务的运行检查点（优先盘，其次内存）；不存在返回 None。
    pub fn 读取检查点(&self, 会话id: u64, 任务id: u64) -> Option<运行检查点> {
        if let Some(d) = self.当前目录() {
            if let Ok(文本) = std::fs::read_to_string(self.检查点文件(&d, 会话id, 任务id)) {
                if let Ok(检查点) = serde_json::from_str::<运行检查点>(&文本) {
                    return Some(检查点);
                }
            }
        }
        self.内存检查点
            .lock()
            .expect("检查点内存锁中毒")
            .iter()
            .find(|c| c.会话id == 会话id && c.任务id == 任务id)
            .cloned()
    }

    fn 检查点文件(&self, 目录: &str, 会话id: u64, 任务id: u64) -> String {
        format!("{目录}/会话-{会话id}-运行-{任务id}{会话文件后缀}")
    }

    /// 会话清单（按会话id降序——id 为毫秒时间戳即创建先后）
    pub fn 清单(&self) -> Vec<驱动会话摘要> {
        let mut 会话 = match self.当前目录() {
            Some(d) => self.清单从盘(&d),
            None => self.内存会话.lock().expect("会话内存锁中毒").clone(),
        };
        会话.sort_by(|a, b| b.会话id.cmp(&a.会话id));
        会话
    }

    fn 清单从盘(&self, 目录: &str) -> Vec<驱动会话摘要> {
        let mut 会话 = Vec::new();
        if let Ok(条目) = std::fs::read_dir(目录) {
            for 项 in 条目.flatten() {
                let 名 = 项.file_name().to_string_lossy().to_string();
                if 名.starts_with("会话-") && 名.ends_with(会话文件后缀) {
                    if let Ok(文本) = std::fs::read_to_string(项.path()) {
                        if let Ok(摘要) = serde_json::from_str::<驱动会话摘要>(&文本) {
                            会话.push(摘要);
                        }
                    }
                }
            }
        }
        会话
    }

    /// 回放某会话：元数据 + 全程事件（最新上限条）
    pub fn 回放(&self, 会话id: u64) -> Option<驱动会话详情> {
        match self.当前目录() {
            Some(d) => {
                let 摘要 = self.清单从盘(&d).into_iter().find(|s| s.会话id == 会话id)?;
                let 事件 = 读到事件(&self.事件文件(&d, 会话id));
                Some(驱动会话详情 { 摘要, 事件: 截断最新(事件) })
            }
            None => {
                let 摘要 = self.内存会话.lock().expect("会话内存锁中毒").iter().find(|s| s.会话id == 会话id)?.clone();
                let 事件 = self
                    .内存事件
                    .lock()
                    .expect("会话事件锁中毒")
                    .iter()
                    .filter(|(id, _)| *id == 会话id)
                    .map(|(_, e)| e.clone())
                    .collect();
                Some(驱动会话详情 { 摘要, 事件: 截断最新(事件) })
            }
        }
    }

    fn 写元数据(&self, 目录: &str, 会话: &驱动会话摘要) {
        if let Ok(文本) = serde_json::to_string_pretty(会话) {
            let _ = std::fs::write(self.会话文件(目录, 会话.会话id), 文本);
        }
    }

    fn 会话文件(&self, 目录: &str, id: u64) -> String {
        format!("{目录}/会话-{id}{会话文件后缀}")
    }

    fn 事件文件(&self, 目录: &str, id: u64) -> String {
        format!("{目录}/会话-{id}-事件.jsonl")
    }
}

fn 毫秒时间戳() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn 追加事件行(路径: &str, 事件: &驱动过程事件) -> std::io::Result<()> {
    use std::io::Write;
    let mut 文件 = std::fs::OpenOptions::new().create(true).append(true).open(路径)?;
    if let Ok(行) = serde_json::to_string(事件) {
        writeln!(文件, "{行}")?;
    }
    Ok(())
}

fn 读到事件(路径: &str) -> Vec<驱动过程事件> {
    let mut 事件 = Vec::new();
    if let Ok(文本) = std::fs::read_to_string(路径) {
        for 行 in 文本.lines() {
            if let Ok(e) = serde_json::from_str::<驱动过程事件>(&行) {
                事件.push(e);
            }
        }
    }
    事件
}

fn 截断最新(mut 事件: Vec<驱动过程事件>) -> Vec<驱动过程事件> {
    if 事件.len() > 会话事件回放上限 {
        let 起点 = 事件.len() - 会话事件回放上限;
        事件 = 事件.split_off(起点);
    }
    事件
}

