//! 流式-渲染-园 · 流式渲染测试：验证日志行渲染格式「时间 - 级别 - 模块 - 消息」。

#[cfg(test)]
mod 测试 {
    use rizhi_fu::渲染器;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::writer::MakeWriter;

    /// 捕获器：把写入内容收集到内存缓冲
    #[derive(Clone)]
    struct 捕获器(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for 捕获器 {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for 捕获器 {
        type Writer = 捕获器;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn 渲染出短横线分隔的一行() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "测试模块", "测试消息");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(
            输出.contains(" - 信息 - 测试模块 - 测试消息"),
            "实际输出：{输出}"
        );
    }
}
