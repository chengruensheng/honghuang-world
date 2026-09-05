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
        // type 是白名单内命令，但读取不存在的文件会失败
        assert!(执行器.运行命令("type 不存在的文件.txt").is_err());
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
    fn 白名单外命令被拦截且不执行() {
        let 根 = 准备工作区("危险命令");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("a.txt", "重要内容").expect("写文件");
        // del 不在白名单，应被拦截
        let 结果 = 执行器.运行命令("del a.txt");
        assert!(结果.is_err());
        assert!(结果.expect_err("应拦截").to_string().contains("危险命令"));
        assert_eq!(执行器.读文件("a.txt").expect("读"), "重要内容");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 白名单内命令不被拦截() {
        let 根 = 准备工作区("误伤");
        let 执行器 = 本地执行器::new(&根);
        // echo 在白名单内，应正常执行
        let 输出 = 执行器.运行命令("echo model").expect("白名单内命令不应被拦截");
        assert!(输出.contains("model"));
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 转义符命令被拦截() {
        let 根 = 准备工作区("转义绕过");
        let 执行器 = 本地执行器::new(&根);
        // d^el 含转义符，应被拦截（防止 cmd 转义绕过白名单）
        let 结果 = 执行器.运行命令("d^el a.txt");
        assert!(结果.is_err());
        assert!(结果.expect_err("应拦截").to_string().contains("危险命令"));
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 脚本扩展名命令被拦截() {
        let 根 = 准备工作区("脚本绕过");
        let 执行器 = 本地执行器::new(&根);
        // .bat 扩展名应被拦截（防止通过脚本间接执行白名单外命令）
        let 结果 = 执行器.运行命令("恶意.bat");
        assert!(结果.is_err());
        assert!(结果.expect_err("应拦截").to_string().contains("危险命令"));
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 命令连接符分割检查() {
        let 根 = 准备工作区("连接符");
        let 执行器 = 本地执行器::new(&根);
        // cargo 在白名单但 del 不在，&& 连接应被拦截
        let 结果 = 执行器.运行命令("echo hello && del a.txt");
        assert!(结果.is_err());
        assert!(结果.expect_err("应拦截").to_string().contains("危险命令"));
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 列目录_区分目录与文件并按名排序() {
        let 根 = 准备工作区("列目录");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("b.txt", "内容").expect("写文件");
        执行器.写文件("a.txt", "内容").expect("写文件");
        执行器.写文件("子/c.txt", "内容").expect("写子目录文件");
        let 输出 = 执行器.列目录("").expect("列目录应成功");
        assert!(输出.contains("[文件] a.txt"), "应列出 a.txt，实际: {输出}");
        assert!(输出.contains("[文件] b.txt"), "应列出 b.txt，实际: {输出}");
        assert!(输出.contains("[目录] 子"), "应列出子目录，实际: {输出}");
        assert!(!输出.contains("c.txt"), "列目录应只列一层，不递归");
        let a位置 = 输出.find("a.txt").expect("有 a.txt");
        let b位置 = 输出.find("b.txt").expect("有 b.txt");
        assert!(a位置 < b位置, "应按名排序，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 列目录_空目录返回提示() {
        let 根 = 准备工作区("空目录");
        let 执行器 = 本地执行器::new(&根);
        let 输出 = 执行器.列目录("").expect("列目录应成功");
        assert!(输出.contains("空目录"), "空目录应提示，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 列目录_越界路径报错() {
        let 根 = 准备工作区("列目录越界");
        let 执行器 = 本地执行器::new(&根);
        assert!(执行器.列目录("../").is_err());
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 按名找文件_递归匹配返回相对路径() {
        let 根 = 准备工作区("glob");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("src/主程序.rs", "内容").expect("写嵌套文件");
        执行器.写文件("根文件.rs", "内容").expect("写根文件");
        执行器.写文件("src/备注.txt", "内容").expect("写非匹配文件");
        let 输出 = 执行器.按名找文件("**/*.rs").expect("glob 应成功");
        assert!(输出.contains("根文件.rs"), "应匹配根文件，实际: {输出}");
        assert!(输出.contains("主程序.rs"), "应递归匹配嵌套文件，实际: {输出}");
        assert!(!输出.contains("备注.txt"), "不应匹配 .txt，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 按名找文件_无匹配返回提示() {
        let 根 = 准备工作区("glob空");
        let 执行器 = 本地执行器::new(&根);
        let 输出 = 执行器.按名找文件("*.rs").expect("glob 应成功");
        assert!(输出.contains("无匹配"), "无匹配应提示，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 搜索内容_返回路径行号内容() {
        let 根 = 准备工作区("grep");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("a.txt", "第一行无关\n第二行含关键词测试\n").expect("写文件");
        let 输出 = 执行器.搜索内容("关键词").expect("搜索应成功");
        assert!(输出.contains("a.txt:2:"), "应返回 路径:行号:内容，实际: {输出}");
        assert!(输出.contains("第二行含关键词测试"), "应返回匹配行内容，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 搜索内容_跳过二进制文件() {
        let 根 = 准备工作区("grep二进制");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("文本.txt", "含关键词的文本行").expect("写文本");
        let 二进制路径 = std::path::Path::new(&根).join("数据.bin");
        let mut 字节 = "含关键词的二进制".as_bytes().to_vec();
        字节.push(0);
        std::fs::write(&二进制路径, 字节).expect("写二进制");
        let 输出 = 执行器.搜索内容("关键词").expect("搜索应成功");
        assert!(输出.contains("文本.txt"), "应匹配文本文件，实际: {输出}");
        assert!(!输出.contains("数据.bin"), "应跳过含 NUL 的二进制文件，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 搜索内容_无匹配返回提示() {
        let 根 = 准备工作区("grep空");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("a.txt", "无关键内容").expect("写文件");
        let 输出 = 执行器.搜索内容("不存在词").expect("搜索应成功");
        assert!(输出.contains("无匹配"), "无匹配应提示，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 精确编辑_唯一匹配替换成功并备份() {
        let 根 = 准备工作区("编辑");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("a.txt", "版本=1.0.0").expect("写文件");
        let 输出 = 执行器.精确编辑("a.txt", "1.0.0", "1.1.0").expect("编辑应成功");
        assert!(输出.contains("替换成功"), "应返回成功提示，实际: {输出}");
        assert_eq!(执行器.读文件("a.txt").expect("读"), "版本=1.1.0");
        assert_eq!(执行器.读文件("a.txt.bak").expect("读备份"), "版本=1.0.0");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 精确编辑_多处匹配报错() {
        let 根 = 准备工作区("编辑多处");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("a.txt", "旧 旧 旧").expect("写文件");
        let 结果 = 执行器.精确编辑("a.txt", "旧", "新");
        assert!(结果.is_err());
        assert!(结果.expect_err("应报错").to_string().contains("更精确"), "应提示提供更精确上下文");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 精确编辑_旧串缺失报错() {
        let 根 = 准备工作区("编辑缺失");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("a.txt", "内容").expect("写文件");
        let 结果 = 执行器.精确编辑("a.txt", "不存在", "新");
        assert!(结果.is_err());
        assert!(结果.expect_err("应报错").to_string().contains("未找到"), "应提示未找到");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 精确编辑_越界路径报错() {
        let 根 = 准备工作区("编辑越界");
        let 执行器 = 本地执行器::new(&根);
        assert!(执行器.精确编辑("../a.txt", "旧", "新").is_err());
        let _ = std::fs::remove_dir_all(&根);
    }
}