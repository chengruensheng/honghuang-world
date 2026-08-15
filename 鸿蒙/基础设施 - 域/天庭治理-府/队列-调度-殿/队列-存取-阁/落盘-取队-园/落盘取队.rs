//! 落盘 - 取队 - 园：要求/设计队列的落盘入队、取队与水位，以及八态状态机。

use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use crate::类型_定义_殿::要求状态;

/// 磁盘 jsonl 队列（重启不丢）。
pub struct 落盘队列<T> {
    路径: PathBuf,
    _标记: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> 落盘队列<T> {
    /// 打开（不存在则创建空文件）。
    pub fn 打开(路径: impl AsRef<Path>) -> 落盘队列<T> {
        let 路径 = 路径.as_ref().to_path_buf();
        if let Some(父) = 路径.parent() {
            let _ = fs::create_dir_all(父);
        }
        if !路径.exists() {
            let _ = fs::write(&路径, "");
        }
        落盘队列 { 路径, _标记: PhantomData }
    }

    /// 入队（追加一行 JSON）。
    pub fn 入队(&self, 项: &T) -> Result<(), String> {
        let 行 = serde_json::to_string(项).map_err(|错误| format!("序列化队列项失败: {错误}"))?;
        use std::io::Write;
        let mut 文件 = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.路径)
            .map_err(|错误| format!("打开队列失败: {错误}"))?;
        writeln!(文件, "{行}").map_err(|错误| format!("入队失败: {错误}"))
    }

    /// 取队（读首行并删除）。
    pub fn 取队(&self) -> Result<Option<T>, String> {
        let 内容 = fs::read_to_string(&self.路径).map_err(|错误| format!("读队列失败: {错误}"))?;
        let mut 行们: Vec<&str> = 内容.lines().filter(|行| !行.trim().is_empty()).collect();
        if 行们.is_empty() {
            return Ok(None);
        }
        let 首行 = 行们.remove(0);
        let 剩余 = 行们.join("\n");
        let 剩余 = if 剩余.is_empty() { String::new() } else { format!("{剩余}\n") };
        fs::write(&self.路径, 剩余).map_err(|错误| format!("写队列失败: {错误}"))?;
        let 项 = serde_json::from_str::<T>(首行).map_err(|错误| format!("解析队列项失败: {错误}"))?;
        Ok(Some(项))
    }

    /// 水位（当前行数）。
    pub fn 水位(&self) -> Result<usize, String> {
        let 内容 = fs::read_to_string(&self.路径).map_err(|错误| format!("读队列失败: {错误}"))?;
        Ok(内容.lines().filter(|行| !行.trim().is_empty()).count())
    }

    /// 读全部（不删除，按行解析，供列表/详情用）。
    pub fn 读全部(&self) -> Result<Vec<T>, String> {
        let 内容 = fs::read_to_string(&self.路径).map_err(|错误| format!("读队列失败: {错误}"))?;
        let mut 项们 = Vec::new();
        for 行 in 内容.lines().filter(|行| !行.trim().is_empty()) {
            let 项 = serde_json::from_str::<T>(行).map_err(|错误| format!("解析队列项失败: {错误}"))?;
            项们.push(项);
        }
        Ok(项们)
    }
}

/// 八态合法迁移表。
pub fn 合法迁移(当前: &要求状态) -> Vec<要求状态> {
    match 当前 {
        要求状态::待领 => vec![要求状态::设计中],
        要求状态::设计中 => vec![要求状态::待确认],
        要求状态::待确认 => vec![要求状态::已确认, 要求状态::设计中],
        要求状态::已确认 => vec![要求状态::待实现],
        要求状态::待实现 => vec![要求状态::实现中],
        要求状态::实现中 => vec![要求状态::已验收],
        要求状态::已验收 => vec![要求状态::已存档, 要求状态::待实现],
        要求状态::已存档 => vec![],
    }
}

/// 状态推进：合法则返回目标态，非法则报错。
pub fn 状态推进(当前: &要求状态, 目标: &要求状态) -> Result<要求状态, String> {
    if 合法迁移(当前).contains(目标) {
        Ok(目标.clone())
    } else {
        Err(format!("非法状态推进：从 {:?} 到 {:?}", 当前, 目标))
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct 测试项 { 名: String }

    #[test]
    fn 入队取队水位() {
        let 路径 = std::env::temp_dir().join("识海测试-队列.jsonl");
        let 队列 = 落盘队列::<测试项>::打开(&路径);
        队列.入队(&测试项 { 名: "一".to_string() }).unwrap();
        队列.入队(&测试项 { 名: "二".to_string() }).unwrap();
        assert_eq!(队列.水位().unwrap(), 2);
        let 取 = 队列.取队().unwrap().unwrap();
        assert_eq!(取.名, "一");
        assert_eq!(队列.水位().unwrap(), 1);
        let _ = fs::remove_file(&路径);
    }

    #[test]
    fn 非法迁移被拒() {
        assert!(状态推进(&要求状态::待领, &要求状态::已存档).is_err());
        assert!(状态推进(&要求状态::待确认, &要求状态::设计中).is_ok());
    }
}
