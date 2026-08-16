//! 原子-读取-园 · 读取文件测试：验证读文件全部文本。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::读文件;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_读文件_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 读文件取全部文本() {
        let 路径 = 临时路径("取全部.txt");
        std::fs::write(&路径, "第一行\n第二行").unwrap();
        assert_eq!(读文件(路径.to_str().unwrap()).unwrap(), "第一行\n第二行");
        std::fs::remove_file(&路径).unwrap();
    }

    #[test]
    fn 读文件不存在报错() {
        let 路径 = 临时路径("不存在.txt");
        assert!(读文件(路径.to_str().unwrap()).is_err());
    }
}
