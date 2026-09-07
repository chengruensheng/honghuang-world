#[cfg(test)]
mod tests {
    use tc_task::{漂移类型, 漂移检测, 扫尾检查, 解析快照};

    fn 串集(项: &[&str]) -> Vec<String> {
        项.iter().map(|s| s.to_string()).collect()
    }

    /// 测试1：声明全落地、无多余 → 零漂移通过
    #[test]
    fn 漂移检测_声明全落地无多余零漂移() {
        let 声明 = 串集(&["src/main.rs", "src/lib.rs"]);
        let 实际 = 串集(&["src/lib.rs", "src/main.rs"]);
        let 报告 = 漂移检测(&声明, &实际);
        assert_eq!(报告.声明数, 2);
        assert_eq!(报告.实际数, 2);
        assert_eq!(报告.兑现数, 2);
        assert_eq!(报告.未兑现数, 0);
        assert_eq!(报告.多余数, 0);
        assert!(报告.零漂移());
        assert_eq!(报告.漂移项.len(), 0);
        let 结论 = 扫尾检查(1, 2, true, &报告);
        assert!(结论.通过);
        assert!(结论.说明.is_empty());
    }

    /// 测试2：声明含不存在文件 → 未兑现漂移项
    #[test]
    fn 漂移检测_声明含未落地文件检出未兑现() {
        let 声明 = 串集(&["src/main.rs", "docs/设计.md"]);
        let 实际 = 串集(&["src/main.rs"]);
        let 报告 = 漂移检测(&声明, &实际);
        assert_eq!(报告.未兑现数, 1);
        assert_eq!(报告.多余数, 0);
        assert!(!报告.零漂移());
        let 项 = 报告.漂移项.iter().find(|i| i.路径 == "docs/设计.md").unwrap();
        assert_eq!(项.类型, 漂移类型::声明未兑现);
        let 结论 = 扫尾检查(2, 2, true, &报告);
        assert!(!结论.通过);
        assert_eq!(结论.未兑现数, 1);
        assert!(结论.说明.contains("声明未兑现"));
    }

    /// 测试3：工作区有未声明文件 → 多余新增漂移项
    #[test]
    fn 漂移检测_工作区未声明文件检出多余新增() {
        let 声明 = 串集(&["src/main.rs"]);
        let 实际 = 串集(&["src/main.rs", "secret.txt", "target/debug/tmp"]);
        let 报告 = 漂移检测(&声明, &实际);
        // target/ 被忽略；secret.txt 为多余
        assert_eq!(报告.实际数, 2);
        assert_eq!(报告.多余数, 1);
        let 项 = 报告.漂移项.iter().find(|i| i.路径 == "secret.txt").unwrap();
        assert_eq!(项.类型, 漂移类型::未声明新增);
        let 结论 = 扫尾检查(3, 1, true, &报告);
        assert!(!结论.通过);
        assert_eq!(结论.多余数, 1);
    }

    /// 测试4：空声明 + 空实际 → 零漂移通过（空集边界）
    #[test]
    fn 漂移检测_空声明空实际零漂移() {
        let 声明 = Vec::new();
        let 实际 = Vec::new();
        let 报告 = 漂移检测(&声明, &实际);
        assert_eq!(报告.声明数, 0);
        assert_eq!(报告.实际数, 0);
        assert!(报告.零漂移());
        assert!(报告.漂移项.is_empty());
        let 结论 = 扫尾检查(4, 0, true, &报告);
        assert!(结论.通过);
        assert!(结论.说明.is_empty());
    }

    /// 测试5：声明重复路径去重
    #[test]
    fn 漂移检测_声明重复路径去重() {
        let 声明 = 串集(&["src/main.rs", "src/main.rs", "src/lib.rs"]);
        let 实际 = 串集(&["src/main.rs", "src/lib.rs"]);
        let 报告 = 漂移检测(&声明, &实际);
        assert_eq!(报告.声明数, 2, "重复声明应去重");
        assert_eq!(报告.兑现数, 2);
        assert_eq!(报告.未兑现数, 0);
        assert_eq!(报告.漂移项.len(), 0);
    }

    /// 测试6：自检不通过 → 扫尾不通过且说明含「自检未通过」
    #[test]
    fn 扫尾检查_自检不通过则不通过() {
        let 声明 = 串集(&["src/main.rs"]);
        let 实际 = 串集(&["src/main.rs"]);
        let 报告 = 漂移检测(&声明, &实际);
        assert!(报告.零漂移());
        let 结论 = 扫尾检查(6, 1, false, &报告);
        assert!(!结论.通过, "自检不通过应判不通过");
        assert!(结论.说明.contains("自检未通过"));
    }

    /// 测试7：快照解析——归一化分隔符、剔除 target/ 与（无匹配）、去空
    #[test]
    fn 解析快照_归一化并剔除系统目录() {
        let 快照 = "src\\main.rs\nsrc/lib.rs\n（无匹配）\n\ntarget/debug/x.exe\n.git/config";
        let 路径们 = 解析快照(快照);
        assert_eq!(路径们, vec!["src/lib.rs".to_string(), "src/main.rs".to_string()]);
    }
}