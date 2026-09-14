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
                    // 循环读满：TCP 可能分片，需等请求行+头部+Content-Length 体全部到达；
                    // 若只读一次就回包+关连接，读不全时会触发 RST，令 ureq 偶发报错（测试 flaky 根因）。
                    let mut 缓冲: Vec<u8> = Vec::new();
                    let mut 临时 = [0u8; 4096];
                    loop {
                        let n = 流.read(&mut 临时).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        缓冲.extend_from_slice(&临时[..n]);
                        let 文本 = String::from_utf8_lossy(&缓冲);
                        if let Some(头尾) = 文本.find("\r\n\r\n") {
                            let 头 = &文本[..头尾];
                            let clen = 头
                                .lines()
                                .find_map(|l| {
                                    let 低 = l.to_ascii_lowercase();
                                    低.strip_prefix("content-length:")
                                        .and_then(|v| v.trim().parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            if 缓冲.len() >= 头尾 + 4 + clen {
                                break;
                            }
                        }
                    }
                    let 状态文本 = match 状态码 {
                        200 => "200 OK",
                        429 => "429 Too Many Requests",
                        _ => "500 Internal Server Error",
                    };
                    let 响应 = format!(
                        "HTTP/1.0 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        状态文本,
                        响应体.len(),
                        响应体,
                    );
                    let _ = 流.write_all(响应.as_bytes());
                    let _ = 流.flush();
                    let _ = 流.shutdown(std::net::Shutdown::Both);
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

    /// 回归：网关把 arguments 回成 JSON 对象（规范要求是字符串）时，
    /// 旧实现 `as_str()` 会静默取空串，下游只报「解析工具参数失败: EOF」，看不出真因。
    #[test]
    fn 解析工具调用_arguments为对象时序列化为可解析参数文本() {
        let 消息 = serde_json::json!({
            "tool_calls": [{
                "id": "c1",
                "function": { "name": "run_command", "arguments": { "命令": "dir" } }
            }]
        });
        let 调用 = hm_content::解析工具调用(&消息);
        assert_eq!(调用.len(), 1);
        let 参数: serde_json::Value = serde_json::from_str(&调用[0].参数).expect("参数必须可解析");
        assert_eq!(参数["命令"], "dir", "对象形态 arguments 应序列化为可解析文本，实际: {}", 调用[0].参数);
    }

    /// 回归：arguments 被 markdown 代码围栏包裹时须剥离围栏取正文，否则下游解析必失败
    #[test]
    fn 解析工具调用_参数被代码围栏包裹时剥离取正文() {
        let 消息 = serde_json::json!({
            "tool_calls": [{
                "id": "c2",
                "function": { "name": "list_dir", "arguments": "```json\n{\"路径\":\".\"}\n```" }
            }]
        });
        let 调用 = hm_content::解析工具调用(&消息);
        let 参数: serde_json::Value = serde_json::from_str(&调用[0].参数).expect("剥离围栏后必须可解析");
        assert_eq!(参数["路径"], ".", "应剥离围栏保留正文，实际: {}", 调用[0].参数);
    }

    /// 回归：arguments 为 null 时回退成空对象 `{}`（而非空串），使「列目录」等可缺省工具仍能正常执行
    #[test]
    fn 解析工具调用_arguments为空时回退空对象() {
        let 消息 = serde_json::json!({
            "tool_calls": [{
                "id": "c3",
                "function": { "name": "list_dir", "arguments": serde_json::Value::Null }
            }]
        });
        let 调用 = hm_content::解析工具调用(&消息);
        assert_eq!(调用[0].参数, "{}", "null 参数应回退为可解析的空对象");
    }
}