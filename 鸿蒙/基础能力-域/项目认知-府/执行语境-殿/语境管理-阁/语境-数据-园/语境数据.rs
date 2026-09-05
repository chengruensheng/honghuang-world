use serde::{Deserialize, Serialize};

/// 消息角色：过程上下文中一条消息的来源
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum 消息角色 {
    系统,
    用户,
    助手,
    工具结果,
}

/// 语境消息：过程上下文的一条消息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct 语境消息 {
    pub 角色: 消息角色,
    pub 内容: String,
}

/// 过程上下文：执行过程的语境流转；有就加，超阈值压缩（压缩策略后续接入）
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