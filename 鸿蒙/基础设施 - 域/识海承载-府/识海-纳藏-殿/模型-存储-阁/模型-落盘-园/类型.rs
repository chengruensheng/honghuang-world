//! 识海承载-府 · 核心类型：格位、记录、坐标、会话记录。

use serde::{Deserialize, Serialize};

/// 六范畴。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 范畴 { 目标, 规则, 自我, 程序, 世界, 经历 }

/// 固化度：经=不可改，权=可迭代。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 固化度 { 经, 权 }

/// 共享度：共享=复用，私有=隔离。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 共享度 { 共享, 私有 }

/// 顺序档位：最前 / 中间 / 最后。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 顺序档位 { 最前, 中间, 最后 }

/// 来源（可信度排序：代码 > 人类 > LLM）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum 来源 { 代码, LLM, 人类 }

/// 坐标层：项目 / 模块 / 文件 / 符号 / 代码（五级颗粒度）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum 坐标层 {
    #[default]
    项目,
    模块,
    文件,
    符号,
    代码,
}

/// 格位：心智模型的基本单元。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 格位 {
    pub 名字: String,
    pub 范畴: 范畴,
    pub 种子提示词: String,
    pub 来源: 来源,
    pub 固化度: 固化度,
    pub 共享度: 共享度,
    pub 顺序档位: 顺序档位,
    pub 推荐位置: String,
    pub 存储: String,
}

impl 格位 {
    /// 构造一个格位（推荐位置留空，存储文件名 = 格位名.jsonl）。
    pub fn 新(
        名字: &str,
        范畴: 范畴,
        种子提示词: &str,
        来源: &str,
        固化度: 固化度,
        共享度: 共享度,
        顺序档位: 顺序档位,
    ) -> 格位 {
        格位 {
            名字: 名字.to_string(),
            范畴,
            种子提示词: 种子提示词.to_string(),
            来源: 解析来源(来源),
            固化度,
            共享度,
            顺序档位,
            推荐位置: String::new(),
            存储: format!("{}.jsonl", 名字),
        }
    }
}

/// 坐标：微观定位（层 · 对象 · 属性）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 坐标 {
    pub 层: 坐标层,
    pub 对象: String,
    pub 属性: String,
}

/// 记录：格位里的一条内容，必带证据。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 记录 {
    pub 格位名: String,
    pub 内容: String,
    pub 证据: String,
    pub 时间戳: u64,
    pub 来源: 来源,
    pub 前记录: Option<String>,
    pub 坐标: Option<坐标>,
    pub 失效: bool,
}

impl 记录 {
    /// 构造一条记录（时间戳 = 当前毫秒，前记录/坐标留空，失效 = false）。
    pub fn 新(格位名: &str, 内容: &str, 证据: &str, 来源: &str) -> 记录 {
        记录 {
            格位名: 格位名.to_string(),
            内容: 内容.to_string(),
            证据: 证据.to_string(),
            时间戳: 当前毫秒(),
            来源: 解析来源(来源),
            前记录: None,
            坐标: None,
            失效: false,
        }
    }
}

/// 工具清单：系统可用工具（硬编码配置，不占格位名额）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 工具清单 {
    pub 工具们: Vec<String>,
}

/// 会话记录：一次任务执行的完整工作记忆，不截断。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 会话记录 {
    pub 会话id: String,
    pub 内容: String,
    pub 时间戳: u64,
}

/// 时间戳（unix 毫秒）。
pub fn 当前毫秒() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|时长| 时长.as_millis() as u64)
        .unwrap_or(0)
}

/// 解析来源字符串 → 来源枚举（组合来源取主要填充源）。
pub fn 解析来源(值: &str) -> 来源 {
    if 值.starts_with("人类") {
        来源::人类
    } else if 值.starts_with("代码") {
        来源::代码
    } else {
        来源::LLM
    }
}

/// 来源可信度（代码 > 人类 > LLM），用于防幻觉排序。
pub fn 来源可信度(值: 来源) -> u8 {
    match 值 {
        来源::代码 => 3,
        来源::人类 => 2,
        来源::LLM => 1,
    }
}

/// 微观坐标归并回宏观语义格位（结构 / 文件 / 变更）。
pub fn 归并到语义格位(层: 坐标层) -> &'static str {
    match 层 {
        坐标层::项目 | 坐标层::模块 => "结构",
        坐标层::文件 => "文件",
        坐标层::符号 | 坐标层::代码 => "变更",
    }
}
