//! 依赖-边-园 · 测试：依赖图查询与落盘加载。

#[cfg(test)]
mod 测试 {
    use shihai_fu::{依赖图, 符号档案};

    fn 造图() -> 依赖图 {
        let mut 图 = 依赖图::default();
        let mut 甲 = 符号档案::新(
            "世界",
            "道术施展-府",
            "道术施展-府/读取.rs",
            "读文件",
            "fn 读文件",
            "读取文件内容",
            "",
        );
        甲.波及.push("道术施展-府/调用.rs".to_string());
        图.档案们.push(甲);
        图.档案们.push(符号档案::新(
            "世界",
            "道术施展-府",
            "道术施展-府/调用.rs",
            "调用读文件",
            "fn 调用读文件",
            "",
            "",
        ));
        图
    }

    #[test]
    fn 查符号命中() {
        let 图 = 造图();
        assert_eq!(图.查符号("读文件").len(), 1);
        assert!(图.查符号("不存在").is_empty());
    }

    #[test]
    fn 查涉及文件含波及() {
        let 图 = 造图();
        let 相关 = 图.查涉及文件(&["读文件".to_string()]);
        assert!(相关.iter().any(|文件| 文件.contains("读取.rs")));
        assert!(相关.iter().any(|文件| 文件.contains("调用.rs")));
    }

    #[test]
    fn 保存加载一致() {
        let 图 = 造图();
        let 路径 = std::env::temp_dir().join("识海测试-依赖图.json");
        图.保存(路径.to_str().unwrap()).unwrap();
        let 读回 = 依赖图::加载(路径.to_str().unwrap()).unwrap();
        assert_eq!(读回.档案们.len(), 图.档案们.len());
        let _ = std::fs::remove_file(&路径);
    }
}
