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
    /// 防JSON粘连：文件非空且末尾非换*换行时先补换行（上次写入或外部修改可能丢末尾换行，
    /// 2026-08-19 实测：手动修改任务线.jsonl后末尾换行丢失，新入队的JSON粘在旧JSON后面，
    /// 逐行解析读不到新任务）。
    pub fn 入队(&self, 项: &T) -> Result<(), String> {
        let 行 = serde_json::to_string(项).map_err(|错误| format!("序列化队列项失败: {错误}"))?;
        use std::io::Write;
        let 锁 = self.排他()?;
        if let Ok(已有) = fs::read_to_string(&self.路径) {
            if !已有.is_empty() && !已有.ends_with('\n') {
                let mut 补 = fs::OpenOptions::new()
                    .append(true)
                    .open(&self.路径)
                    .map_err(|错误| format!("打开补换行失败: {错误}"))?;
                补.write_all(b"\n")
                    .map_err(|错误| format!("补换行失败: {错误}"))?;
            }
        }
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
        要求状态::待实现 => vec![要求状态::实现中, 要求状态::已归档],
        要求状态::实现中 => vec![要求状态::已验收, 要求状态::已归档],
        要求状态::已验收 => vec![要求状态::已存档, 要求状态::待实现],
        要求状态::已存档 => vec![],
        要求状态::已归档 => vec![],
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

#[cfg(test)]
mod 队列边界测试 {
    //! 落盘 - 取队 - 园 · 队列边界测试：边界场景下入队对文件末尾无换行的容错。
    //!
    //! 测试隔离（2026-08-19）：进程级 `static Mutex<()>` 串行化 + 临时路径用
    //! `process::id()` 命名，并行 cargo test 不再因 `std::env::temp_dir()` 残留
    //! 导致水位断言假阴（照 `落盘取队测试.rs` 模式）。

    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::sync::Mutex;

    /// 本 crate 测试进程级互斥锁：并行测试下临时路径不互相残留。
    static 测试环境锁: Mutex<()> = Mutex::new(());

    /// 造临时路径（用 process::id 隔离并行测试）。
    fn 建临时路径(标签: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "识海测试-队列边界-{}-{}-{}",
            标签,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct 测试项 {
        名: String,
    }

    /// 边界（2026-08-19 整治）：外部改写或上次写入异常可能丢末尾换行，
    /// 入队须自动补换行并与旧 JSON 隔开，避免逐行解析粘连失败。
    #[test]
    fn 末尾无换行_入队不粘连() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 路径 = 建临时路径("末尾无换行");

        // 旧项原文（无尾随换行）
        let 旧原文 = serde_json::to_string(&测试项 {
            名: "一".to_string(),
        })
        .unwrap();
        fs::write(&路径, &旧原文).unwrap();

        // 前置：文件非空且不以换行结尾
        let 前置 = fs::read_to_string(&路径).unwrap();
        assert!(!前置.is_empty());
        assert!(!前置.ends_with('\n'), "前置文件必须故意缺末尾换行");

        let 队列 = super::落盘队列::<测试项>::打开(&路径);
        队列
            .入队(&测试项 {
                名: "二".to_string(),
            })
            .unwrap();

        // 字节级断言：旧 JSON 字符末尾与新 JSON 首字符之间须出现换行分隔
        let 后置 = fs::read_to_string(&路径).unwrap();
        assert!(
            后置.contains(&format!("{旧原文}\n")),
            "旧项后必须紧跟换行分隔，实际字节: {后置:?}"
        );
        // 逐行解析不得把两行合并为一段
        let 行们: Vec<&str> = 后置.lines().filter(|行| !行.trim().is_empty()).collect();
        assert_eq!(行们.len(), 2, "逐行解析应得两条独立 JSON，实际: {后置:?}");
        assert_eq!(行们[0], 旧原文, "旧项必须原样保留");
        let 新原文 = serde_json::to_string(&测试项 {
            名: "二".to_string(),
        })
        .unwrap();
        assert_eq!(行们[1], 新原文);
        // 入队后文件仍以换行收尾，为下一轮入队提供干净基线
        assert!(
            后置.ends_with('\n'),
            "入队后末尾必须换行收尾，避免下次再粘连"
        );
        // 读全部 + 水位
        let 全 = 队列.读全部().unwrap();
        assert_eq!(全.len(), 2);
        assert_eq!(全[0].名, "一");
        assert_eq!(全[1].名, "二");
        assert_eq!(队列.水位().unwrap(), 2);

        let _ = fs::remove_file(&路径);
    }

    /// 边界：空文件入队后第一条 JSON 必须自带换行结尾，
    /// 为下一轮入队提供干净基线，不得被无换行基线误判。
    #[test]
    fn 空文件_入队_末尾以换行收尾() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 路径 = 建临时路径("空文件");

        let 队列 = super::落盘队列::<测试项>::打开(&路径);
        队列
            .入队(&测试项 {
                名: "孤".to_string(),
            })
            .unwrap();

        let 内容 = fs::read_to_string(&路径).unwrap();
        assert!(内容.ends_with('\n'), "空文件入队后末尾必须换行收尾");
        assert_eq!(队列.水位().unwrap(), 1);
        let 全 = 队列.读全部().unwrap();
        assert_eq!(全.len(), 1);
        assert_eq!(全[0].名, "孤");

        let _ = fs::remove_file(&路径);
    }
}
