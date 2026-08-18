//! 增量-改写-园 · 改写文件测试：验证替换首次出现的旧文，以及改写前备份可撤销。
//!
//! WORLD_WORKSPACE_ROOT 等进程级环境变量经隔离-互斥-园 的全局共享锁串行化，
//! 与原子-写入-园 共用同一把锁，避免并行测试互相覆盖。

#[cfg(test)]
mod 测试 {
    use crate::道术施展_验证_殿::手脚_验证_阁::隔离_互斥_园::隔离设施::设施::临时工作区;
    use daoshu_fu::改文件;
    use shihai_fu::{进入任务, 回滚垫, 工作区};

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_改文件_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 改文件替换首处() {
        let 路径 = 临时路径("替换首处.txt");
        std::fs::write(&路径, "甲乙甲").unwrap();
        改文件(路径.to_str().unwrap(), "甲", "丙").unwrap();
        assert_eq!(std::fs::read_to_string(&路径).unwrap(), "丙乙甲");
        std::fs::remove_file(&路径).unwrap();
    }

    #[test]
    fn 改文件找不到旧文报错() {
        let 路径 = 临时路径("找不到.txt");
        std::fs::write(&路径, "内容").unwrap();
        assert!(改文件(路径.to_str().unwrap(), "没有", "x").is_err());
        std::fs::remove_file(&路径).unwrap();
    }

    #[test]
    fn 改前备份_撤销恢复原文() {
        let (根, _锁) = 临时工作区("改文件", "备份");
        let 目标 = 根.join("甲.rs");
        std::fs::write(&目标, "旧文本").unwrap();
        let _守卫 = 进入任务("任务A");
        改文件(目标.to_str().unwrap(), "旧文本", "新文本").unwrap();
        assert_eq!(std::fs::read_to_string(&目标).unwrap(), "新文本");
        回滚垫::在工作区(&工作区::新(&根)).撤销("任务A").unwrap();
        assert_eq!(
            std::fs::read_to_string(&目标).unwrap(),
            "旧文本",
            "撤销应恢复改写前原文"
        );
        let _ = std::fs::remove_dir_all(&根);
        drop(_锁);
    }
}
