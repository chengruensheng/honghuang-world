//! 格位-清单-园 · 测试：36 格位 + 72 格位定义。

#[cfg(test)]
mod 测试 {
    use shihai_fu::{全部格位, 全部格位72};

    #[test]
    fn 格位数为三十六() {
        assert_eq!(全部格位().len(), 36);
    }

    #[test]
    fn 格位数为七十二() {
        assert_eq!(全部格位72().len(), 72);
    }
}
