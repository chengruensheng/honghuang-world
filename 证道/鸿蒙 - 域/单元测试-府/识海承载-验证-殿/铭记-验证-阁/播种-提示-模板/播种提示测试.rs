//! 播种-提示-模板 · 测试：播种提示词模板与渲染。

#[cfg(test)]
mod 测试 {
    use shihai_fu::渲染播种提示;

    #[test]
    fn 渲染后不含占位符() {
        let 提示 = 渲染播种提示("架构原则", "素材内容", "a.rs");
        assert!(提示.contains("架构原则"));
        assert!(提示.contains("素材内容"));
        assert!(提示.contains("a.rs"));
        assert!(!提示.contains("{种子}"));
        assert!(!提示.contains("{素材}"));
    }

    #[test]
    fn 印证为空填无() {
        let 提示 = 渲染播种提示("架构原则", "素材内容", "");
        assert!(提示.contains("（无）"));
    }
}
