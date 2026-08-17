//! 快照 - 落库 - 园 · 快照落库测试：源码快照排除构建物 + 回退演练。

#[cfg(test)]
mod 测试 {
    use crate::道术施展_验证_殿::手脚_验证_阁::隔离_互斥_园::隔离设施::设施::临时工作区;
    use std::fs;
    use tianting_fu::{回退版本, 源码快照};

    #[test]
    fn 快照排除构建物() {
        let 源 = std::env::temp_dir().join("识海测试-快照源");
        let 目标 = std::env::temp_dir().join("识海测试-快照目标");
        fs::create_dir_all(源.join("target")).unwrap();
        fs::write(源.join("a.rs"), "x").unwrap();
        fs::write(源.join("target/b.rs"), "y").unwrap();
        let 数 = 源码快照(&源, &目标).unwrap();
        assert_eq!(数, 1); // 只复制 a.rs，跳过 target
        let _ = fs::remove_dir_all(&源);
        let _ = fs::remove_dir_all(&目标);
    }

    /// 回退演练（生产化 4.3）：快照目录 → 目标目录整体回退，旧内容被快照内容覆盖，
    /// 构建物/target 不入快照不回退（防回退把构建物当源码写回）。
    /// 持有隔离锁：构建产物目录识别读 env 工作区根（.cargo/config.toml），并行测试 env 污染会时好时坏。
    #[test]
    fn 回退演练_快照覆盖目标且排除构建物() {
        let (根, _锁) = 临时工作区("离线主链路", "回退");
        let 快照 = 根.join("版本-v2").join("源码-快照");
        let 目标 = 根.join("工作区");
        // 快照：源码 v2 版 + 构建物目录（应被排除）。
        fs::create_dir_all(快照.join("鸿蒙")).unwrap();
        fs::create_dir_all(快照.join("道果树").join("构建物-域")).unwrap();
        fs::write(快照.join("鸿蒙").join("甲.rs"), "v2 源码").unwrap();
        fs::write(快照.join("道果树").join("构建物-域").join("旧.exe"), "旧产物").unwrap();
        // 目标：旧版源码 + 残留构建物。
        fs::create_dir_all(目标.join("鸿蒙")).unwrap();
        fs::create_dir_all(目标.join("道果树")).unwrap();
        fs::write(目标.join("鸿蒙").join("甲.rs"), "v1 旧源码").unwrap();
        fs::write(目标.join("道果树").join("残留.txt"), "残留").unwrap();

        let 数 = 回退版本(&快照, &目标).unwrap();
        assert_eq!(数, 1, "只回退源码 1 件（构建物被排除）");
        assert_eq!(fs::read_to_string(目标.join("鸿蒙").join("甲.rs")).unwrap(), "v2 源码", "快照内容应覆盖目标");
        assert!(!目标.join("道果树").join("残留.txt").exists(), "目标旧残留应被清空");
        assert!(!目标.join("道果树").join("构建物-域").exists(), "快照构建物不回退");
        let _ = fs::remove_dir_all(&根);
    }
}
