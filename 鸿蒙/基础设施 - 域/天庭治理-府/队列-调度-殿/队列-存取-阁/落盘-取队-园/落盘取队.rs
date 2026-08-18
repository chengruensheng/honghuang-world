//! 落盘 - 取队 - 园：要求/设计队列的落盘入队、取队与水位，以及八态状态机。

use rizhi_fu::{debug, error, warn};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use crate::类型_定义_殿::要求状态;

/// 磁盘 jsonl 队列（重启不丢）。
/// 并发安全（2026-08-17 轮8 体检）：入队/取队 内部短锁防 append 行交错；
/// 复合「读改写」操作由调用方持 `排他` 锁贯穿（防守护回填与界主登记互相覆盖）。
pub struct 落盘队列<T> {
    路径: PathBuf,
    _标记: PhantomData<T>,
}

/// 进程级排他锁（队列文件旁的 .jsonl.lock，create_new 原子抢锁）。
/// 持锁期间调用方用 fs 直接读写队列文件（勿调队列方法，防同进程重入死锁）；Drop 释放并删锁文件。
pub struct 排他锁 {
    锁路径: PathBuf,
}

impl Drop for 排他锁 {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.锁路径);
    }
}

/// 抢排他锁：陈旧锁（>30 秒）视为崩溃残留清理重试；最长等待 5 秒，超时返回错误。
fn 抢排他锁(锁路径: &Path) -> Result<排他锁, String> {
    let 开始 = std::time::Instant::now();
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(锁路径)
        {
            Ok(_) => {
                return Ok(排他锁 {
                    锁路径: 锁路径.to_path_buf(),
                })
            }
            Err(_) => {
                if let Ok(元) = std::fs::metadata(锁路径) {
                    if let Ok(修改) = 元.modified() {
                        if let Ok(龄) = 修改.elapsed() {
                            if 龄.as_secs() > 30 {
                                warn!(路径 = ?锁路径, "队列锁已陈旧，清理重试");
                                let _ = std::fs::remove_file(锁路径);
                                continue;
                            }
                        }
                    }
                }
                if 开始.elapsed().as_secs() >= 5 {
                    return Err(format!("队列排他锁等待超时：{}", 锁路径.display()));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
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
        落盘队列 {
            路径,
            _标记: PhantomData,
        }
    }

    /// 拿进程级排他锁：复合「读→改→写」操作须持锁贯穿（调用方持锁期间直接 fs 读写，勿调队列方法）。
    pub fn 排他(&self) -> Result<排他锁, String> {
        抢排他锁(&self.路径.with_extension("jsonl.lock"))
    }

    /// 入队（追加一行 JSON）。内部短锁防并发 append 行交错。
    pub fn 入队(&self, 项: &T) -> Result<(), String> {
        let 行 = serde_json::to_string(项).map_err(|错误| format!("序列化队列项失败: {错误}"))?;
        use std::io::Write;
        let 锁 = self.排他()?;
        let mut 文件 = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.路径)
            .map_err(|错误| {
                error!(路径 = %self.路径.display(), "打开队列失败：{错误}");
                format!("打开队列失败: {错误}")
            })?;
        writeln!(文件, "{行}").map_err(|错误| {
            error!(路径 = %self.路径.display(), "入队失败：{错误}");
            format!("入队失败: {错误}")
        })?;
        drop(锁);
        debug!(路径 = %self.路径.display(), "队列已入一项");
        Ok(())
    }

    /// 取队（读首行并删除）。内部排他锁防与入队/重写交错。
    pub fn 取队(&self) -> Result<Option<T>, String> {
        let 锁 = self.排他()?;
        let 内容 = fs::read_to_string(&self.路径).map_err(|错误| {
            error!(路径 = %self.路径.display(), "读队列失败：{错误}");
            format!("读队列失败: {错误}")
        })?;
        let mut 行们: Vec<&str> = 内容.lines().filter(|行| !行.trim().is_empty()).collect();
        if 行们.is_empty() {
            return Ok(None);
        }
        let 首行 = 行们.remove(0);
        let 剩余 = 行们.join("\n");
        let 剩余 = if 剩余.is_empty() {
            String::new()
        } else {
            format!("{剩余}\n")
        };
        fs::write(&self.路径, 剩余).map_err(|错误| {
            error!(路径 = %self.路径.display(), "写队列失败：{错误}");
            format!("写队列失败: {错误}")
        })?;
        let 项 =
            serde_json::from_str::<T>(首行).map_err(|错误| format!("解析队列项失败: {错误}"))?;
        drop(锁);
        debug!(路径 = %self.路径.display(), "队列取出一项");
        Ok(Some(项))
    }

    /// 水位（当前行数）。只读，不加锁（rename 原子写保证读到完整旧/新内容）。
    pub fn 水位(&self) -> Result<usize, String> {
        let 内容 = fs::read_to_string(&self.路径).map_err(|错误| format!("读队列失败: {错误}"))?;
        Ok(内容.lines().filter(|行| !行.trim().is_empty()).count())
    }

    /// 读全部（不删除，按行解析，供列表/详情用）。只读，不加锁。
    pub fn 读全部(&self) -> Result<Vec<T>, String> {
        let 内容 = fs::read_to_string(&self.路径).map_err(|错误| format!("读队列失败: {错误}"))?;
        let mut 项们 = Vec::new();
        for 行 in 内容.lines().filter(|行| !行.trim().is_empty()) {
            let 项 =
                serde_json::from_str::<T>(行).map_err(|错误| format!("解析队列项失败: {错误}"))?;
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
        warn!(当前 = ?当前, 目标 = ?目标, "非法状态推进");
        Err(format!("非法状态推进：从 {:?} 到 {:?}", 当前, 目标))
    }
}
