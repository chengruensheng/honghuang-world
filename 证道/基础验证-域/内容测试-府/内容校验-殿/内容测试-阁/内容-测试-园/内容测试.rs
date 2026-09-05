#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use hm_content::对话生成器;
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

    /// 极简本地 HTTP 服务器：返回固定状态码与 JSON 体，供对话生成器降级测试使用
    fn 起模拟模型服务器(状态码: u16, 响应体: &'static str) -> std::net::SocketAddr {
        let 监听 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let 地址 = 监听.local_addr().unwrap();
        std::thread::spawn(move || {
            for 连接 in 监听.incoming() {
                if let Ok(mut 流) = 连接 {
                    use std::io::{Read, Write};
                    let mut 缓冲 = [0u8; 4096];
                    let _ = 流.read(&mut 缓冲);
                    let 状态文本 = match 状态码 {
                        200 => "200 OK",
                        429 => "429 Too Many Requests",
                        _ => "500 Internal Server Error",
                    };
                    let 响应 = format!(
                        "HTTP/1.0 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        状态文本,
                        响应体.len(),
                        响应体,
                    );
                    let _ = 流.write_all(响应.as_bytes());
                    let _ = 流.flush();
                }
            }
        });
        地址
    }

    #[test]
    fn 主模型限流时自动降级备选() {
        let 主地址 = 起模拟模型服务器(429, "{\"error\":\"rate limited\"}");
        let 备地址 = 起模拟模型服务器(200, "{\"choices\":[{\"message\":{\"content\":\"备选答复\"}}]}");
        let 生成器 = 对话生成器::新带备选(
            "main-key".into(), format!("http://{主地址}"), "主模型".into(),
            "fallback-key".into(), format!("http://{备地址}"), "备模型".into(),
        );
        let 结果 = 生成器.生成("你好".into()).unwrap();
        assert_eq!(结果, "备选答复");
    }

    #[test]
    fn 主模型正常时不降级() {
        let 主地址 = 起模拟模型服务器(200, "{\"choices\":[{\"message\":{\"content\":\"主答复\"}}]}");
        let 备地址 = 起模拟模型服务器(200, "{\"choices\":[{\"message\":{\"content\":\"备答复\"}}]}");
        let 生成器 = 对话生成器::新带备选(
            "main-key".into(), format!("http://{主地址}"), "主模型".into(),
            "fallback-key".into(), format!("http://{备地址}"), "备模型".into(),
        );
        let 结果 = 生成器.生成("你好".into()).unwrap();
        assert_eq!(结果, "主答复");
    }
}