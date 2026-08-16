//! 并行-落地-园 · 并行落地测试：验证落地器的控制台/文件开关状态。

#[cfg(test)]
mod 测试 {
    use rizhi_fu::{落地器, 日志去向};

    #[test]
    fn 仅控制台无文件() {
        let 落地器 = 落地器::仅控制台();
        assert!(落地器.写控制台);
        assert!(落地器.文件.is_none());
    }

    #[test]
    fn 新建仅文件去向() {
        let 落地器 = 落地器::新建(&日志去向::仅文件("测试.log".into())).unwrap();
        assert!(!落地器.写控制台);
        assert!(落地器.文件.is_some());
    }
}
