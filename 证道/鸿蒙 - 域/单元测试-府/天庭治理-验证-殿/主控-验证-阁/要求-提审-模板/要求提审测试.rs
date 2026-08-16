//! 要求 - 提审 - 模板 · 要求提审测试：解析想法提示词模板渲染。

#[cfg(test)]
mod 测试 {
    use tianting_fu::渲染要求提审提示;

    #[test]
    fn 渲染后含背景与想法() {
        let 提示 = 渲染要求提审提示("项目记忆", "做一个功能");
        assert!(提示.contains("项目记忆"));
        assert!(提示.contains("做一个功能"));
        assert!(!提示.contains("{背景}"));
    }
}
