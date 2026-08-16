//! 通配-找档-园 · 找文件测试：验证通配符模式在目录树下找文件。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::寻找文件;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_找文件_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 单段星号找文件() {
        let 目录 = 临时路径("星号");
        std::fs::create_dir_all(&目录).unwrap();
        std::fs::write(目录.join("a.rs"), "").unwrap();
        std::fs::write(目录.join("b.txt"), "").unwrap();

        let 命中 = 寻找文件(目录.to_str().unwrap(), "*.rs").unwrap();
        assert_eq!(命中.len(), 1);
        assert!(命中[0].ends_with("a.rs"));

        std::fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 双星号递归找文件() {
        let 目录 = 临时路径("递归");
        let 子 = 目录.join("子");
        std::fs::create_dir_all(&子).unwrap();
        std::fs::write(子.join("c.rs"), "").unwrap();
        std::fs::write(目录.join("a.rs"), "").unwrap();

        let 命中 = 寻找文件(目录.to_str().unwrap(), "**/*.rs").unwrap();
        assert_eq!(命中.len(), 2);

        std::fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 根不存在报错() {
        let 目录 = 临时路径("无此根");
        assert!(寻找文件(目录.to_str().unwrap(), "*.rs").is_err());
    }
}
