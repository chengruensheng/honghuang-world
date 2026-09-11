#[cfg(test)]
mod tests {
    use hm_contract::Component;
    use hm_execute::{本地执行器, 规范化工作区};
    use hm_execute_contract::执行器;
    use std::path::PathBuf;

    fn 准备工作区(名: &str) -> String {
        // 路径带进程号：避免并行/重入运行同一测试二进制时同名工作区互相踩踏（清目录/写文件竞态）
        let 根 = std::env::temp_dir().join(format!("zd_execute_test_{}_{名}", std::process::id()));
        let 根字符串 = 根.to_string_lossy().into_owned();
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).expect("创建工作区目录");
        根字符串
    }

    #[test]
    fn 规范化工作区_相对路径转绝对并剥点组件() {
        // 生产配置形态 dev_workspace="./工作区"：根若带前导 CurDir 组件，
        // 按名找文件的 strip_prefix 与 glob 规范化路径失配 → 感知工具全瞎（2026-09-09 实测缺陷）
        let 规范 = 规范化工作区(PathBuf::from("./某工作区"));
        assert!(规范.is_absolute(), "相对工作区应锚定 cwd 转绝对，实际: {}", 规范.display());
        assert!(
            !规范.components().any(|c| matches!(c, std::path::Component::CurDir)),
            "规范化后不得残留 . 组件，实际: {}",
            规范.display()
        );
    }

    #[test]
    fn 规范化工作区_绝对路径保持不变() {
        let 根 = PathBuf::from(准备工作区("规范化绝对"));
        let 规范 = 规范化工作区(根.clone());
        assert_eq!(
            规范.components().collect::<Vec<_>>(),
            根.components().collect::<Vec<_>>(),
            "绝对路径规范化应保持组件一致"
        );
        let _ = std::fs::remove_dir_all(&根);
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
    fn 删除文件_走回收站且名册记录() {
        // 删除防护四步闭环（执行器入口）：预审通过 → 移入 .回收站 → 复核真实移除 → 名册留痕
        let 根 = 准备工作区("删除闭环");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("产物/输出.tmp2", "中间产物").expect("写产物");
        let 结果 = 执行器.删除文件("产物/输出.tmp2").expect("删除应成功");
        assert!(结果.contains(".回收站"), "结果应说明回收站落点: {结果}");
        let 根路径 = PathBuf::from(&根);
        assert!(!根路径.join("产物").join("输出.tmp2").exists(), "原路径应已真实移除（复核）");
        assert!(根路径.join(".回收站").join("产物").join("输出.tmp2").exists(), "回收站应有落点可恢复");
        let 名册 = std::fs::read_to_string(根路径.join(".回收站").join("删除名册.jsonl")).expect("名册应存在");
        assert!(名册.contains("产物/输出.tmp2"), "名册应记录被删路径: {名册}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 删除文件_回收站与名册受保护() {
        let 根 = 准备工作区("删除保护");
        let 执行器 = 本地执行器::new(&根);
        assert!(执行器.删除文件(".回收站/删除名册.jsonl").is_err(), "删除名册禁止删除");
        assert!(执行器.删除文件(".回收站/内含.bak").is_err(), "回收站内文件禁止删除");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 感知扫描_回收站内容不当残留() {
        // 回归：回收站里的已删除文件被 glob 扫出 → 清理核验门永远驳回 → 清理死循环
        let 根 = 准备工作区("感知排除");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("产物/输出.tmp2", "中间产物").expect("写产物");
        执行器.删除文件("产物/输出.tmp2").expect("安全删除入回收站");
        let 根路径 = PathBuf::from(&根);
        std::fs::create_dir_all(根路径.join(".回收站").join("旧目录")).expect("建回收站结构");
        std::fs::write(根路径.join(".回收站").join("旧目录").join("遗留.bak"), "x").expect("写遗留");
        let 扫描 = 执行器.按名找文件("**/*.bak").expect("扫描应成功");
        assert_eq!(扫描, "（无匹配）", "回收站内容不得被当工作区残留: {扫描}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 感知扫描_搜索内容跳过回收站() {
        // 回收站内是已删除文件：内容检索同样排除，避免搜出已删内容误导模型
        let 根 = 准备工作区("搜索排除");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("可见.txt", "标记词在这里").expect("写可见");
        执行器.写文件("将删.txt", "标记词也在这里").expect("写将删");
        执行器.删除文件("将删.txt").expect("安全删除入回收站");
        let 输出 = 执行器.搜索内容("标记词").expect("搜索应成功");
        assert!(输出.contains("可见.txt"), "可见文件应命中: {输出}");
        assert!(!输出.contains("将删.txt"), "回收站内文件不得命中: {输出}");
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
    fn 白名单_重定向合并符不误拦() {
        let 根 = 准备工作区("重定向误伤");
        let 执行器 = 本地执行器::new(&根);
        // 2>&1 是重定向而非命令分隔符，cargo/echo 在白名单内，应正常执行而非把 1 当独立命令误拦
        let 输出 = 执行器.运行命令("echo hello 2>&1").expect("重定向不应导致误拦");
        assert!(输出.contains("hello"), "应正常输出，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 白名单_逻辑与连接均白名单命令不拦截() {
        let 根 = 准备工作区("连接不误伤");
        let 执行器 = 本地执行器::new(&根);
        // 两条命令都在白名单内，&& 连接应整体放行
        let 输出 = 执行器.运行命令("echo 甲 && echo 乙").expect("白名单内 && 不应被拦截");
        assert!(输出.contains("甲") && 输出.contains("乙"), "应输出两条，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 白名单_管道双方白名单不拦截() {
        let 根 = 准备工作区("管道");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("数据.txt", "alpha\nbeta\nalpha").expect("写文件");
        // findstr 在白名单内，管道两侧均合法应放行
        let 输出 = 执行器.运行命令("type 数据.txt | findstr alpha").expect("管道不应被拦截");
        assert!(输出.contains("alpha"), "应过滤出 alpha，实际: {输出}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 删除文件_存在文件_删除成功() {
        // 删除语义已升级为安全删除：文件移入 .回收站（可恢复），原路径真实移除
        let 根 = 准备工作区("删除");
        let 执行器 = 本地执行器::new(&根);
        执行器.写文件("残留.bak", "旧备份").expect("写备份");
        let 输出 = 执行器.删除文件("残留.bak").expect("删除应成功");
        assert!(输出.contains("已安全删除"), "应返回安全删除成功，实际: {输出}");
        assert!(执行器.读文件("残留.bak").is_err(), "原路径文件应已不存在");
        assert!(
            PathBuf::from(&根).join(".回收站").join("残留.bak").exists(),
            "回收站应有落点可恢复"
        );
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 删除文件_不存在文件_返回错误() {
        let 根 = 准备工作区("删除不存在");
        let 执行器 = 本地执行器::new(&根);
        let 结果 = 执行器.删除文件("不存在的文件.bak");
        assert!(结果.is_err(), "删除不存在文件应报错");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 删除文件_越界路径_拒绝() {
        let 根 = 准备工作区("删除越界");
        let 执行器 = 本地执行器::new(&根);
        let 结果 = 执行器.删除文件("../越界.bak");
        assert!(结果.is_err(), "越界路径应被拒绝");
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