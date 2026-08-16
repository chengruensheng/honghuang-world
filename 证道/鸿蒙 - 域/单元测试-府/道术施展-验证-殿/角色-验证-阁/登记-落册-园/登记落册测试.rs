//! 登记-落册-园 · 登记落册测试：验证角色卡登记落册与取用。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::{执行角色, 角色册};

    fn 造角色(身份: &str) -> 执行角色 {
        执行角色 {
            身份: 身份.to_string(),
            道: "代码".to_string(),
            职司: "实现".to_string(),
            模型池: "executor".to_string(),
            契约: "写代码".to_string(),
        }
    }

    #[test]
    fn 登记与取用() {
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝"));
        assert_eq!(册.数量(), 1);
        assert_eq!(册.取("多宝").unwrap().道, "代码");
    }
}
