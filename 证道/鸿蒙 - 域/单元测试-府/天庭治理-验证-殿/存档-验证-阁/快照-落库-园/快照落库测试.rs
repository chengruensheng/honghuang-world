//! 快照 - 落库 - 园 · 快照落库测试：源码快照排除构建物。

#[cfg(test)]
mod 测试 {
    use std::fs;
    use tianting_fu::源码快照;

    #[test]
    fn 快照排除构建物() {
        let 源 = std::env::temp_dir().join("识海测试-快照源");
        let 目标 = std::env::temp_dir().join("识海测试-快照目标");
        fs::create_dir_all(源.join("target")).unwrap();
        fs::write(源.join("a.rs"), "x").unwrap();
        fs::write(源.join("target/b.rs"), "y").unwrap();
        let 数 = 源码快照(&源, &目标).unwrap();
        assert_eq!(数, 1); // 只复制 a.rs，跳过 target
        let _ = fs::remove_dir_all(&源);
        let _ = fs::remove_dir_all(&目标);
    }
}
