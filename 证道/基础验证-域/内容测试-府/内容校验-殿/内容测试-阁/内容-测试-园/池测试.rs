#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicU64, Ordering};

    use hm_config::{LlmConfig, LlmProvider};
    use hm_content::LLM池;
    use hm_content_contract::内容生成器;

    /// 可记录的模拟 LLM 服务器：POST /chat 与 GET /models 按路径分响应，记录全部请求原文
    struct 模拟模型服务器 {
        地址: std::net::SocketAddr,
        记录: Arc<Mutex<Vec<String>>>,
    }

    fn 起模型服务器(对话状态: u16, 对话响应: &str, 列表状态: u16, 列表响应: &str) -> 模拟模型服务器 {
        let 监听 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let 地址 = 监听.local_addr().unwrap();
        let 记录: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let 记录2 = 记录.clone();
        let 对话响应 = 对话响应.to_string();
        let 列表响应 = 列表响应.to_string();
        std::thread::spawn(move || {
            for 连接 in 监听.incoming() {
                if let Ok(mut 流) = 连接 {
                    // 循环读满：TCP 可能分片，需等请求行+头部+Content-Length 体全部到达
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
                    let 请求 = String::from_utf8_lossy(&缓冲).to_string();
                    记录2.lock().unwrap().push(请求.clone());
                    let 请求行 = 请求.lines().next().unwrap_or("");
                    let (状态, 响应体) = if 请求行.starts_with("GET") {
                        (列表状态, 列表响应.clone())
                    } else {
                        (对话状态, 对话响应.clone())
                    };
                    let 状态文本 = match 状态 {
                        200 => "200 OK",
                        429 => "429 Too Many Requests",
                        500 => "500 Internal Server Error",
                        _ => "200 OK",
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
        模拟模型服务器 { 地址, 记录 }
    }

    fn 供应商(名: &str, 地址: &std::net::SocketAddr, 密钥: &str, 模型: &str, 启用: bool) -> LlmProvider {
        LlmProvider {
            name: 名.into(),
            kind: "openai".into(),
            base_url: format!("http://{地址}/chat"),
            models_url: format!("http://{地址}/models"),
            api_key: 密钥.into(),
            model: 模型.into(),
            timeout_secs: 5,
            retry: 0,
            enabled: 启用,
        }
    }

    static 池序号: AtomicU64 = AtomicU64::new(0);

    /// 独立状态文件路径（避免并行测试互踩）
    fn 临时状态文件() -> String {
        std::env::temp_dir()
            .join(format!("洪荒池选择_{}.json", 池序号.fetch_add(1, Ordering::SeqCst)))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn 池_从配置构造默认选第一个可用() {
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &"127.0.0.1:1".parse().unwrap(), "key-a", "甲模型", true),
                供应商("乙", &"127.0.0.1:1".parse().unwrap(), "key-b", "乙模型", true),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        assert!(池.可用());
        let 选择 = 池.当前选择().unwrap();
        assert_eq!(选择.供应商, "甲");
        assert_eq!(选择.模型, "甲模型");
    }

    #[test]
    fn 池_选择校验存在且生效() {
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &"127.0.0.1:1".parse().unwrap(), "key-a", "甲模型", true),
                供应商("乙", &"127.0.0.1:1".parse().unwrap(), "key-b", "乙模型", true),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        池.选择("乙", "乙模型").unwrap();
        let 选择 = 池.当前选择().unwrap();
        assert_eq!(选择.供应商, "乙");
        assert_eq!(选择.模型, "乙模型");
        // 非法供应商：报错且选择不变
        assert!(池.选择("不存在", "某模型").is_err());
        let 选择2 = 池.当前选择().unwrap();
        assert_eq!(选择2.供应商, "乙");
    }

    #[test]
    fn 池_生成路由到选中供应商() {
        let 甲 = 起模型服务器(200, r#"{"choices":[{"message":{"content":"甲答复"}}]}"#, 200, r#"{"data":[]}"#);
        let 乙 = 起模型服务器(200, r#"{"choices":[{"message":{"content":"乙答复"}}]}"#, 200, r#"{"data":[]}"#);
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &甲.地址, "key-a", "甲模型", true),
                供应商("乙", &乙.地址, "key-b", "乙模型", true),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        let 结果 = 池.生成("你好".into()).unwrap();
        assert_eq!(结果, "甲答复");
        // 甲收到请求：body 携带选中模型与 Bearer 密钥
        let 甲请求 = 甲.记录.lock().unwrap().first().unwrap().clone();
        assert!(甲请求.contains("/chat"));
        assert!(甲请求.contains("甲模型"), "请求体应含选中模型，实际: {甲请求}");
        assert!(甲请求.contains("Bearer key-a"));
        assert!(!甲请求.contains("乙模型"));
        // 乙未收到请求
        assert!(乙.记录.lock().unwrap().is_empty());
    }

    #[test]
    fn 池_选中失败故障转移到下一供应商() {
        let 甲 = 起模型服务器(500, r#"{"error":"boom"}"#, 200, r#"{"data":[]}"#);
        let 乙 = 起模型服务器(200, r#"{"choices":[{"message":{"content":"乙答复"}}]}"#, 200, r#"{"data":[]}"#);
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &甲.地址, "key-a", "甲模型", true),
                供应商("乙", &乙.地址, "key-b", "乙模型", true),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        let 结果 = 池.生成("你好".into()).unwrap();
        assert_eq!(结果, "乙答复");
        assert!(!乙.记录.lock().unwrap().is_empty(), "乙应收到故障转移请求");
    }

    #[test]
    fn 池_全部禁用返回无可用错误() {
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &"127.0.0.1:1".parse().unwrap(), "key-a", "甲模型", false),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        assert!(!池.可用());
        let 错误 = 池.生成("你好".into()).unwrap_err();
        assert!(错误.to_string().contains("无可用"), "错误应含「无可用」，实际: {错误}");
    }

    #[test]
    fn 池_api_key缺失env视为不可用() {
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &"127.0.0.1:1".parse().unwrap(), "env:LLM_测试_不存在变量_abcxyz", "甲模型", true),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        assert!(!池.可用());
        assert!(池.生成("你好".into()).is_err());
    }

    #[test]
    fn 池_列表模型聚合含失败标注() {
        let 甲 = 起模型服务器(200, r#"{"choices":[]}"#, 200, r#"{"data":[{"id":"甲-1"},{"id":"甲-2"}]}"#);
        // 乙：models_url 指向无人监听的端口 1 → 连接失败
        let 乙地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 乙 = LlmProvider {
            name: "乙".into(),
            kind: "openai".into(),
            base_url: format!("http://{乙地址}/chat"),
            models_url: format!("http://{乙地址}/models"),
            api_key: "key-b".into(),
            model: "乙模型".into(),
            timeout_secs: 2,
            retry: 0,
            enabled: true,
        };
        let 配置 = LlmConfig {
            providers: vec![供应商("甲", &甲.地址, "key-a", "甲模型", true), 乙],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        let 结果 = 池.列表模型();
        assert_eq!(结果.len(), 2);
        let 甲条 = 结果.iter().find(|r| r.供应商 == "甲").unwrap();
        let 甲ids: Vec<&str> = 甲条.模型.as_ref().unwrap().iter().map(|m| m.id.as_str()).collect();
        assert_eq!(甲ids, vec!["甲-1", "甲-2"]);
        assert!(甲条.错误.is_none());
        let 乙条 = 结果.iter().find(|r| r.供应商 == "乙").unwrap();
        assert!(乙条.模型.is_none());
        assert!(乙条.错误.is_some(), "乙应标注错误，实际: {乙条:?}");
    }

    #[test]
    fn 池_供应商清单脱敏不含密钥() {
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &"127.0.0.1:1".parse().unwrap(), "super-secret-key", "甲模型", true),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        let 清单 = 池.供应商清单();
        let json = serde_json::to_string(&清单).unwrap();
        assert!(!json.contains("super-secret-key"), "清单不得泄漏密钥: {json}");
        assert!(json.contains("甲"));
    }

    #[test]
    fn 池_选择文件损坏回退默认() {
        let 文件 = 临时状态文件();
        std::fs::write(&文件, "这不是合法json{{{").unwrap();
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &"127.0.0.1:1".parse().unwrap(), "key-a", "甲模型", true),
                供应商("乙", &"127.0.0.1:1".parse().unwrap(), "key-b", "乙模型", true),
            ],
            selected_provider: "乙".into(),
            selected_model: "乙模型".into(),
            state_file: 文件.clone(),
        };
        let 池 = LLM池::从配置(&配置);
        let 选择 = 池.当前选择().unwrap();
        assert_eq!(选择.供应商, "乙");
        assert_eq!(选择.模型, "乙模型");
        let _ = std::fs::remove_file(&文件);
    }

    #[test]
    fn 池_从环境兼容主备构造() {
        let 主 = 起模型服务器(200, r#"{"choices":[{"message":{"content":"环境主答复"}}]}"#, 200, r#"{"data":[]}"#);
        std::env::set_var("LLM_API_KEY", "env-main-key");
        std::env::set_var("LLM_BASE_URL", format!("http://{}/chat", 主.地址));
        std::env::set_var("LLM_MODEL", "环境主模型");
        std::env::set_var("LLM_FALLBACK_API_KEY", "env-fallback-key");
        std::env::set_var("LLM_FALLBACK_BASE_URL", "http://127.0.0.1:1/chat");
        std::env::set_var("LLM_FALLBACK_MODEL", "环境备模型");
        let 池 = LLM池::从环境().unwrap();
        let 选择 = 池.当前选择().unwrap();
        assert_eq!(选择.供应商, "主");
        assert_eq!(选择.模型, "环境主模型");
        let 结果 = 池.生成("你好".into()).unwrap();
        assert_eq!(结果, "环境主答复");
        std::env::remove_var("LLM_API_KEY");
        std::env::remove_var("LLM_BASE_URL");
        std::env::remove_var("LLM_MODEL");
        std::env::remove_var("LLM_FALLBACK_API_KEY");
        std::env::remove_var("LLM_FALLBACK_BASE_URL");
        std::env::remove_var("LLM_FALLBACK_MODEL");
    }
}
