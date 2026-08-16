//! 读取-落配-园 · 读取落配测试：验证环境文件键值行解析。

#[cfg(test)]
mod 测试 {
    use peizhi_fu::解析环境文件;

    #[test]
    fn 解析键值行() {
        let 临时 = std::env::temp_dir().join("识海测试.env");
        std::fs::write(&临时, "LLM_API_KEY=abc\nLLM_MODEL=MiniMax-M3\n# 注释\n").unwrap();
        let 映射 = 解析环境文件(临时.to_str().unwrap());
        assert_eq!(映射.get("LLM_API_KEY").unwrap(), "abc");
        assert_eq!(映射.get("LLM_MODEL").unwrap(), "MiniMax-M3");
        let _ = std::fs::remove_file(&临时);
    }
}
