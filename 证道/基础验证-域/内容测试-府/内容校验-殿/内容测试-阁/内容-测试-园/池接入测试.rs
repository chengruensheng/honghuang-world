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
                        401 => "401 Unauthorized",
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
            json_mode: false,
        }
    }

    static 池序号: AtomicU64 = AtomicU64::new(0);

    /// 独立状态文件路径（独立子目录，避免并行测试互踩接入文件）
    fn 临时状态文件() -> String {
        let 目录 = std::env::temp_dir().join(format!("洪荒池测试_{}", 池序号.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::create_dir_all(&目录);
        目录.join("llm-选择.json").to_string_lossy().to_string()
    }

    #[test]
    fn 池_探测_自定义网络返回模型() {
        let 服务器 = 起模型服务器(200, r#"{"choices":[]}"#, 200, r#"{"data":[{"id":"甲-1"},{"id":"甲-2"}]}"#);
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: String::new() });
        let (来源, 模型) = 池.探测模型(None, Some(&format!("http://{}", 服务器.地址)), None).unwrap();
        assert_eq!(来源, "网络");
        let ids: Vec<&str> = 模型.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["甲-1", "甲-2"]);
    }

    #[test]
    fn 池_探测_请求内密钥优先() {
        let 服务器 = 起模型服务器(200, r#"{"choices":[]}"#, 200, r#"{"data":[{"id":"甲-1"}]}"#);
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: String::new() });
        池.探测模型(None, Some(&format!("http://{}", 服务器.地址)), Some("sk-probe")).unwrap();
        let 请求 = 服务器.记录.lock().unwrap().first().unwrap().clone();
        assert!(请求.contains("Bearer sk-probe"), "请求应带请求内密钥，实际: {请求}");
    }

    #[test]
    fn 池_探测_回退池内存密钥() {
        let 服务器 = 起模型服务器(200, r#"{"choices":[]}"#, 200, r#"{"data":[{"id":"甲-1"}]}"#);
        let 配置 = LlmConfig {
            providers: vec![供应商("甲", &服务器.地址, "key-stored", "甲模型", true)],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        池.探测模型(Some("甲"), None, None).unwrap();
        let 请求 = 服务器.记录.lock().unwrap().first().unwrap().clone();
        assert!(请求.contains("Bearer key-stored"), "应回退池内存密钥，实际: {请求}");
    }

    #[test]
    fn 池_探测_无鉴权探测() {
        let 服务器 = 起模型服务器(200, r#"{"choices":[]}"#, 200, r#"{"data":[{"id":"甲-1"}]}"#);
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: String::new() });
        池.探测模型(None, Some(&format!("http://{}", 服务器.地址)), None).unwrap();
        let 请求 = 服务器.记录.lock().unwrap().first().unwrap().clone();
        assert!(!请求.contains("Authorization"), "无密钥不应带鉴权头，实际: {请求}");
    }

    #[test]
    fn 池_探测_401提示密钥() {
        let 服务器 = 起模型服务器(200, r#"{"choices":[]}"#, 401, r#"{"error":"bad key"}"#);
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: String::new() });
        let 错误 = 池.探测模型(None, Some(&format!("http://{}", 服务器.地址)), Some("wrong")).unwrap_err();
        assert!(错误.to_string().contains("检查 API 密钥"), "401 应提示检查密钥，实际: {错误}");
    }

    #[test]
    fn 池_探测_坏响应报错() {
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: String::new() });
        assert!(池.探测模型(None, Some("http://127.0.0.1:1"), Some("key")).is_err());
        assert!(池.探测模型(None, None, None).is_err());
    }

    #[test]
    fn 池_探测_坏行跳过() {
        let 服务器 = 起模型服务器(200, r#"{"choices":[]}"#, 200, r#"{"data":[{"id":"甲-1"},{"no-id":1},{"id":"甲-3"}]}"#);
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: String::new() });
        let (_, 模型) = 池.探测模型(None, Some(&format!("http://{}", 服务器.地址)), None).unwrap();
        assert_eq!(模型.len(), 2, "坏行应跳过，实际: {模型:?}");
    }

    #[test]
    fn 池_接入_明文不落盘() {
        let 文件 = 临时状态文件();
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: 文件.clone() });
        let (选择, 片段) = 池.接入("甲", "http://127.0.0.1:1/v1", "plain-secret-key", "甲模型").unwrap();
        assert_eq!(选择.供应商, "甲");
        assert_eq!(选择.模型, "甲模型");
        assert!(片段.contains("plain-secret-key"), "配置片段应含 env 提示，实际: {片段}");
        let 接入文件 = std::path::Path::new(&文件).parent().unwrap().join("llm-接入.json");
        assert!(!接入文件.exists(), "明文密钥不得落盘");
        assert!(池.可用());
        let _ = std::fs::remove_file(&文件);
    }

    #[test]
    fn 池_接入_env引用落盘并恢复() {
        std::env::set_var("LLM_测试_接入_密钥_abcxyz", "env-secret");
        let 文件 = 临时状态文件();
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: 文件.clone() });
        池.接入("乙", "http://127.0.0.1:1/v1", "env:LLM_测试_接入_密钥_abcxyz", "乙模型").unwrap();
        let 接入文件 = std::path::Path::new(&文件).parent().unwrap().join("llm-接入.json");
        let 文本 = std::fs::read_to_string(&接入文件).unwrap();
        assert!(文本.contains("env:LLM_测试_接入_密钥_abcxyz"));
        assert!(!文本.contains("env-secret"), "接入文件不得含明文密钥: {文本}");
        // 重启恢复：新池 从配置（同状态文件）+ 从接入文件合并
        let 新池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: 文件.clone() });
        新池.从接入文件合并().unwrap();
        let 清单 = 新池.供应商清单();
        assert!(清单.iter().any(|s| s.名称 == "乙"), "合并后应恢复供应商: {清单:?}");
        assert!(新池.可用());
        let _ = std::fs::remove_file(&文件);
        let _ = std::fs::remove_file(&接入文件);
        std::env::remove_var("LLM_测试_接入_密钥_abcxyz");
    }

    #[test]
    fn 池_接入_重名覆盖() {
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: String::new() });
        池.接入("甲", "http://127.0.0.1:1/v1", "key-1", "模型1").unwrap();
        池.接入("甲", "http://127.0.0.1:2/v2", "key-2", "模型2").unwrap();
        let 清单 = 池.供应商清单();
        assert_eq!(清单.len(), 1, "重名应覆盖");
        // 接入时端点已归一化为完整 chat/completions 端点（供应商.rs 接入契约）
        assert_eq!(清单[0].地址, "http://127.0.0.1:2/v2/chat/completions");
        assert_eq!(清单[0].模型, "模型2");
        let 选择 = 池.当前选择().unwrap();
        assert_eq!(选择.供应商, "甲");
        assert_eq!(选择.模型, "模型2");
    }

    #[test]
    fn 池_从接入文件_损坏跳过() {
        let 文件 = 临时状态文件();
        let 接入文件 = std::path::Path::new(&文件).parent().unwrap().join("llm-接入.json");
        std::fs::write(&接入文件, "坏json{{{").unwrap();
        let 池 = LLM池::从配置(&LlmConfig { providers: vec![], selected_provider: String::new(), selected_model: String::new(), state_file: 文件.clone() });
        assert!(池.从接入文件合并().is_ok(), "损坏接入文件应跳过不阻断");
        let _ = std::fs::remove_file(&文件);
        let _ = std::fs::remove_file(&接入文件);
    }

    #[test]
    fn 池_json模式_启用时生成请求含response_format() {
        let 服务器 = 起模型服务器(200, r#"{"choices":[{"message":{"content":"json答复"}}]}"#, 200, r#"{"data":[]}"#);
        let 配置 = LlmConfig {
            providers: vec![
                LlmProvider {
                    name: "甲".into(),
                    kind: "openai".into(),
                    base_url: format!("http://{}/chat", 服务器.地址),
                    models_url: format!("http://{}/models", 服务器.地址),
                    api_key: "key-a".into(),
                    model: "甲模型".into(),
                    timeout_secs: 5,
                    retry: 0,
                    enabled: true,
                    json_mode: true, // 开启 JSON 输出模式
                },
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        let 结果 = 池.生成("输出json".into()).unwrap();
        assert_eq!(结果, "json答复");
        let 请求 = 服务器.记录.lock().unwrap().first().unwrap().clone();
        assert!(请求.contains("response_format"), "启用 json_mode 请求应含 response_format: {请求}");
        assert!(请求.contains("json_object"), "response_format 应为 json_object: {请求}");
    }

    #[test]
    fn 池_json模式_未启用时生成请求不含response_format() {
        let 服务器 = 起模型服务器(200, r#"{"choices":[{"message":{"content":"普通答复"}}]}"#, 200, r#"{"data":[]}"#);
        let 配置 = LlmConfig {
            providers: vec![
                供应商("甲", &服务器.地址, "key-a", "甲模型", true), // 默认 json_mode = false
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        let 池 = LLM池::从配置(&配置);
        let 结果 = 池.生成("普通请求".into()).unwrap();
        assert_eq!(结果, "普通答复");
        let 请求 = 服务器.记录.lock().unwrap().first().unwrap().clone();
        assert!(!请求.contains("response_format"), "未启用 json_mode 请求不应含 response_format: {请求}");
    }
}
