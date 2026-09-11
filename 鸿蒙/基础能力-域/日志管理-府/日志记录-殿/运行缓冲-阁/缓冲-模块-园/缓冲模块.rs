use serde::{Deserialize, Serialize};
use hm_contract::当前时间戳;

/// 运行日志默认环形缓冲容量（超出后丢弃最旧记录）
const 默认容量: usize = 500;

/// 日志记录：一条运行流转记录（标签 / 样式 / 内容 / 时间戳）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 日志记录 {
    pub 标签: String,
    pub 样式: String,
    pub 内容: String,
    pub 时间: u64,
}

/// 运行日志记录器：内存环形缓冲，记录业务运行流转，供客户端日志视图读取
#[derive(Debug, Clone)]
pub struct 运行日志记录器 {
    记录: Vec<日志记录>,
    容量: usize,
}

impl 运行日志记录器 {
    pub fn new() -> Self {
        Self::new_with_容量(默认容量)
    }

    /// 指定环形缓冲容量（超出后丢弃最旧记录）
    pub fn new_with_容量(容量: usize) -> Self {
        运行日志记录器 {
            记录: Vec::new(),
            容量,
        }
    }

    /// 追加一条日志记录；超出容量时丢弃最旧
    pub fn 记日志(&mut self, 标签: String, 样式: String, 内容: String) {
        self.记录.push(日志记录 {
            标签,
            样式,
            内容,
            时间: 当前时间戳(),
        });
        if self.记录.len() > self.容量 {
            self.记录.remove(0);
        }
    }

    /// 全部日志记录（按时间先后）
    pub fn 全部(&self) -> Vec<日志记录> {
        self.记录.clone()
    }
}