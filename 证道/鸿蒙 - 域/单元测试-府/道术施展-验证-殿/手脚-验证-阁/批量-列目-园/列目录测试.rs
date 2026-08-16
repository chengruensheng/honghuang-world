//! 批量-列目-园 · 列目录测试：验证目录列举与排序。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::列举目录;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_列目录_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 列目录取条目() {
        let 目录 = 临时路径("目录");
        std::fs::create_dir_all(&目录).unwrap();
        std::fs::write(目录.join("b.txt"), "b").unwrap();
        std::fs::write(目录.join("a.txt"), "aa").unwrap();

        let 条目们 = 列举目录(目录.to_str().unwrap()).unwrap();
        assert_eq!(条目们.len(), 2);
        assert_eq!(条目们[0].名称, "a.txt");
        assert_eq!(条目们[0].字节数, 2);
        assert_eq!(条目们[1].名称, "b.txt");

        std::fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 列目录不存在报错() {
        let 目录 = 临时路径("不存在目录");
        assert!(列举目录(目录.to_str().unwrap()).is_err());
    }
}
