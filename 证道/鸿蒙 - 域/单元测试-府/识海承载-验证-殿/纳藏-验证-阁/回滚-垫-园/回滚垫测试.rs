//! 回滚-垫-园 · 回滚垫测试：验证写前存档、失败撤销、成功清理、任务隔离。
//! 仅经 shihai_fu 根 pub 符号（回滚垫/工作区/进入任务/当前任务）测试。

#[cfg(test)]
mod 测试 {
    use shihai_fu::{当前任务, 进入任务, 回滚垫, 工作区};
    use std::fs;

    /// 临时根：回滚垫 + 一组文件，测试结束清理。
    fn 临时根(名: &str) -> std::path::PathBuf {
        let 目录 = std::env::temp_dir().join(format!("证道_回滚垫_{名}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&目录);
        fs::create_dir_all(&目录).unwrap();
        目录
    }

    fn 垫(根: &std::path::Path) -> 回滚垫 {
        回滚垫::在工作区(&工作区::新(根))
    }

    #[test]
    fn 备份撤销_恢复旧内容() {
        let 根 = 临时根("恢复");
        let 目标 = 根.join("甲.rs");
        fs::write(&目标, "旧内容").unwrap();
        垫(&根).备份("任务A", 目标.to_str().unwrap()).unwrap();
        fs::write(&目标, "新内容").unwrap();
        assert_eq!(垫(&根).撤销("任务A").unwrap(), 1);
        assert_eq!(fs::read_to_string(&目标).unwrap(), "旧内容");
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 备份撤销_曾不存在则删除() {
        let 根 = 临时根("删除");
        let 目标 = 根.join("新建.rs");
        垫(&根).备份("任务A", 目标.to_str().unwrap()).unwrap();
        fs::write(&目标, "内容").unwrap();
        assert_eq!(垫(&根).撤销("任务A").unwrap(), 1);
        assert!(!目标.exists());
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 同路径只备份首次() {
        let 根 = 临时根("首次");
        let 目标 = 根.join("甲.rs");
        fs::write(&目标, "原始").unwrap();
        垫(&根).备份("任务A", 目标.to_str().unwrap()).unwrap();
        fs::write(&目标, "第一版").unwrap();
        垫(&根).备份("任务A", 目标.to_str().unwrap()).unwrap();
        fs::write(&目标, "第二版").unwrap();
        垫(&根).撤销("任务A").unwrap();
        assert_eq!(fs::read_to_string(&目标).unwrap(), "原始", "只恢复首次备份的原始状态");
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 任务隔离_互不影响() {
        let 根 = 临时根("隔离");
        let 甲 = 根.join("甲.rs");
        let 乙 = 根.join("乙.rs");
        fs::write(&甲, "甲旧").unwrap();
        fs::write(&乙, "乙旧").unwrap();
        垫(&根).备份("任务A", 甲.to_str().unwrap()).unwrap();
        垫(&根).备份("任务B", 乙.to_str().unwrap()).unwrap();
        fs::write(&甲, "甲新").unwrap();
        fs::write(&乙, "乙新").unwrap();
        垫(&根).撤销("任务A").unwrap();
        assert_eq!(fs::read_to_string(&甲).unwrap(), "甲旧");
        assert_eq!(fs::read_to_string(&乙).unwrap(), "乙新", "任务B 的存档不受影响");
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 当前任务_线程本地() {
        let 守卫 = 进入任务("任务A");
        assert_eq!(当前任务(), "任务A");
        drop(守卫);
        assert_eq!(当前任务(), "");
    }

    #[test]
    fn 清理_丢弃存档() {
        let 根 = 临时根("清理");
        let 目标 = 根.join("甲.rs");
        fs::write(&目标, "内容").unwrap();
        垫(&根).备份("任务A", 目标.to_str().unwrap()).unwrap();
        垫(&根).清理("任务A").unwrap();
        assert!(!根.join(".上下文").join("回滚垫").join("任务A").exists());
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 工作区外路径不归档() {
        let 根 = 临时根("外路径");
        let 外部 = std::env::temp_dir().join(format!("证道_回滚垫_外部_{}.txt", std::process::id()));
        fs::write(&外部, "外部").unwrap();
        垫(&根).备份("任务A", 外部.to_str().unwrap()).unwrap();
        垫(&根).撤销("任务A").unwrap();
        assert_eq!(fs::read_to_string(&外部).unwrap(), "外部", "工作区外文件不应被归档或撤销");
        let _ = fs::remove_file(&外部);
        let _ = fs::remove_dir_all(&根);
    }
}
