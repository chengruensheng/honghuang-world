//! 依赖 - 边 - 园：依赖图落盘 / 加载。

use crate::{依赖图, 工作区};
use rizhi_fu::debug;
use std::path::Path;

impl 依赖图 {
    /// 保存在工作区（.上下文/依赖图.json）。
    pub fn 保存在工作区(&self, 工作区: &工作区) -> Result<(), String> {
        self.保存(工作区.依赖图路径())
    }

    /// 保存到指定路径。
    pub fn 保存(&self, 路径: impl AsRef<Path>) -> Result<(), String> {
        let 文本 = serde_json::to_string_pretty(self).map_err(|错误| format!("序列化依赖图失败: {错误}"))?;
        debug!(路径 = %路径.as_ref().display(), "依赖图已保存");
        std::fs::write(路径.as_ref(), 文本).map_err(|错误| format!("保存依赖图失败: {错误}"))
    }

    /// 从工作区加载（.上下文/依赖图.json）。
    pub fn 加载自工作区(工作区: &工作区) -> Result<依赖图, String> {
        Self::加载(工作区.依赖图路径())
    }

    /// 从指定路径加载（不存在则返回空图）。
    pub fn 加载(路径: impl AsRef<Path>) -> Result<依赖图, String> {
        let 路径 = 路径.as_ref();
        if !路径.exists() {
            return Ok(依赖图::default());
        }
        let 文本 = std::fs::read_to_string(路径).map_err(|错误| format!("读取依赖图失败: {错误}"))?;
        serde_json::from_str(&文本).map_err(|错误| format!("解析依赖图失败: {错误}"))
    }
}
