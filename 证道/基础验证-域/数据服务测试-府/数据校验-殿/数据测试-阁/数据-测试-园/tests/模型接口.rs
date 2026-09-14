use super::*;

    /// 构造带 LLM 池的状态（复用 构造数据状态，仅覆写池字段）
    fn 构造池状态(池: Arc<LLM池>) -> 数据服务状态 {
        let mut 状态 = 构造数据状态();
        状态.llm池 = Some(池);
        状态
    }

    fn 池供应商(名: &str, 端点: &str, 列表: &str, 密钥: &str, 模型: &str) -> LlmProvider {
        LlmProvider {
            name: 名.into(),
            kind: "openai".into(),
            base_url: 端点.into(),
            models_url: 列表.into(),
            api_key: 密钥.into(),
            model: 模型.into(),
            timeout_secs: 2,
            retry: 0,
            enabled: true,
            json_mode: false,
            max_tokens: 32768,
        }
    }

    #[tokio::test]
    async fn 模型接口_未装配返回未配置() {
        let 状态 = 构造数据状态();
        let Json(响应) = 模型状态接口(State(状态)).await;
        assert!(!响应.配置);
        assert!(响应.供应商.is_empty());
        assert!(响应.当前选择.is_none());
    }

    #[tokio::test]
    async fn 模型接口_状态返回选择与供应商() {
        let 甲地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 池 = LLM池::从配置(&LlmConfig {
            providers: vec![池供应商("甲", &format!("http://{甲地址}/chat"), &format!("http://{甲地址}/models"), "secret-http", "甲模型")],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        });
        let 状态 = 构造池状态(Arc::new(池));
        let Json(响应) = 模型状态接口(State(状态)).await;
        // 状态接口序列化不得泄漏密钥（先整体借用，避免部分 move）
        let json = serde_json::to_string(&响应).unwrap();
        assert!(!json.contains("secret-http"));
        assert!(响应.配置);
        let 选择 = 响应.当前选择.clone().unwrap();
        assert_eq!(选择.供应商, "甲");
        assert_eq!(选择.模型, "甲模型");
        assert_eq!(响应.供应商.len(), 1);
        assert_eq!(响应.供应商[0].名称, "甲");
    }

    #[tokio::test]
    async fn 模型接口_模型列表聚合() {
        // 甲：本地可访问 list-models；乙：端口 1 拒绝连接 → 标注错误
        let 监听 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let 地址 = 监听.local_addr().unwrap();
        let 列表体 = r#"{"data":[{"id":"甲-1"},{"id":"甲-2"}]}"#.to_string();
        std::thread::spawn(move || {
            for 连接 in 监听.incoming() {
                if let Ok(mut 流) = 连接 {
                    use std::io::{Read, Write};
                    let mut 缓冲: Vec<u8> = Vec::new();
                    let mut 临时 = [0u8; 4096];
                    loop {
                        let n = 流.read(&mut 临时).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        缓冲.extend_from_slice(&临时[..n]);
                        if String::from_utf8_lossy(&缓冲).contains("\r\n\r\n") {
                            break;
                        }
                    }
                    let 响应 = format!(
                        "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        列表体.len(),
                        列表体,
                    );
                    let _ = 流.write_all(响应.as_bytes());
                    let _ = 流.flush();
                }
            }
        });
        let 乙地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 池 = LLM池::从配置(&LlmConfig {
            providers: vec![
                池供应商("甲", &format!("http://{地址}/chat"), &format!("http://{地址}/models"), "key-a", "甲模型"),
                池供应商("乙", &format!("http://{乙地址}/chat"), &format!("http://{乙地址}/models"), "key-b", "乙模型"),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        });
        let 状态 = 构造池状态(Arc::new(池));
        let Json(响应) = 模型列表接口(State(状态)).await;
        assert!(响应.配置);
        assert_eq!(响应.结果.len(), 2);
        let 甲条 = 响应.结果.iter().find(|r| r.供应商 == "甲").unwrap();
        let ids: Vec<&str> = 甲条.模型.as_ref().unwrap().iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["甲-1", "甲-2"]);
        let 乙条 = 响应.结果.iter().find(|r| r.供应商 == "乙").unwrap();
        assert!(乙条.错误.is_some(), "乙应标注列表错误，实际: {乙条:?}");
    }

    #[tokio::test]
    async fn 模型接口_选择生效() {
        let 甲地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 乙地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 池 = LLM池::从配置(&LlmConfig {
            providers: vec![
                池供应商("甲", &format!("http://{甲地址}/chat"), &format!("http://{甲地址}/models"), "key-a", "甲模型"),
                池供应商("乙", &format!("http://{乙地址}/chat"), &format!("http://{乙地址}/models"), "key-b", "乙模型"),
            ],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        });
        let 状态 = 构造池状态(Arc::new(池));
        let Json(响应) = 模型选择接口(
            State(状态.clone()),
            Json(LLM选择请求 { 供应商: "乙".into(), 模型: "乙模型".into() }),
        )
        .await
        .unwrap();
        assert!(响应.成功);
        let Json(状态响应) = 模型状态接口(State(状态)).await;
        let 选择 = 状态响应.当前选择.unwrap();
        assert_eq!(选择.供应商, "乙");
        assert_eq!(选择.模型, "乙模型");
    }

    #[tokio::test]
    async fn 模型接口_选择非法供应商报错() {
        let 甲地址: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let 池 = LLM池::从配置(&LlmConfig {
            providers: vec![池供应商("甲", &format!("http://{甲地址}/chat"), &format!("http://{甲地址}/models"), "key-a", "甲模型")],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        });
        let 状态 = 构造池状态(Arc::new(池));
        let 结果 = 模型选择接口(
            State(状态),
            Json(LLM选择请求 { 供应商: "不存在".into(), 模型: "某模型".into() }),
        )
        .await;
        assert!(结果.is_err());
        assert_eq!(结果.unwrap_err(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn 模型接口_模板清单返回() {
        let Json(响应) = 模型模板接口().await;
        assert!(响应.模板.len() >= 9);
        assert!(响应.模板.iter().any(|t| t.名称 == "deepseek"));
        let json = serde_json::to_string(&响应).unwrap();
        assert!(!json.contains("api_key") && !json.contains("密钥"), "模板不得含密钥字段: {json}");
    }

    #[tokio::test]
    async fn 模型接口_探测_目录命中() {
        let 状态 = 构造数据状态();
        let Json(响应) = 模型探测接口(State(状态), Json(LLM探测请求 {
            供应商: Some("deepseek".into()),
            地址: None,
            密钥: None,
        })).await;
        assert_eq!(响应.来源, "目录");
        assert!(响应.模型.iter().any(|m| m.id == "deepseek-chat"));
        assert!(响应.错误.is_none());
    }

    #[tokio::test]
    async fn 模型接口_探测_自定义网络() {
        let 监听 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let 地址 = 监听.local_addr().unwrap();
        let 列表体 = r#"{"data":[{"id":"探测-1"},{"id":"探测-2"}]}"#.to_string();
        std::thread::spawn(move || {
            for 连接 in 监听.incoming() {
                if let Ok(mut 流) = 连接 {
                    use std::io::{Read, Write};
                    let mut 缓冲: Vec<u8> = Vec::new();
                    let mut 临时 = [0u8; 4096];
                    loop {
                        let n = 流.read(&mut 临时).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        缓冲.extend_from_slice(&临时[..n]);
                        if String::from_utf8_lossy(&缓冲).contains("\r\n\r\n") {
                            break;
                        }
                    }
                    let 响应 = format!(
                        "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        列表体.len(),
                        列表体,
                    );
                    let _ = 流.write_all(响应.as_bytes());
                    let _ = 流.flush();
                }
            }
        });
        let 状态 = 构造数据状态();
        let Json(响应) = 模型探测接口(State(状态), Json(LLM探测请求 {
            供应商: None,
            地址: Some(format!("http://{地址}")),
            密钥: None,
        })).await;
        assert_eq!(响应.来源, "网络");
        let ids: Vec<&str> = 响应.模型.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["探测-1", "探测-2"]);
    }

    #[tokio::test]
    async fn 模型接口_接入_缺字段400() {
        let 状态 = 构造池状态(Arc::new(LLM池::从配置(&LlmConfig {
            providers: vec![],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        })));
        let 结果 = 模型接入接口(State(状态), Json(LLM接入请求 {
            名称: String::new(),
            地址: "http://127.0.0.1:1/v1".into(),
            密钥: "key".into(),
            模型: "m".into(),
        })).await;
        assert!(结果.is_err());
        assert_eq!(结果.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn 模型接口_接入_成功含配置片段() {
        let 状态 = 构造池状态(Arc::new(LLM池::从配置(&LlmConfig {
            providers: vec![],
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        })));
        let Json(响应) = 模型接入接口(State(状态), Json(LLM接入请求 {
            名称: "甲".into(),
            地址: "http://127.0.0.1:1/v1".into(),
            密钥: "secret-key".into(),
            模型: "甲模型".into(),
        })).await.unwrap();
        assert!(响应.成功);
        let 选择 = 响应.选择.clone().unwrap();
        assert_eq!(选择.供应商, "甲");
        assert_eq!(选择.模型, "甲模型");
        let 片段 = 响应.配置片段.clone().unwrap();
        assert!(片段.contains("base_url"), "配置片段应含 base_url: {片段}");
        assert!(片段.contains("secret-key"), "配置片段应含 env 提示: {片段}");
    }
