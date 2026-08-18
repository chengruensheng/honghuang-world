//! 命令-沙箱-园 · 沙箱护栏测试：验证命令进沙箱前的最后拦截与超时边界。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::{最大超时毫秒, 校验命令护栏};

    #[test]
    fn 拦截进程终止与自身二进制() {
        assert!(校验命令护栏("taskkill", &["/F", "/IM", "号令.exe"], None)
            .unwrap_err()
            .contains("护栏拦截"));
        assert!(校验命令护栏(
            "powershell.exe",
            &["-Command", "Stop-Process -Name 号令"],
            None
        )
        .unwrap_err()
        .contains("护栏拦截"));
        assert!(校验命令护栏(
            "cargo",
            &["run", "--bin", "号令", "--", "世界", "时间"],
            None
        )
        .unwrap_err()
        .contains("护栏拦截"));
        assert!(校验命令护栏("cargo", &["build", "--bin", "号令"], None)
            .unwrap_err()
            .contains("护栏拦截"));
        assert!(
            校验命令护栏("cmd.exe", &["/c", "号令.exe", "世界", "时间"], None)
                .unwrap_err()
                .contains("护栏拦截")
        );
        assert!(校验命令护栏("Get-Process", &["-Name", "号令"], None)
            .unwrap_err()
            .contains("护栏拦截"));
    }

    #[test]
    fn 放行编译类命令() {
        assert!(校验命令护栏("cargo", &["build", "--workspace", "--lib"], None).is_ok());
        assert!(校验命令护栏("cargo", &["test"], None).is_ok());
        assert!(校验命令护栏("cmd.exe", &["/c", "echo", "洪荒"], None).is_ok());
    }

    #[test]
    fn 校验超时上限拒绝() {
        // 超过最大上限被拒。
        let 错 = 校验命令护栏("cargo", &["build"], Some(最大超时毫秒 + 1)).unwrap_err();
        assert!(
            错.contains("超时") && 错.contains("超过最大上限"),
            "应拒超上限：{错}"
        );
    }

    #[test]
    fn 校验零超时拒绝() {
        // 0 毫秒无意义，必须 > 0。
        let 错 = 校验命令护栏("cargo", &["build"], Some(0)).unwrap_err();
        assert!(错.contains("超时毫秒为 0"), "应拒 0 超时：{错}");
    }

    #[test]
    fn 校验合法超时通过() {
        // 边界值与中间值都应放行。
        assert!(校验命令护栏("cargo", &["build"], Some(1)).is_ok());
        assert!(校验命令护栏("cargo", &["build"], Some(最大超时毫秒)).is_ok());
        assert!(校验命令护栏("cargo", &["build"], Some(60_000)).is_ok());
    }

    #[test]
    fn 校验无超时由调用方兜底() {
        // None 不拦截；由调用方自行兜底为 默认超时毫秒。
        assert!(校验命令护栏("cargo", &["build"], None).is_ok());
    }
}
