use serde::{Deserialize, Serialize};
use hm_contract::当前时间戳;

/// 消息角色：过程上下文中一条消息的来源
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum 消息角色 {
    系统,
    用户,
    助手,
    工具结果,
    /// 系统信号/事件（如纠错暂存记录）
    信号,
}

/// 语境消息：过程上下文的一条消息
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 语境消息 {
    pub 角色: 消息角色,
    pub 内容: String,
}

/// 过程上下文：执行过程的语境流转；有就加，超阈值压缩（压缩策略由 上下文压缩 模块承担）
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct 过程上下文 {
    pub 消息集: Vec<语境消息>,
}

impl 过程上下文 {
    pub fn 新() -> Self {
        过程上下文::default()
    }

    /// 追加一条消息
    pub fn 追加(&mut self, 消息: 语境消息) {
        self.消息集.push(消息);
    }

    /// 当前消息条数
    pub fn 消息数(&self) -> usize {
        self.消息集.len()
    }

    /// 列出全部消息
    pub fn 全部(&self) -> &[语境消息] {
        &self.消息集
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 上下文库：临时态消息流（对齐原型 上下文库.py）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 上下文消息：带自增 id 与时间戳的不可变消息
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 上下文消息 {
    pub id: u64,
    pub 角色: 消息角色,
    pub 内容: String,
    pub 时间戳: u64,
}

/// 上下文库：AI 实际执行留下的轨迹（流式追加 + 硬上限兜底 + 最近相关查询）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct 上下文库 {
    pub 消息流: Vec<上下文消息>,
    下一id: u64,
    最大条数: usize,
}

impl Default for 上下文库 {
    fn default() -> Self {
        上下文库 {
            消息流: Vec::new(),
            下一id: 1,
            最大条数: 1000,
        }
    }
}

impl 上下文库 {
    pub fn 新() -> Self {
        上下文库::default()
    }

    /// 带硬上限构造（压缩未跑时的兜底丢弃）
    pub fn 新_带上限(最大条数: usize) -> Self {
        上下文库 {
            消息流: Vec::new(),
            下一id: 1,
            最大条数,
        }
    }

    /// 从已持久化消息流重建（供三态存储加载：恢复 下一id 单调性 + 截断到上限）
    pub fn 导入(消息流: Vec<上下文消息>, 最大条数: usize) -> Self {
        let 长度 = 消息流.len();
        let 消息流 = if 长度 > 最大条数 {
            消息流.into_iter().skip(长度 - 最大条数).collect()
        } else {
            消息流
        };
        let 下一id = 消息流.iter().map(|m| m.id).max().unwrap_or(0) + 1;
        上下文库 { 消息流, 下一id, 最大条数 }
    }

    /// 追加一条消息，返回新消息；超过硬上限时丢弃最早
    pub fn 追加(&mut self, 角色: 消息角色, 内容: impl Into<String>) -> 上下文消息 {
        let 消息 = 上下文消息 {
            id: self.下一id,
            角色,
            内容: 内容.into(),
            时间戳: 当前时间戳(),
        };
        self.下一id += 1;
        self.消息流.push(消息.clone());
        if self.消息流.len() > self.最大条数 {
            self.消息流.remove(0);
        }
        消息
    }

    /// 当前条数
    pub fn 长度(&self) -> usize {
        self.消息流.len()
    }

    /// 列出全部消息
    pub fn 全部(&self) -> &[上下文消息] {
        &self.消息流
    }

    /// 最近 n 条（不足 n 返回全部）
    pub fn 最近(&self, n: usize) -> Vec<&上下文消息> {
        self.消息流
            .iter()
            .skip(self.消息流.len().saturating_sub(n))
            .collect()
    }

    /// 清空消息流（id 不重置，保持稳定）
    pub fn 清空(&mut self) {
        self.消息流.clear();
    }

    /// 最近相关：按关键词命中数排序取前 k 条
    pub fn 最近相关(&self, 关键词: &str, k: usize) -> Vec<&上下文消息> {
        if 关键词.is_empty() {
            return self.最近(k);
        }
        let 词: Vec<String> = 关键词
            .split(|c: char| !c.is_ascii_alphanumeric() && !('\u{4e00}'..='\u{9fff}').contains(&c))
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect();
        let mut 命中: Vec<(usize, &上下文消息)> = self
            .消息流
            .iter()
            .map(|消息| {
                let 内容 = 消息.内容.to_lowercase();
                let 数 = 词.iter().filter(|w| 内容.contains(&**w)).count();
                (数, 消息)
            })
            .filter(|(数, _)| *数 > 0)
            .collect();
        命中.sort_by(|a, b| b.0.cmp(&a.0));
        命中.into_iter().take(k).map(|(_, 消息)| 消息).collect()
    }
}
