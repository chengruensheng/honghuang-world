#[cfg(test)]
mod tests {
    use hm_cognition::{
        图谱, 符号, 符号种类, 模块, 维度, 格位, 心智地图, 消息角色,
        维度载荷, 目标载荷, 上下文库, 上下文消息, 纠错事件, 三态存储, Rust扫描器,
    };

    // ==================== 三态存储（v1.42 持久化） ====================

    fn 三态临时目录(名: &str) -> String {
        let 目录 = std::env::temp_dir().join("zd-cognition-三态").join(名);
        let _ = std::fs::remove_dir_all(&目录);
        目录.to_string_lossy().into_owned()
    }

    fn 造三态() -> (图谱, 心智地图, 上下文库) {
        let mut 图谱 = 图谱::新();
        图谱.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动装配".into() });
        图谱.添加符号(符号 { 名称: "运行".into(), 种类: 符号种类::函数, 所属模块: "hm-agent".into(), 签名: Some("fn(任务)".into()) });
        图谱.添加技术栈("serde");
        let mut 心智 = 心智地图::新();
        心智.写(维度::目标, "现况", "目标现况摘要", 0.85, vec!["证据A".into()]).expect("写格位");
        心智.写载荷(维度::目标, "初心", "初心摘要", 0.9, 维度载荷::目标(目标载荷 {
            初心: "自主演化".into(),
            现况: "三态就绪".into(),
            愿景: "无人干预".into(),
            偏移: Some("偏慢".into()),
            度量: "测试数".into(),
            依赖: vec!["Rust".into()],
        }), vec![]).expect("写载荷格位");
        let mut 上下文 = 上下文库::新();
        上下文.追加(消息角色::用户, "第一条");
        上下文.追加(消息角色::助手, "第二条");
        (图谱, 心智, 上下文)
    }

    #[test]
    fn 三态存储_保存加载往返() {
        let (图谱, 心智, 上下文) = 造三态();
        let 存储 = 三态存储::新(三态临时目录("往返")).expect("创建存储");
        存储.保存(&图谱, &心智, &上下文).expect("保存三态");

        let (图谱2, 心智2, 上下文2) = 存储.加载().expect("加载三态");
        assert_eq!(图谱2.模块集[0].名称, "hm-linkage");
        assert_eq!(图谱2.符号集[0].签名.as_deref(), Some("fn(任务)"));
        assert_eq!(图谱2.技术栈, vec!["serde".to_string()]);
        let 格位 = 心智2.查询格位(维度::目标, "现况").expect("格位应存在");
        assert_eq!(格位.摘要, "目标现况摘要");
        assert!((格位.可信度 - 0.85).abs() < 1e-6);
        assert_eq!(上下文2.长度(), 2);
        assert_eq!(上下文2.最近(2)[0].内容, "第一条");
        assert_eq!(上下文2.最近(2)[1].内容, "第二条");
    }

    #[test]
    fn 三态存储_空目录加载报错() {
        let 存储 = 三态存储::新(三态临时目录("空目录")).expect("创建存储");
        assert!(存储.加载().is_err(), "空目录加载应报错");
        assert!(!存储.已存在());
    }

    #[test]
    fn 三态存储_上下文库超限截断() {
        let 目录 = 三态临时目录("截断");
        let 存储 = 三态存储::新(&目录).expect("创建存储");
        let 消息流: Vec<上下文消息> = (1..=1005)
            .map(|id| 上下文消息 { id, 角色: 消息角色::用户, 内容: format!("消息{id}"), 时间戳: id })
            .collect();
        let 上下文 = 上下文库::导入(消息流, 1000);
        存储.保存上下文(&上下文).expect("保存上下文");

        let 加载 = 存储.加载上下文().expect("加载上下文");
        assert_eq!(加载.长度(), 1000, "超限应截断到硬上限");
        assert_eq!(加载.最近(1)[0].内容, "消息1005", "最新消息保留");
        assert_eq!(加载.全部()[0].内容, "消息6", "最旧 5 条被丢弃");
    }

    #[test]
    fn 三态存储_心智地图格位载荷保留() {
        let (_图谱, 心智, _上下文) = 造三态();
        let 存储 = 三态存储::新(三态临时目录("载荷")).expect("创建存储");
        存储.保存心智(&心智).expect("保存心智");

        let 加载 = 存储.加载心智().expect("加载心智");
        let 格位 = 加载.查询格位(维度::目标, "初心").expect("格位应存在");
        match &格位.维度载荷 {
            Some(维度载荷::目标(载荷)) => {
                assert_eq!(载荷.初心, "自主演化");
                assert_eq!(载荷.愿景, "无人干预");
                assert_eq!(载荷.依赖, vec!["Rust".to_string()]);
            }
            _ => panic!("目标格位应持有目标载荷"),
        }
    }

    #[test]
    fn 三态存储_纠错事件追加与加载() {
        let 存储 = 三态存储::新(三态临时目录("纠错")).expect("创建存储");
        let 事件1 = 纠错事件 { 时间: 1, 检测到的差异: "摘要过时".into(), 受影响格位: vec!["目标·现况".into()], 新摘要: "新摘要".into(), 新可信度: 0.6, 教训触发: None, 联动链: vec![] };
        let 事件2 = 纠错事件 { 时间: 2, 检测到的差异: "规则冲突".into(), 受影响格位: vec!["规则·红线".into()], 新摘要: "规则修正".into(), 新可信度: 0.5, 教训触发: Some("同类纠错≥2，应提炼教训".into()), 联动链: vec![] };
        存储.追加纠错事件(&事件1).expect("追加");
        存储.追加纠错事件(&事件2).expect("追加");

        let 事件集 = 存储.加载纠错事件().expect("加载");
        assert_eq!(事件集.len(), 2);
        assert_eq!(事件集[0].新摘要, "新摘要");
        assert_eq!(事件集[1].新可信度, 0.5);
        assert_eq!(事件集[1].教训触发.as_deref(), Some("同类纠错≥2，应提炼教训"));
    }

    #[test]
    fn 三态存储_损坏文件加载报错() {
        let 目录 = 三态临时目录("损坏");
        let 存储 = 三态存储::新(&目录).expect("创建存储");
        std::fs::write(存储.目录().join("心智地图.json"), "这不是合法JSON{").expect("写入坏文件");

        assert!(存储.加载心智().is_err(), "损坏心智文件应报错");
        assert!(存储.已存在(), "存在文件应判定为已存在");
    }

    #[test]
    fn 三态存储_上下文库坏行容忍() {
        let 目录 = 三态临时目录("坏行");
        let 存储 = 三态存储::新(&目录).expect("创建存储");
        let 好消息 = 上下文消息 { id: 2, 角色: 消息角色::助手, 内容: "完好".into(), 时间戳: 2 };
        let 内容 = format!("坏行不是JSON\n{}\n", serde_json::to_string(&好消息).expect("序列化"));
        std::fs::write(存储.目录().join("上下文库.jsonl"), 内容).expect("写坏行文件");

        let 加载 = 存储.加载上下文().expect("坏行应被跳过");
        assert_eq!(加载.长度(), 1, "坏行跳过，好行保留");
        assert_eq!(加载.最近(1)[0].内容, "完好");
    }

    // ==================== Rust 扫描器（v1.43 图谱自动构建） ====================

    fn 微型workspace() -> std::path::PathBuf {
        let 根 = std::env::temp_dir().join("zd-cognition-扫描器").join(format!("微_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(根.join("甲").join("src")).expect("建甲");
        std::fs::create_dir_all(根.join("乙").join("src")).expect("建乙");
        std::fs::create_dir_all(根.join("target")).expect("建target");
        std::fs::create_dir_all(根.join("命名空间")).expect("建命名空间");
        // 顶层 workspace
        std::fs::write(根.join("Cargo.toml"), "[workspace]\nmembers = [\"甲\", \"乙\"]\n").expect("写根Cargo");
        // 甲：依赖 乙 + serde；含 pub fn / pub struct / pub const / 私有 fn
        std::fs::write(根.join("甲").join("Cargo.toml"), "[package]\nname = \"甲\"\n[dependencies]\n乙 = { path = \"../乙\" }\nserde = \"1\"\n").expect("写甲Cargo");
        std::fs::write(
            根.join("甲").join("src").join("入口.rs"),
            "// 注释 pub fn 不应匹配\n#[derive(Clone)]\npub fn 运行(任务: &str) -> bool { true }\nfn 私有() {}\npub struct 图 { pub 名称: String }\npub const 阈值: usize = 5;\nimpl 图 { pub fn 新() -> Self { 图 { 名称: String::new() } } }\npub async fn 异步(输入: u32) -> u32 { 输入 }\n",
        ).expect("写甲入口");
        // 乙：无 Cargo.toml 依赖段；含 pub trait
        std::fs::write(根.join("乙").join("Cargo.toml"), "[package]\nname = \"乙\"\n").expect("写乙Cargo");
        std::fs::write(根.join("乙").join("src").join("模块.rs"), "pub trait 契约 { fn 交付(&self) -> bool; }\npub mod 壳 {}").expect("写乙模块");
        // 命名空间：无 .rs 只有子目录
        std::fs::create_dir_all(根.join("命名空间").join("深层")).expect("建深层");
        根
    }

    #[test]
    fn 扫描器_临时workspace建图谱() {
        let 根 = 微型workspace();
        let 图 = Rust扫描器::新().扫描(&根).expect("扫描应成功");

        let 名称: Vec<&str> = 图.模块集.iter().map(|模块| 模块.名称.as_str()).collect();
        assert!(名称.contains(&"甲"), "应含甲 crate，实际 {名称:?}");
        assert!(名称.contains(&"乙"), "应含乙 crate，实际 {名称:?}");
        assert_eq!(图.模块集.len(), 2, "只收 crate 级模块（命名空间/壳 不入模块集）");

        let 运行 = 图.查符号("运行").expect("pub fn 运行 应被提取");
        assert_eq!(运行.种类, 符号种类::函数);
        assert_eq!(运行.所属模块, "甲");
        assert!(运行.签名.as_deref().unwrap_or("").contains("fn 运行(任务: &str)"), "签名应含参数");
        let 图符 = 图.查符号("图").expect("pub struct 图 应被提取");
        assert_eq!(图符.种类, 符号种类::类型);
        let 阈值 = 图.查符号("阈值").expect("pub const 阈值 应被提取");
        assert_eq!(阈值.种类, 符号种类::常量);
        assert!(图.查符号("私有").is_none(), "私有 fn 不应提取");
        let 异步 = 图.查符号("异步").expect("pub async fn 应被提取");
        assert_eq!(异步.种类, 符号种类::函数);
        assert!(图.查符号("契约").is_some(), "乙 的 pub trait 应提取（壳模块内符号归包）");
        assert!(图.查符号("壳").is_none(), "pub mod 壳 不应提取");

        assert!(图.依赖集.iter().any(|边| 边.源 == "甲" && 边.目标 == "乙"), "甲→乙 依赖边应存在");
        assert!(图.技术栈.iter().any(|栈| 栈 == "serde"), "外部依赖 serde 应入技术栈");
    }

    #[test]
    fn 扫描器_不存在路径报错() {
        let 根 = std::env::temp_dir().join(format!("zd-cognition-不存在-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&根);
        let 结果 = Rust扫描器::新().扫描(&根);
        assert!(结果.is_err(), "不存在路径应报错");
    }

    #[test]
    fn 扫描器_非目录报错() {
        let 文件 = std::env::temp_dir().join("zd-cognition-扫描器-非目录.txt");
        std::fs::write(&文件, "x").expect("写临时文件");
        let 结果 = Rust扫描器::新().扫描(&文件);
        assert!(结果.is_err(), "文件路径应报错");
        let _ = std::fs::remove_file(&文件);
    }

    #[test]
    fn 扫描器_单crate模式() {
        let 根 = std::env::temp_dir().join("zd-cognition-扫描器").join(format!("单_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(根.join("src")).expect("建src");
        std::fs::write(根.join("Cargo.toml"), "[package]\nname = \"单包\"\n").expect("写Cargo");
        std::fs::write(根.join("src").join("main.rs"), "pub fn 主() {}").expect("写main");

        let 图 = Rust扫描器::新().扫描(&根).expect("扫描应成功");
        let 名称: Vec<&str> = 图.模块集.iter().map(|模块| 模块.名称.as_str()).collect();
        assert_eq!(名称, vec!["单包"], "无 workspace 时根即 package，模块集 {名称:?}");
        assert!(图.查符号("主").is_some(), "pub fn 主 应提取");
    }

    #[test]
    fn 扫描器_真实洪荒靶子() {
        // 从本 crate 目录向上找含 [workspace] 声明的工程根（不硬编码盘符）
        let 起点 = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let 根 = 起点
            .ancestors()
            .find(|目录| {
                let 货 = 目录.join("Cargo.toml");
                货.is_file()
                    && std::fs::read_to_string(&货)
                        .map(|内容| 内容.contains("[workspace]"))
                        .unwrap_or(false)
            })
            .map(|目录| 目录.to_path_buf());
        let Some(根) = 根 else {
            return; // 找不到工程根时跳过（不视为失败）
        };
        let 图 = Rust扫描器::新().扫描(&根).expect("扫描真实工程应成功");
        assert!(图.模块集.len() >= 20, "crate 级模块应 ≥20，实际 {}", 图.模块集.len());
        assert!(图.查符号("启动").is_some(), "pub fn 启动 应被提取（hm-bootstrap）");
        assert!(图.查符号("装配认知").is_some(), "pub fn 装配认知 应被提取（hm-agent）");
        assert!(图.依赖集.iter().any(|边| 边.源 == "hm-agent"), "hm-agent 应有依赖边");
        assert!(图.技术栈.iter().any(|栈| 栈 == "serde"), "技术栈应含 serde");
    }

    #[test]
    fn 扫描器_技术栈去重() {
        let 根 = std::env::temp_dir().join("zd-cognition-扫描器").join(format!("栈_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(根.join("丙")).expect("建丙");
        std::fs::write(根.join("Cargo.toml"), "[workspace]\nmembers = [\"丙\"]\n").expect("写Cargo");
        std::fs::write(
            根.join("丙").join("Cargo.toml"),
            "[package]\nname = \"丙\"\n[dependencies]\nserde = \"1\"\nserde = \"1\"\n[dependencies.tokio]\nversion = \"1\"\n",
        ).expect("写丙Cargo");
        std::fs::create_dir_all(根.join("丙").join("src")).expect("建丙src");
        std::fs::write(根.join("丙").join("src").join("主.rs"), "pub fn 主() {}").expect("写主");

        let 图 = Rust扫描器::新().扫描(&根).expect("扫描应成功");
        let 次数 = 图.技术栈.iter().filter(|栈| 栈.as_str() == "serde").count();
        assert_eq!(次数, 1, "技术栈应去重，实际 {次数}");
        assert!(图.技术栈.contains(&"tokio".to_string()), "多行依赖段也应解析");
    }
}
