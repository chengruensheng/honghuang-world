//! 进程-驱动-园 · 跑命令测试：验证外部进程驱动的退出码、错误码与超时强杀。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::{运行命令, 运行命令超时};

    #[test]
    fn 运行命令取退出码与输出() {
        let 结果 = 运行命令("cargo", &["--version"], None).unwrap();
        assert_eq!(结果.退出码, Some(0));
        assert_eq!(结果.错误码, "OK");
        assert!(!结果.标准输出.is_empty());
    }

    #[test]
    fn 运行不存在的命令报错() {
        assert!(运行命令("不存在的命令_xyz", &[], None).is_err());
    }

    #[test]
    fn 运行命令超时_短命令正常返回() {
        // 10 秒超时，cargo --version 应秒级完成。
        let 结果 = 运行命令超时("cargo", &["--version"], None, 10_000).unwrap();
        assert_eq!(结果.退出码, Some(0));
        assert_eq!(结果.错误码, "OK");
        assert!(!结果.标准输出.is_empty());
    }

    #[test]
    fn 运行命令超时_长命令被强杀返回错误() {
        // 1 秒超时，命令真等 30 秒：必触发超时强杀。
        let 错 = 运行命令超时(
            "powershell.exe",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            None,
            1_000,
        )
        .unwrap_err();
        assert!(错.contains("超时"), "应返回超时错误：{错}");
        assert!(错.contains("强杀"), "应说明子进程已被强杀：{错}");
    }

    #[test]
    fn 运行命令_默认超时仍生效() {
        // 默认超时为 10 分钟，正常 cargo --version 应能完成（不走超时分支）。
        let 结果 = 运行命令("cargo", &["--version"], None).unwrap();
        assert_eq!(结果.退出码, Some(0));
        assert_eq!(结果.错误码, "OK");
    }
}
