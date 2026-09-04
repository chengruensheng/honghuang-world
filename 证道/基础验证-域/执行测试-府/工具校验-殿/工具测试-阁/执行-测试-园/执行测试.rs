#[cfg(test)]
mod tests {
    use hm_contract::Component;
    use hm_execute::本地执行器;
    use hm_execute_contract::执行器;

    fn 准备工作区(名: &str) -> String {
        let 根 = std::env::temp_dir().join(format!("zd_execute_test_{名}"));
        let 根字符串 = 根.to_string_lossy().into_owned();
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).expect("创建工作区目录");
        根字符串
    }

    #[test]
    fn 写文件后读回内容一致() {
        let 根 = 准备工作区("写读");
        let 执行器 = 本地执行器::new(&根);
        assert!(执行器.写文件("hello.txt", "你好世界").is_ok());
        let 内容 = 执行器.读文件("hello.txt").expect("读文件应成功");
        assert_eq!(内容, "你好世界");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 越界路径被拒绝() {
        let 根 = 准备工作区("越界");
        let 执行器 = 本地执行器::new(&根);
        assert!(执行器.写文件("../越界.txt", "x").is_err());
        assert!(执行器.读文件("../越界.txt").is_err());
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 运行命令返回标准输出() {
        let 根 = 准备工作区("命令");
        let 执行器 = 本地执行器::new(&根);
        let 输出 = 执行器.运行命令("echo hello").expect("命令应成功");
        assert!(输出.contains("hello"));
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 命令失败返回错误() {
        let 根 = 准备工作区("失败命令");
        let 执行器 = 本地执行器::new(&根);
        assert!(执行器.运行命令("definitely_not_a_command_12345").is_err());
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 组件名称正确() {
        let 执行器 = 本地执行器::new(".");
        assert_eq!(执行器.name(), "本地执行器");
    }

    #[test]
    fn 运行命令超时返回错误() {
        let 根 = 准备工作区("超时");
        let 执行器 = 本地执行器::new(&根).设置命令超时(1);
        let 结果 = 执行器.运行命令("ping -t 127.0.0.1");
        assert!(结果.is_err());
        assert!(结果.expect_err("应超时").to_string().contains("命令超时"));
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 运行命令输出被截断到上限() {
        let 根 = 准备工作区("截断");
        let 执行器 = 本地执行器::new(&根).设置最大输出(16);
        let 输出 = 执行器.运行命令("echo 012345678901234567890123456789").expect("命令应成功");
        assert!(输出.len() <= 16, "输出应被截断到 16 字节内，实际 {}", 输出.len());
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 写文件覆盖前自动备份() {
        let 根 = 准备工作区("备份");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("a.txt", "旧内容").expect("首次写");
        执行器.写文件("a.txt", "新内容").expect("覆盖写");
        assert_eq!(执行器.读文件("a.txt").expect("读"), "新内容");
        assert_eq!(执行器.读文件("a.txt.bak").expect("读备份"), "旧内容");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 危险命令被拦截且不执行() {
        let 根 = 准备工作区("危险命令");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("a.txt", "重要内容").expect("写文件");
        let 结果 = 执行器.运行命令("del a.txt");
        assert!(结果.is_err());
        assert!(结果.expect_err("应拦截").to_string().contains("危险命令"));
        assert_eq!(执行器.读文件("a.txt").expect("读"), "重要内容");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 危险命令关键词不误伤合法命令() {
        let 根 = 准备工作区("误伤");
        let 执行器 = 本地执行器::new(&根);
        // "model" 含 "del" 子串，但按词边界不匹配，不应被拦截
        let 输出 = 执行器.运行命令("echo model").expect("合法命令不应被拦截");
        assert!(输出.contains("model"));
        let _ = std::fs::remove_dir_all(&根);
    }
}