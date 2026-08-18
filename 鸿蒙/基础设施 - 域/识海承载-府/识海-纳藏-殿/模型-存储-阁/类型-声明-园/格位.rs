//! 格位 - 体系 - 类型：格位、记录、会话记录、工具清单。

use serde::{Deserialize, Serialize};

/// 六范畴。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 范畴 {
    目标,
    规则,
    自我,
    程序,
    世界,
    经历,
}

/// 固化度：经=不可改，权=可迭代。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 固化度 {
    经,
    权,
}

/// 共享度：共享=复用，私有=隔离。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 共享度 {
    共享,
    私有,
}

/// 顺序档位：最前 / 中间 / 最后。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 顺序档位 {
    最前,
    中间,
    最后,
}

/// 来源（可信度排序：代码 > 人类 > LLM）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum 来源 {
    代码,
    LLM,
    人类,
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
    pub token上限: usize,
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
        let token上限 = 默认token上限(来源, &顺序档位);
        格位 {
            名字: 名字.to_string(),
            范畴,
            种子提示词: 种子提示词.to_string(),
            来源: 解析来源(来源),
            固化度,
            共享度,
            顺序档位,
            token上限,
            推荐位置: String::new(),
            存储: format!("{}.jsonl", 名字),
        }
    }
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
    pub 失效: bool,
    /// 实体键：同实体多条记录成链（块），链头 = 最新一条。
    /// 默认取格位名（每格位一个块），代码扫描快照可显式覆盖。
    #[serde(default)]
    pub 实体键: String,
}

impl 记录 {
    /// 构造一条记录（时间戳 = 当前毫秒，前记录留空，失效 = false，实体键 = 格位名）。
    pub fn 新(格位名: &str, 内容: &str, 证据: &str, 来源: &str) -> 记录 {
        记录 {
            格位名: 格位名.to_string(),
            内容: 内容.to_string(),
            证据: 证据.to_string(),
            时间戳: 当前毫秒(),
            来源: 解析来源(来源),
            前记录: None,
            失效: false,
            实体键: 格位名.to_string(),
        }
    }
}

/// 工具清单：系统可用工具（硬编码配置，不占格位名额）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 工具清单 {
    pub 工具们: Vec<String>,
}

/// 全量工具名（与 道术施展-府·手脚-施展-殿 一一对应）。
pub const 全部工具名: [&str; 10] = [
    "写文件",
    "读文件",
    "改文件",
    "删文件",
    "列举目录",
    "寻找文件",
    "搜索内容",
    "运行命令",
    "读格位",
    "查格位历史",
];

impl 工具清单 {
    /// 全量工具（硬编码配置，随道术施展-府 手脚-施展-殿 同步）。
    pub fn 全部() -> 工具清单 {
        工具清单 {
            工具们: 全部工具名.iter().map(|名| 名.to_string()).collect(),
        }
    }
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

/// 默认 token 上限：按来源与顺序档位推导（中文 1 字 ≈ 1 token 近似）。
/// 人类经格位内容精炼给充足配额；代码世界格位折叠后很小给有限配额；
/// 当前档（最后）内容少给较小配额。独立计数，互不挤占。
pub fn 默认token上限(来源: &str, 顺序: &顺序档位) -> usize {
    match 顺序 {
        顺序档位::最前 => {
            if 来源.starts_with("人类") {
                500
            } else if 来源.starts_with("代码") {
                300
            } else {
                400
            }
        }
        顺序档位::最后 => 300,
        顺序档位::中间 => {
            if 来源.starts_with("人类") {
                500
            } else {
                400
            }
        }
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
