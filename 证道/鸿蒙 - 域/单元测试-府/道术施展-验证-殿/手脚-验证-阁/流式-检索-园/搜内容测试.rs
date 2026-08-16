//! 流式-检索-园 · 搜内容测试：验证目录树下的字面串检索。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::搜索内容;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_搜内容_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 搜内容命中带行号() {
        let 目录 = 临时路径("命中");
        std::fs::create_dir_all(&目录).unwrap();
        std::fs::write(目录.join("a.rs"), "第一行\n目标词\n第三行").unwrap();

        let 命中们 = 搜索内容(目录.to_str().unwrap(), "目标词").unwrap();
        assert_eq!(命中们.len(), 1);
        assert_eq!(命中们[0].行号, 2);
        assert_eq!(命中们[0].行内容, "目标词");

        std::fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 搜内容空串不命中() {
        let 目录 = 临时路径("空串");
        std::fs::create_dir_all(&目录).unwrap();
        assert!(搜索内容(目录.to_str().unwrap(), "").unwrap().is_empty());
        std::fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 搜内容根不存在报错() {
        let 目录 = 临时路径("无此根");
        assert!(搜索内容(目录.to_str().unwrap(), "x").is_err());
    }
}
