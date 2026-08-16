//! 巡世 - 扫描 - 园 · 巡世扫描测试：扫描世界，产出巡世报告。

#[cfg(test)]
mod 测试 {
    use std::fs;
    use tianting_fu::扫描世界;

    #[test]
    fn 扫描产出报告() {
        let 目录 = std::env::temp_dir().join("识海测试-巡世");
        fs::create_dir_all(&目录).unwrap();
        fs::write(目录.join("a.rs"), "x").unwrap();
        let 报告 = 扫描世界(&目录);
        assert!(报告.候选.is_empty()); // 文件数少，无候选
        let _ = fs::remove_dir_all(&目录);
    }
}
