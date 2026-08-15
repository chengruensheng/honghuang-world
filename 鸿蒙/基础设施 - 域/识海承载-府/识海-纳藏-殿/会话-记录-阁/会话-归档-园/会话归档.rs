//! 会话 - 归档 - 园：会话记录读写与归档。

use crate::{会话记录, 当前毫秒, 记录, 模型存储};
use std::fs;
use std::path::{Path, PathBuf};

/// 会话缓存：一次任务执行的完整工作记忆。
pub struct 会话缓存 {
    目录: PathBuf,
}

impl 会话缓存 {
    /// 打开（目录不存在则创建）。
    pub fn 打开(目录: impl AsRef<Path>) -> 会话缓存 {
        let 目录 = 目录.as_ref().to_path_buf();
        let _ = fs::create_dir_all(&目录);
        会话缓存 { 目录 }
    }

    /// 在工作区根下打开（会话落 .上下文/会话/）。
    pub fn 在工作区(工作区: &crate::工作区) -> 会话缓存 {
        会话缓存::打开(工作区.会话目录())
    }

    /// 写一份会话记录（完整覆盖）。
    pub fn 写会话(&self, 会话id: &str, 内容: &str) -> Result<(), String> {
        let 记录 = 会话记录 {
            会话id: 会话id.to_string(),
            内容: 内容.to_string(),
            时间戳: 当前毫秒(),
        };
        let 文本 = serde_json::to_string(&记录).map_err(|错误| format!("序列化会话失败: {错误}"))?;
        fs::write(self.路径(会话id), 文本).map_err(|错误| format!("写会话失败: {错误}"))
    }

    /// 读一份会话记录。
    pub fn 读会话(&self, 会话id: &str) -> Result<Option<会话记录>, String> {
        let 路径 = self.路径(会话id);
        if !路径.exists() {
            return Ok(None);
        }
        let 文本 = fs::read_to_string(&路径).map_err(|错误| format!("读会话失败: {错误}"))?;
        let 记录 = serde_json::from_str::<会话记录>(&文本).map_err(|错误| format!("解析会话失败: {错误}"))?;
        Ok(Some(记录))
    }

    /// 归档一份会话：写入「事件」格位（经历记忆），再移动文件到归档目录。
    pub fn 归档会话(&self, 会话id: &str, 归档目录: &Path, 存储: &模型存储) -> Result<(), String> {
        if let Some(会话) = self.读会话(会话id)? {
            存储.写记录(&记录::新("事件", &会话.内容, &format!("会话「{会话id}」归档"), "代码"))?;
        }
        let 源 = self.路径(会话id);
        if !源.exists() {
            return Ok(());
        }
        let _ = fs::create_dir_all(归档目录);
        let 目标 = 归档目录.join(format!("{会话id}.json"));
        fs::rename(&源, &目标).map_err(|错误| format!("归档会话失败: {错误}"))
    }

    fn 路径(&self, 会话id: &str) -> PathBuf {
        self.目录.join(format!("{会话id}.json"))
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 写入再读回一致() {
        let 目录 = std::env::temp_dir().join("识海测试-会话");
        let 缓存 = 会话缓存::打开(&目录);
        缓存.写会话("s1", "完整现场").unwrap();
        let 读回 = 缓存.读会话("s1").unwrap().unwrap();
        assert_eq!(读回.内容, "完整现场");
        let _ = fs::remove_dir_all(&目录);
    }
}
