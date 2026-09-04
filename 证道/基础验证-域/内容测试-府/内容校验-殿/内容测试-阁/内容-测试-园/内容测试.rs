#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use hm_content_contract::内容生成器;
    use hm_contract::Component;
    use hm_error::Result;

    /// mock 内容生成器：验证契约可插拔，返回预设内容并记录调用次数
    struct 模拟内容生成器 {
        内容: String,
        调用次数: AtomicU64,
    }

    impl 模拟内容生成器 {
        fn 新(内容: &str) -> Self {
            模拟内容生成器 { 内容: 内容.to_string(), 调用次数: AtomicU64::new(0) }
        }
    }

    impl Component for 模拟内容生成器 {
        fn name(&self) -> &'static str { "模拟内容生成器" }
    }

    impl 内容生成器 for 模拟内容生成器 {
        fn 生成(&self, _提示词: String) -> Result<String> {
            self.调用次数.fetch_add(1, Ordering::SeqCst);
            Ok(self.内容.clone())
        }
    }

    #[test]
    fn mock生成器返回预设内容() {
        let 生成器 = 模拟内容生成器::新("预设内容");
        assert_eq!(生成器.生成("提示".into()).unwrap(), "预设内容");
    }

    #[test]
    fn mock生成器记录调用次数() {
        let 生成器 = 模拟内容生成器::新("内容");
        生成器.生成("一".into()).unwrap();
        生成器.生成("二".into()).unwrap();
        assert_eq!(生成器.调用次数.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn 契约可插拔_通过dyn分发() {
        let 生成器: &dyn 内容生成器 = &模拟内容生成器::新("可插拔");
        assert_eq!(生成器.生成("任意".into()).unwrap(), "可插拔");
        assert_eq!(生成器.name(), "模拟内容生成器");
    }
}