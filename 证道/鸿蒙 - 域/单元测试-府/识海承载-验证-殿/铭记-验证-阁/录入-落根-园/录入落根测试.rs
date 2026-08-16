//! 录入-落根-园 · 测试：人类录入根记录（经格位校验）。

#[cfg(test)]
mod 测试 {
    use shihai_fu::{录入根, 模型存储};

    #[test]
    fn 经格位可落根() {
        let 存储 = 模型存储::打开(std::env::temp_dir());
        assert!(录入根(&存储, "铁律·总纲", "不可破的约束", "人").is_ok());
    }

    #[test]
    fn 权格位不可落根() {
        let 存储 = 模型存储::打开(std::env::temp_dir());
        assert!(录入根(&存储, "任务", "内容", "人").is_err());
    }
}
