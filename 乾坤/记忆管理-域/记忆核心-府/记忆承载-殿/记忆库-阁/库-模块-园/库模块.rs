use std::collections::HashMap;
use std::sync::Arc;
use hm_contract::{Component, 当前时间戳};
use hm_error::{Error, Result};
use hm_signal::{信号总线, 信号类型, 信号载荷};
use hm_signal::引擎支撑;
use hm_domain_contract::记忆库契约;
use serde::{Deserialize, Serialize};
use crate::记忆定义_殿::{Memory};

/// 容量上限默认值：不限（约束由装配层注入）。
/// 可移植写法：64 位平台可序列化 i64::MAX，32 位平台回退 usize::MAX 避免溢出。
fn 默认容量上限() -> usize {
    match usize::try_from(i64::MAX) {
        Ok(上限) => 上限,
        Err(_) => usize::MAX,
    }
}

/// 记忆库：承载记忆，支持容量约束与落盘持久化。
/// 以 Vec 保存记忆保证持久化顺序，另建 id 索引使按 id 查询 O(1)。
#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryStore {
    memories: Vec<Memory>,
    next_id: u64,
    #[serde(default = "默认容量上限")]
    容量上限: usize,
    #[serde(skip)]
    索引: HashMap<u64, usize>,
    #[serde(skip)]
    信号总线: Option<Arc<dyn 信号总线>>,
    #[serde(skip)]
    持久化路径: Option<String>,
}

impl MemoryStore {
    pub fn new() -> Self {
        MemoryStore {
            memories: Vec::new(),
            next_id: 1,
            容量上限: 默认容量上限(),
            索引: HashMap::new(),
            信号总线: None,
            持久化路径: None,
        }
    }

    引擎支撑!();

    /// 设置记忆容量上限（木克土：过盛时新记忆降级为待归档）
    pub fn 设置容量上限(&mut self, 上限: usize) {
        self.容量上限 = 上限;
    }

    /// 写入记忆，返回记忆 id；记忆过盛时新记忆标注待归档（而非删除旧记忆）
    pub fn 写入(&mut self, 内容: String, 标签: String) -> Result<u64> {
        let 归档 = self.memories.len() >= self.容量上限;
        let id = self.next_id;
        self.next_id += 1;
        let mut memory = Memory::新建(id, 内容.clone(), 标签.clone(), 当前时间戳());
        memory.归档 = 归档;
        self.索引.insert(id, self.memories.len());
        self.memories.push(memory);
        self.发布信号(
            信号类型::记忆写入,
            信号载荷 {
                内容: Some(内容),
                标签: Some(标签),
                ..信号载荷::default()
            },
        );
        self.自动保存();
        Ok(id)
    }

    /// 按 id 查询
    pub fn 查询(&self, id: u64) -> Option<&Memory> {
        self.索引.get(&id).map(|&i| &self.memories[i])
    }

    /// 按标签查询
    pub fn 按标签(&self, 标签: &str) -> Vec<&Memory> {
        self.memories.iter().filter(|m| m.标签 == 标签).collect()
    }

    /// 全部记忆
    pub fn 全部(&self) -> Vec<&Memory> {
        self.memories.iter().collect()
    }

    /// 保存到文件（落盘）
    pub fn 保存(&self, path: &str) -> Result<()> {
        let content = toml::to_string(self)
            .map_err(|e| Error::序列化(format!("序列化记忆失败: {e}")))?;
        std::fs::write(path, content).map_err(Error::Io)?;
        Ok(())
    }

    /// 从文件加载（还原），并重建 id 索引
    pub fn 加载(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
        let mut store: MemoryStore = toml::from_str(&content)
            .map_err(|e| Error::反序列化(format!("解析记忆文件失败: {e}")))?;
        store.重建索引();
        Ok(store)
    }

    /// 重建 id 索引（加载后调用）
    fn 重建索引(&mut self) {
        self.索引.clear();
        for (i, m) in self.memories.iter().enumerate() {
            self.索引.insert(m.id, i);
        }
    }

}

impl Component for MemoryStore {
    fn name(&self) -> &'static str { "记忆库" }
}

impl 记忆库契约<Memory> for MemoryStore {
    fn 写入(&mut self, 内容: String, 标签: String) -> Result<u64> {
        MemoryStore::写入(self, 内容, 标签)
    }

    fn 查询(&self, id: u64) -> Option<&Memory> {
        MemoryStore::查询(self, id)
    }

    fn 按标签(&self, 标签: &str) -> Vec<&Memory> {
        MemoryStore::按标签(self, 标签)
    }

    fn 全部(&self) -> Vec<&Memory> {
        MemoryStore::全部(self)
    }

}
