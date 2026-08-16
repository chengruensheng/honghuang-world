//! 会话-归档-园 · 测试：会话记录读写与归档。

#[cfg(test)]
mod 测试 {
    use shihai_fu::会话缓存;
    use std::fs;

    #[test]
    fn 写入再读回一致() {
        let 目录 = std::env::temp_dir().join("识海测试-会话");
        let 缓存 = 会话缓存::打开(&目录);
        缓存.写会话("s1", "完整现场").unwrap();
        let 读回 = 缓存.读会话("s1").unwrap().unwrap();
        assert_eq!(读回.内容, "完整现场");
        let _ = fs::remove_dir_all(&目录);
    }
}
