//! 分级 - 缓存 - 园：三级缓存（永久 / 版本 / 会话）+ 预算生长。

use std::collections::HashMap;

use crate::{格位, 记录, 会话记录, 模型存储, 固化度, 顺序档位};
use rizhi_fu::debug;

/// 永久缓存：固化度 = 经 的格位（直接注入，不重读）。
pub fn 经格位(格位们: &[格位]) -> Vec<格位> {
    格位们.iter().filter(|格位| 格位.固化度 == 固化度::经).cloned().collect()
}

/// 版本缓存：固化度 = 权 的格位（版本未变即复用）。
pub fn 权格位(格位们: &[格位]) -> Vec<格位> {
    格位们.iter().filter(|格位| 格位.固化度 == 固化度::权).cloned().collect()
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
    pub fn 取永久(&mut self, 存储: &模型存储, 格位名: &str) -> Result<Vec<记录>, String> {
        if let Some(记录们) = self.永久.get(格位名) {
            return Ok(记录们.clone());
        }
        let 记录们 = 存储.读格位(格位名)?;
        self.永久.insert(格位名.to_string(), 记录们.clone());
        Ok(记录们)
    }

    /// 取版本（权格位）记录：版本戳未变则复用，变了则重读。
    pub fn 取版本(&mut self, 存储: &模型存储, 格位名: &str, 版本戳: u64) -> Result<Vec<记录>, String> {
        if let Some((旧戳, 记录们)) = self.版本.get(格位名) {
            if *旧戳 == 版本戳 {
                return Ok(记录们.clone());
            }
        }
        let 记录们 = 存储.读格位(格位名)?;
        self.版本.insert(格位名.to_string(), (版本戳, 记录们.clone()));
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
    pub fn 拼装(&mut self, 指纹: &str, 拼装: impl FnOnce() -> Result<String, String>) -> Result<String, String> {
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

