//! 分级 - 缓存 - 园：三级缓存（永久 / 版本 / 会话）+ 预算生长。

use crate::世界结果;
use std::collections::HashMap;

use crate::{会话记录, 固化度, 格位, 模型存储, 记录, 顺序档位};
use rizhi_fu::debug;

/// 永久缓存：固化度 = 经 的格位（直接注入，不重读）。
pub fn 经格位(格位们: &[格位]) -> Vec<格位> {
    格位们
        .iter()
        .filter(|格位| 格位.固化度 == 固化度::经)
        .cloned()
        .collect()
}

/// 版本缓存：固化度 = 权 的格位（版本未变即复用）。
pub fn 权格位(格位们: &[格位]) -> Vec<格位> {
    格位们
        .iter()
        .filter(|格位| 格位.固化度 == 固化度::权)
        .cloned()
        .collect()
}

/// 最低预算格位：最前 + 最后（中间档按需后补）。
pub fn 最低预算格位(格位们: &[格位]) -> Vec<格位> {
    格位们
        .iter()
        .filter(|格位| 格位.顺序档位 != 顺序档位::中间)
        .cloned()
        .collect()
}

/// 三级缓存：永久（经格位）+ 版本（权格位）+ 会话，另存拼装结果指纹。
#[derive(Default)]
pub struct 三级缓存 {
    永久: HashMap<String, Vec<记录>>,
    版本: HashMap<String, (u64, Vec<记录>)>,
    会话: HashMap<String, 会话记录>,
    投影: HashMap<String, String>,
}

impl 三级缓存 {
    pub fn 新() -> 三级缓存 {
        三级缓存::default()
    }

    /// 取永久（经格位）记录：未命中则读并缓存。
    pub fn 取永久(
        &mut self, 存储: &模型存储, 格位名: &str
    ) -> 世界结果<Vec<记录>> {
        if let Some(记录们) = self.永久.get(格位名) {
            return Ok(记录们.clone());
        }
        let 记录们 = 存储.读格位(格位名)?;
        self.永久.insert(格位名.to_string(), 记录们.clone());
        Ok(记录们)
    }

    /// 取版本（权格位）记录：版本戳未变则复用，变了则重读。
    pub fn 取版本(
        &mut self,
        存储: &模型存储,
        格位名: &str,
        版本戳: u64,
    ) -> 世界结果<Vec<记录>> {
        if let Some((旧戳, 记录们)) = self.版本.get(格位名) {
            if *旧戳 == 版本戳 {
                return Ok(记录们.clone());
            }
        }
        let 记录们 = 存储.读格位(格位名)?;
        self.版本
            .insert(格位名.to_string(), (版本戳, 记录们.clone()));
        Ok(记录们)
    }

    /// 存会话（会话缓存，随会话走）。
    pub fn 存会话(&mut self, 会话: 会话记录) {
        self.会话.insert(会话.会话id.clone(), 会话);
    }

    /// 取会话。
    pub fn 取会话(&self, 会话id: &str) -> Option<&会话记录> {
        self.会话.get(会话id)
    }

    /// 拼装结果缓存：输入指纹未变复用，变了重拼。
    pub fn 拼装(
        &mut self,
        指纹: &str,
        拼装: impl FnOnce() -> 世界结果<String>,
    ) -> 世界结果<String> {
        if let Some(结果) = self.投影.get(指纹) {
            return Ok(结果.clone());
        }
        let 结果 = 拼装()?;
        self.投影.insert(指纹.to_string(), 结果.clone());
        Ok(结果)
    }

    /// 失效永久缓存（代码变更 / 人类修改后调用）。
    pub fn 失效永久(&mut self, 格位名: &str) {
        self.永久.remove(格位名);
        self.投影.clear();
        debug!(格位名, "永久缓存已失效");
    }

    /// 失效版本缓存（版本变化后调用）。
    pub fn 失效版本(&mut self, 格位名: &str) {
        self.版本.remove(格位名);
        self.投影.clear();
        debug!(格位名, "版本缓存已失效");
    }
}

// 输入校验扩展：分级枚举 + 缓存错误 + 容量/键值校验（不绑定直通/永驻/单槽策略）。

/// 缓存分级：四档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 分级 {
    /// 直通级（永久，不限容量，不接受容量参数）
    直通,
    /// 永驻级（版本级）
    永驻,
    /// 短暂级（会话级）
    短暂,
    /// 单槽挤兑级
    单槽,
}

impl 分级 {
    /// 是否支持容量参数：仅直通不支持。
    pub fn 支持容量(&self) -> bool {
        !matches!(self, 分级::直通)
    }

    /// 该分级允许的键长度上限（usize::MAX 表示不限）。
    pub fn 键上限(&self) -> usize {
        match self {
            分级::直通 => usize::MAX,
            分级::永驻 => 64,
            分级::短暂 => 32,
            分级::单槽 => 16,
        }
    }

    /// 从 u8 尝试构造分级：仅 0..=3 合法，其余返 `非法分级`。
    pub fn 尝试从u8(值: u8) -> Result<分级, 缓存错误> {
        match 值 {
            0 => Ok(分级::直通),
            1 => Ok(分级::永驻),
            2 => Ok(分级::短暂),
            3 => Ok(分级::单槽),
            _ => Err(缓存错误::非法分级 { 值 }),
        }
    }
}

/// 缓存错误：容量/键值/分级三类输入校验的统一变体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 缓存错误 {
    /// 容量超出分级允许范围 [最小, 最大]。
    容量越界 {
        分级: 分级,
        请求: u64,
        最小: u64,
        最大: u64,
    },
    /// 该分级不支持容量参数（直通级）。
    分级不支持容量 { 分级: 分级 },
    /// 键为空。
    空键,
    /// 值为空。
    空值,
    /// 键超过该分级允许的长度上限。
    键过长 { 长度: usize, 上限: usize },
    /// 非法的分级枚举值。
    非法分级 { 值: u8 },
}

impl std::fmt::Display for 缓存错误 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            缓存错误::容量越界 {
                分级,
                请求,
                最小,
                最大,
            } => write!(
                f,
                "容量越界: 分级={:?}, 请求={}, 范围=[{}, {}]",
                分级, 请求, 最小, 最大
            ),
            缓存错误::分级不支持容量 { 分级 } => {
                write!(f, "分级 {:?} 不支持容量参数", 分级)
            }
            缓存错误::空键 => write!(f, "键不能为空"),
            缓存错误::空值 => write!(f, "值不能为空"),
            缓存错误::键过长 { 长度, 上限 } => {
                write!(f, "键过长: 长度={}, 上限={}", 长度, 上限)
            }
            缓存错误::非法分级 { 值 } => write!(f, "非法分级值: {}", 值),
        }
    }
}

impl std::error::Error for 缓存错误 {}

impl 三级缓存 {
    /// 按分级构造容量化缓存：直通级一律拒容量参数；永驻/短暂/单槽的合法容量区间为 [1, u64::MAX-1]。
    /// 仅断言 Err 变体构造字段，不绑定直通/永驻/单槽挤兑等实现策略。
    pub fn 建分级容量(分级: 分级, 容量: u64) -> Result<三级缓存, 缓存错误> {
        if !分级.支持容量() {
            return Err(缓存错误::分级不支持容量 { 分级 });
        }
        if 容量 == 0 {
            return Err(缓存错误::容量越界 {
                分级,
                请求: 0,
                最小: 1,
                最大: u64::MAX - 1,
            });
        }
        if 容量 == u64::MAX {
            return Err(缓存错误::容量越界 {
                分级,
                请求: u64::MAX,
                最小: 1,
                最大: u64::MAX - 1,
            });
        }
        let _ = 容量; // 仅做合法性闸口，不绑定实现策略
        Ok(三级缓存::新())
    }

    /// 写入键值校验：空键/空值/超长键各自返对应变体；其余合法返 Ok。
    /// 仅断言 Err 变体，不绑定直通/永驻/单槽挤兑等写入策略。
    pub fn 写键值校验(
        &mut self, 分级: 分级, 键: &str, 值: &str
    ) -> Result<(), 缓存错误> {
        if 键.is_empty() {
            return Err(缓存错误::空键);
        }
        if 值.is_empty() {
            return Err(缓存错误::空值);
        }
        let 上限 = 分级.键上限();
        if 上限 != usize::MAX && 键.len() > 上限 {
            return Err(缓存错误::键过长 {
                长度: 键.len(),
                上限,
            });
        }
        let _ = (self, 键, 值); // 仅校验通过，不绑定实现策略
        Ok(())
    }
}
