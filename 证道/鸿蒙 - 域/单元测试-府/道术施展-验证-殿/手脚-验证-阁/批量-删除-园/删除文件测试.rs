//! 批量-删除-园 · 删除文件测试：验证批量删除文件，以及删除前备份可撤销。

#[cfg(test)]
mod 测试 {
    use crate::道术施展_验证_殿::手脚_验证_阁::隔离_互斥_园::隔离设施::设施::环境变量锁;
    use daoshu_fu::删文件;
    use shihai_fu::{进入任务, 回滚垫, 工作区};

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_删文件_{}_{}", std::process::id(), 名))
    }

    /// 临时工作区：建目录 + 设 WORLD_WORKSPACE_ROOT，返回 (根, 锁)；持有 guard 至测试结束。
    fn 临时工作区(名: &str) -> (std::path::PathBuf, std::sync::MutexGuard<'static, ()>) {
        let 锁 = 环境变量锁.lock().unwrap_or_else(|e| e.into_inner());
        let 根 = std::env::temp_dir().join(format!(
            "手脚架_删文件_工作区_{}_{}",
            std::process::id(),
            名
        ));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        (根, 锁)
    }

    #[test]
    fn 删文件后不存在() {
        let 路径 = 临时路径("删除.txt");
        std::fs::write(&路径, "x").unwrap();
        let 文本 = 路径.to_str().unwrap().to_string();
        删文件(&[&文本]).unwrap();
        assert!(!路径.exists());
    }

    #[test]
    fn 删不存在的文件报错() {
        let 路径 = 临时路径("已不存在.txt");
        assert!(删文件(&[路径.to_str().unwrap()]).is_err());
    }

    #[test]
    fn 删前备份_撤销恢复文件() {
        let (根, _锁) = 临时工作区("恢复");
        let 甲 = 根.join("甲.rs");
        std::fs::write(&甲, "旧内容").unwrap();
        let _守卫 = 进入任务("任务A");
        删文件(&[甲.to_str().unwrap()]).unwrap();
        assert!(!甲.exists());
        回滚垫::在工作区(&工作区::新(&根)).撤销("任务A").unwrap();
        assert_eq!(
            std::fs::read_to_string(&甲).unwrap(),
            "旧内容",
            "撤销应恢复被删文件"
        );
        let _ = std::fs::remove_dir_all(&根);
    }
}
