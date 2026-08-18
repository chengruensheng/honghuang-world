//! 会话 - 归档 - 园：会话记录读写与归档。

use crate::{会话记录, 当前毫秒, 模型存储, 记录};
use rizhi_fu::{debug, error};
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
        let 文本 =
            serde_json::to_string(&记录).map_err(|错误| format!("序列化会话失败: {错误}"))?;
        fs::write(self.路径(会话id), 文本).map_err(|错误| {
            error!(会话id, "写会话失败：{错误}");
            format!("写会话失败: {错误}")
        })?;
        debug!(会话id, "会话已写入");
        Ok(())
    }

    /// 读一份会话记录。
    pub fn 读会话(&self, 会话id: &str) -> Result<Option<会话记录>, String> {
        let 路径 = self.路径(会话id);
        if !路径.exists() {
            return Ok(None);
        }
        let 文本 = fs::read_to_string(&路径).map_err(|错误| format!("读会话失败: {错误}"))?;
        let 记录 = serde_json::from_str::<会话记录>(&文本)
            .map_err(|错误| format!("解析会话失败: {错误}"))?;
        Ok(Some(记录))
    }

    /// 归档一份会话：写入「事件」格位（经历记忆），再移动文件到归档目录。
    pub fn 归档会话(
        &self,
        会话id: &str,
        归档目录: &Path,
        存储: &模型存储,
    ) -> Result<(), String> {
        if let Some(会话) = self.读会话(会话id)? {
            存储.写记录(&记录::新(
                "事件",
                &会话.内容,
                &format!("会话「{会话id}」归档"),
                "代码",
            ))?;
        }
        let 源 = self.路径(会话id);
        if !源.exists() {
            return Ok(());
        }
        let _ = fs::create_dir_all(归档目录);
        let 目标 = 归档目录.join(format!("{会话id}.json"));
        fs::rename(&源, &目标).map_err(|错误| {
            error!(会话id, "归档会话失败：{错误}");
            format!("归档会话失败: {错误}")
        })?;
        debug!(会话id, "会话已归档");
        Ok(())
    }

    fn 路径(&self, 会话id: &str) -> PathBuf {
        self.目录.join(format!("{会话id}.json"))
    }
}

#[cfg(test)]
mod 测试 {
    use super::会话缓存;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 临时目录 RAII 守卫：离开作用域自动删除目录。
    /// 替代 tempfile::TempDir，避免引入外部依赖。
    struct 临时目录守卫 {
        路径: PathBuf,
    }

    impl 临时目录守卫 {
        fn 新建(前缀: &str) -> Self {
            // 用时间戳 + 计数器确保唯一性，避免并发或重试冲突
            static 计数: AtomicU64 = AtomicU64::new(0);
            let 序号 = 计数.fetch_add(1, Ordering::Relaxed);
            let 毫秒 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let 路径 = std::env::temp_dir().join(format!("{前缀}_{毫秒}_{序号}"));
            fs::create_dir_all(&路径).expect("创建临时目录失败");
            临时目录守卫 { 路径 }
        }

        fn 路径(&self) -> &Path {
            &self.路径
        }
    }

    impl Drop for 临时目录守卫 {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.路径);
        }
    }

    #[test]
    fn 写入再读回一致() {
        let 目录 = std::env::temp_dir().join("识海测试-会话");
        let _ = fs::remove_dir_all(&目录);
        let 缓存 = 会话缓存::打开(&目录);
        缓存.写会话("s1", "完整现场").unwrap();
        let 读回 = 缓存.读会话("s1").unwrap().unwrap();
        assert_eq!(读回.内容, "完整现场");
        let _ = fs::remove_dir_all(&目录);
    }

    #[test]
    fn 临时目录_隔离与自动清理() {
        // 一、隔离性：两个独立临时目录互不干扰
        let 守卫1 = 临时目录守卫::新建("识海测试-隔离A");
        let 守卫2 = 临时目录守卫::新建("识海测试-隔离B");
        assert_ne!(守卫1.路径(), 守卫2.路径(), "两次新建应得到不同路径");

        let 缓存1 = 会话缓存::打开(守卫1.路径());
        let 缓存2 = 会话缓存::打开(守卫2.路径());

        缓存1.写会话("仅在一号", "内容一").unwrap();
        缓存2.写会话("仅在二号", "内容二").unwrap();

        // 隔离断言：一号缓存看不到二号会话，反之亦然
        assert!(
            缓存1.读会话("仅在二号").unwrap().is_none(),
            "一号目录不应读到二号会话"
        );
        assert!(
            缓存2.读会话("仅在一号").unwrap().is_none(),
            "二号目录不应读到一号会话"
        );

        // 各自读回自身写入
        assert_eq!(缓存1.读会话("仅在一号").unwrap().unwrap().内容, "内容一");
        assert_eq!(缓存2.读会话("仅在二号").unwrap().unwrap().内容, "内容二");

        // 二、自动清理（守卫存活时目录应存在）
        assert!(守卫1.路径().exists(), "守卫存活时目录应存在");
        assert!(守卫2.路径().exists(), "守卫存活时目录应存在");

        // 显式 Drop 触发 RAII 自动清理
        let 路径1 = 守卫1.路径().to_path_buf();
        let 路径2 = 守卫2.路径().to_path_buf();
        drop(守卫1);
        drop(守卫2);

        // 自动清理断言：守卫 Drop 后目录应被删除
        assert!(
            !路径1.exists(),
            "临时目录1 未被自动清理：{}",
            路径1.display()
        );
        assert!(
            !路径2.exists(),
            "临时目录2 未被自动清理：{}",
            路径2.display()
        );
    }

    /// 写入一条会话记录后能读回，且 `内容` 与 `时间戳` 字段比对通过。
    #[test]
    fn test_write_read_roundtrip() {
        let 守卫 = 临时目录守卫::新建("识海测试-会话-roundtrip");

        let 缓存 = 会话缓存::打开(守卫.路径());
        缓存.写会话("s_round", "完整现场-roundtrip").unwrap();

        let 读回 = 缓存.读会话("s_round").unwrap().unwrap();
        assert_eq!(读回.会话id, "s_round");
        assert_eq!(读回.内容, "完整现场-roundtrip");
        assert!(读回.时间戳 > 0, "时间戳应为正毫秒，当前: {}", 读回.时间戳);
    }

    /// 归档操作正确写入事件格位（"事件" jsonl 中应含本次会话内容），且源文件被搬入归档目录。
    #[test]
    fn test_archive_event_grid() {
        let 缓存守卫 = 临时目录守卫::新建("识海测试-会话-归档");
        let 存储守卫 = 临时目录守卫::新建("识海测试-存储-归档");
        let 归档守卫 = 临时目录守卫::新建("识海测试-归档目标");

        let 缓存 = 会话缓存::打开(缓存守卫.路径());
        缓存.写会话("s_arch", "待归档内容").unwrap();

        // 归档会话需要模型存储：在存储守卫目录下打开
        let 存储 = crate::模型存储::打开(存储守卫.路径());
        缓存.归档会话("s_arch", 归档守卫.路径(), &存储).unwrap();

        // 归档后源文件应被搬入归档目录
        assert!(
            归档守卫.路径().join("s_arch.json").exists(),
            "归档后源文件应搬入归档目录"
        );

        // 事件格位应至少含一条本次归档记录
        let 事件链头 = 存储.读格位("事件").unwrap();
        assert!(!事件链头.is_empty(), "事件格位应至少含一条记录");
        let 含本次 = 事件链头
            .iter()
            .any(|记录| 记录.格位名 == "事件" && 记录.内容.contains("待归档内容"));
        assert!(含本次, "事件格位应含本次归档内容");
    }
}
