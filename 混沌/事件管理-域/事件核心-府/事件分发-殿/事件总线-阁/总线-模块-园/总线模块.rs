use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use hm_contract::{Component, 当前时间戳};
use hm_error::{Error, Result};
use hm_signal::{信号总线, 信号类型, 信号载荷};
use hm_signal::引擎支撑;
use hm_domain_contract::事件总线契约;
use serde::{Deserialize, Serialize};
use crate::事件定义_殿::{Event};

/// 可序列化的事件日志（EventBus 的持久化载体，订阅关系不落盘）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventLog {
    events: Vec<Event>,
    next_id: u64,
}

/// 事件总线：水之流动，订阅分发 + 事件历史 + 落盘持久化。
/// 事件以 id 为键存入 BTreeMap，按 id 查询 O(log n)，且保持发布顺序。
pub struct EventBus {
    subscribers: Vec<(String, Arc<dyn Fn(&Event) + Send + Sync>)>,
    events: BTreeMap<u64, Event>,
    next_id: u64,
    信号总线: Option<Arc<dyn 信号总线>>,
    持久化路径: Option<String>,
}

impl EventBus {
    pub fn new() -> Self {
        EventBus {
            subscribers: Vec::new(),
            events: BTreeMap::new(),
            next_id: 1,
            信号总线: None,
            持久化路径: None,
        }
    }

    引擎支撑!();

    /// 订阅某类型事件（处理器闭包，运行时态不落盘）
    pub fn 订阅(&mut self, 类型: &str, 处理器: Arc<dyn Fn(&Event) + Send + Sync>) {
        self.subscribers.push((类型.to_string(), 处理器));
    }

    /// 发布事件：创建、分发给匹配类型的处理器、记录历史，返回事件 id
    pub fn 发布(&mut self, 类型: String, 载荷: Vec<(String, String)>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let 摘要 = 载荷
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        let event = Event::新建(id, 类型.clone(), 载荷, 当前时间戳());
        for (订阅类型, 处理器) in &self.subscribers {
            if 订阅类型 == &类型 {
                处理器(&event);
            }
        }
        self.events.insert(id, event);
        self.发布信号(
            信号类型::事件发布,
            信号载荷 {
                类型: Some(类型),
                标识: Some(id.to_string()),
                内容: Some(摘要),
                ..信号载荷::default()
            },
        );
        self.自动保存();
        id
    }

    /// 按 id 查询
    pub fn 查询(&self, id: u64) -> Option<&Event> {
        self.events.get(&id)
    }

    /// 事件历史（按发布顺序）
    pub fn 全部(&self) -> Vec<&Event> {
        self.events.values().collect()
    }

    /// 保存到文件（仅落盘事件历史，订阅关系不落盘）
    pub fn 保存(&self, path: &str) -> Result<()> {
        let log = EventLog {
            events: self.events.values().cloned().collect(),
            next_id: self.next_id,
        };
        let content = toml::to_string(&log)
            .map_err(|e| Error::序列化(format!("序列化事件失败: {e}")))?;
        hm_contract::原子写入文件(path, &content)?;
        Ok(())
    }

    /// 从文件加载（还原事件历史，订阅关系需重新注册）
    pub fn 加载(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
        let log: EventLog = toml::from_str(&content)
            .map_err(|e| Error::反序列化(format!("解析事件文件失败: {e}")))?;
        Ok(EventBus {
            subscribers: Vec::new(),
            events: log.events.into_iter().map(|e| (e.id, e)).collect(),
            next_id: log.next_id,
            信号总线: None,
            持久化路径: None,
        })
    }

    /// 去重事件（土克水：记忆固化事件流），按类型与载荷合并重复事件，返回移除数量。
    /// 载荷排序后再比较，消除载荷键值顺序对去重结果的影响。
    pub fn 去重(&mut self) -> usize {
        let before = self.events.len();
        let mut 已见: HashSet<(String, Vec<(String, String)>)> = HashSet::new();
        self.events.retain(|_, e| {
            let mut 载荷 = e.载荷.clone();
            载荷.sort();
            已见.insert((e.类型.clone(), 载荷))
        });
        let 移除 = before - self.events.len();
        self.自动保存();
        移除
    }

}

impl Component for EventBus {
    fn name(&self) -> &'static str { "事件总线" }
}

impl 事件总线契约<Event> for EventBus {
    fn 订阅(&mut self, 类型: &str, 处理器: Arc<dyn Fn(&Event) + Send + Sync>) {
        EventBus::订阅(self, 类型, 处理器);
    }

    fn 发布(&mut self, 类型: String, 载荷: Vec<(String, String)>) -> u64 {
        EventBus::发布(self, 类型, 载荷)
    }

    fn 查询(&self, id: u64) -> Option<&Event> {
        EventBus::查询(self, id)
    }

    fn 全部(&self) -> Vec<&Event> {
        EventBus::全部(self)
    }

    fn 去重(&mut self) -> usize {
        EventBus::去重(self)
    }
}
