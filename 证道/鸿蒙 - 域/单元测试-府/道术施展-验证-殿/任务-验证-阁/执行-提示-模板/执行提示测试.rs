//! 执行-提示-模板 · 执行提示测试：验证落盘 / 读现状 / 续跑提示渲染。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::{渲染落盘提示, 渲染读现状提示, 渲染续跑提示};

    #[test]
    fn 落盘提示含背景现状目标预算() {
        let 提示 = 渲染落盘提示("背景", "现状", "目标", 24);
        assert!(提示.contains("背景"));
        assert!(提示.contains("现状"));
        assert!(提示.contains("目标"));
        assert!(提示.contains("24 轮"));
        assert!(!提示.contains("{背景}"));
        assert!(!提示.contains("{预算}"));
    }

    #[test]
    fn 读现状提示含背景目标() {
        let 提示 = 渲染读现状提示("背景", "目标");
        assert!(提示.contains("背景"));
        assert!(提示.contains("目标"));
        assert!(!提示.contains("{目标}"));
    }

    #[test]
    fn 续跑提示含报错() {
        let 提示 = 渲染续跑提示("编译错误");
        assert!(提示.contains("编译错误"));
        assert!(!提示.contains("{报错}"));
    }
}
