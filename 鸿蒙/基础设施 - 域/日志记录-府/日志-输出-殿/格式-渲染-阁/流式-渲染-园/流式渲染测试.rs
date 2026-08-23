//! 流式 - 渲染 - 园 · 流式渲染测试：覆盖渲染器构造与实际渲染输出。

#[cfg(test)]
mod 测试 {
    use crate::渲染器;
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

    // ===== 构造分支覆盖 =====

    /// 验证：彩色渲染器可被构造（核心分支一：加色=true）。
    #[test]
    fn 彩色渲染器可被构造() {
        let 渲染 = 渲染器::彩色();
        let _ = 渲染;
    }

    /// 验证：无色渲染器可被构造（核心分支二：加色=false）。
    #[test]
    fn 无色渲染器可被构造() {
        let 渲染 = 渲染器::无色();
        let _ = 渲染;
    }

    /// 验证：彩色与无色渲染器可并存于同一作用域（分支互不干扰）。
    #[test]
    fn 彩色与无色渲染器可并存() {
        let 彩色实例 = 渲染器::彩色();
        let 无色实例 = 渲染器::无色();
        let _ = (彩色实例, 无色实例);
    }

    /// 验证：同一渲染器分支可重复构造无副作用。
    #[test]
    fn 重复构造渲染器无副作用() {
        let _a = 渲染器::彩色();
        let _b = 渲染器::彩色();
        let _c = 渲染器::无色();
        let _d = 渲染器::无色();
    }

    /// 验证：彩色渲染器在函数边界传递后仍可用（值语义）。
    #[test]
    fn 彩色渲染器可跨函数边界传递() {
        fn 制造() -> 渲染器 {
            渲染器::彩色()
        }
        let _渲染 = 制造();
    }

    /// 验证：无色渲染器在函数边界传递后仍可用（值语义）。
    #[test]
    fn 无色渲染器可跨函数边界传递() {
        fn 制造() -> 渲染器 {
            渲染器::无色()
        }
        let _渲染 = 制造();
    }

    // ===== 实际渲染输出覆盖 =====

    /// 验证：无色模式输出不包含 ANSI 转义码。
    #[test]
    fn 无色模式不含转义码() {
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
            !输出.contains("\x1b["),
            "无色模式不应含 ANSI 转义码，实际：{输出}"
        );
    }

    /// 验证：彩色模式可成功触发渲染（不 panic，输出含消息）。
    #[test]
    fn 彩色模式可触发渲染() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::彩色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "彩色测试", "彩色消息");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(输出.contains("彩色消息"), "应包含消息内容，实际：{输出}");
    }

    /// 验证：渲染输出包含短横线分隔的级别与模块。
    #[test]
    fn 渲染包含级别与模块() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::warn!(target: "我的模块", "警告内容");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(
            输出.contains(" - 警告 - 我的模块 - "),
            "应包含分隔格式，实际：{输出}"
        );
    }

    /// 验证：中文消息正确渲染（UTF-8 完整保留）。
    #[test]
    fn 中文消息正确渲染() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "中文模块", "你好世界");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(输出.contains("你好世界"), "应包含中文消息，实际：{输出}");
    }

    // ===== 追加：边界场景覆盖 =====

    /// 验证：错误级别（ERROR）消息可被正确渲染。
    #[test]
    fn 错误级别消息正确渲染() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::error!(target: "错误模块", "严重故障");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(
            输出.contains(" - 错误 - 错误模块 - 严重故障"),
            "实际输出：{输出}"
        );
    }

    /// 验证：调试级别（DEBUG）消息可被正确渲染。
    #[test]
    fn 调试级别消息正确渲染() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::debug!(target: "调试模块", "调试细节");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(
            输出.contains(" - 调试 - 调试模块 - 调试细节"),
            "实际输出：{输出}"
        );
    }

    /// 验证：长消息（80 个汉字）被渲染器完整保留不被截断。
    #[test]
    fn 长消息完整渲染() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        let 长内容 = "句".repeat(80);
        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "长消息模块", "{}", 长内容);
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(输出.contains(&长内容), "长消息应被完整保留");
    }

    /// 验证：同一订阅器连续写入三条消息不丢失。
    #[test]
    fn 连续多次写入不丢失() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "连续模块", "第一条");
            tracing::info!(target: "连续模块", "第二条");
            tracing::info!(target: "连续模块", "第三条");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(输出.contains("第一条"), "第一条丢失");
        assert!(输出.contains("第二条"), "第二条丢失");
        assert!(输出.contains("第三条"), "第三条丢失");
    }

    /// 验证：模块名含横线等特殊字符也能正常渲染。
    #[test]
    fn 含特殊字符模块名正常渲染() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "命名空间-子-模块", "特殊名字");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(
            输出.contains("命名空间-子-模块"),
            "应保留特殊模块名，实际：{输出}"
        );
    }

    // ===== 本次追加：补全级别分支与时间格式覆盖 =====

    /// 验证：追踪级别（TRACE）消息正确渲染——覆盖 级别中文 函数 TRACE 分支。
    #[test]
    fn 追踪级别消息正确渲染() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::trace!(target: "追踪模块", "细节追踪");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(
            输出.contains(" - 追踪 - 追踪模块 - 细节追踪"),
            "TRACE 级别应映射为「追踪」，实际：{输出}"
        );
    }

    /// 验证：渲染输出以换行符结尾（writeln! 调用生效）。
    #[test]
    fn 输出以换行符结尾() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "换行模块", "消息");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(
            输出.ends_with('\n'),
            "输出应以换行符结尾，实际末尾字符：{:?}",
            输出.chars().last()
        );
    }

    /// 验证：时间戳符合 YYYY-MM-DD HH:MM:SS.mmm 格式（23 字符）。
    #[test]
    fn 时间戳符合_iso格式() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "时间模块", "时间测试");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        let 时间戳 = 输出.split(" - ").next().unwrap_or("");
        assert_eq!(时间戳.len(), 23, "时间戳应 23 字符，实际：{时间戳}");
        let 字节 = 时间戳.as_bytes();
        assert_eq!(字节[4], b'-', "第 5 字符应为 -");
        assert_eq!(字节[7], b'-', "第 8 字符应为 -");
        assert_eq!(字节[10], b' ', "第 11 字符应为 空格");
        assert_eq!(字节[13], b':', "第 14 字符应为 :");
        assert_eq!(字节[16], b':', "第 17 字符应为 :");
        assert_eq!(字节[19], b'.', "第 20 字符应为 .");
    }

    /// 验证：空字符串消息不导致渲染器 panic，分隔符仍完整保留。
    #[test]
    fn 空字符串消息正常渲染() {
        let 缓冲 = Arc::new(Mutex::new(Vec::new()));
        let 订阅器 = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .event_format(渲染器::无色())
            .with_writer(捕获器(缓冲.clone()))
            .finish();

        tracing::subscriber::with_default(订阅器, || {
            tracing::info!(target: "空消息模块", "");
        });

        let 输出 = String::from_utf8(缓冲.lock().unwrap().clone()).unwrap();
        assert!(
            输出.contains(" - 信息 - 空消息模块 - \n"),
            "空消息应保留模块名与级别分隔，实际：{输出:?}"
        );
    }
}
