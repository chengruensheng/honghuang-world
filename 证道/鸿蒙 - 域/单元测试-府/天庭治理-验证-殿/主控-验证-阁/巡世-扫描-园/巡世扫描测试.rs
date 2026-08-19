//! 巡世 - 扫描 - 园 · 巡世扫描测试：扫描世界，产出巡世报告。

#[cfg(test)]
mod 测试 {
    use std::fs;
    use tianting_fu::扫描世界;

    /// 唯一临时目录：按测试名 + 进程号 + 当前毫秒隔离，避免并发撞目录。
    fn 临时目录(测试名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "巡世测试-{测试名}-{}-{}",
            std::process::id(),
            shihai_fu::当前毫秒()
        ))
    }

    #[test]
    fn 扫描产出报告() {
        let 目录 = std::env::temp_dir().join("识海测试-巡世");
        fs::create_dir_all(&目录).unwrap();
        fs::write(目录.join("a.rs"), "x").unwrap();
        let 报告 = 扫描世界(&目录);
        assert!(报告.候选.is_empty()); // 文件数少，无候选
        let _ = fs::remove_dir_all(&目录);
    }

    /// ① 园无测试检测：园下 .rs 全无测试标记 → 产候选，目标含「为{园名}生产模块追加单元测试」。
    #[test]
    fn 园无测试检测_无测试的园产候选() {
        let 根 = 临时目录("园无测试-无测试");
        let 园路径 = 根.join("示例-园");
        fs::create_dir_all(&园路径).unwrap();
        fs::write(
            园路径.join("模块.rs"),
            "pub fn 加(a: i32, b: i32) -> i32 { a + b }",
        )
        .unwrap();
        let 报告 = 扫描世界(&根);
        let 命中 = 报告
            .候选
            .iter()
            .any(|候选| 候选.目标.contains("为示例-园生产模块追加单元测试"));
        assert!(命中, "无测试的园应产候选，实际候选: {:?}", 报告.候选);
        let _ = fs::remove_dir_all(&根);
    }

    /// ① 园无测试检测：园下 .rs 含 `#[test]` → 不产候选。
    #[test]
    fn 园无测试检测_有测试的园不产候选() {
        let 根 = 临时目录("园无测试-有测试");
        let 园路径 = 根.join("示例-园");
        fs::create_dir_all(&园路径).unwrap();
        fs::write(
            园路径.join("模块.rs"),
            "pub fn 加(a: i32, b: i32) -> i32 { a + b }\n#[test]\nfn 检查加() { assert_eq!(加(1,2), 3); }",
        )
        .unwrap();
        let 报告 = 扫描世界(&根);
        let 命中 = 报告
            .候选
            .iter()
            .any(|候选| 候选.目标.contains("为示例-园生产模块追加单元测试"));
        assert!(!命中, "有测试的园不应产候选，实际候选: {:?}", 报告.候选);
        let _ = fs::remove_dir_all(&根);
    }

    /// ③ 教训重复模式检测：同前缀（前 40 字符）≥3 条 → 产候选，目标含「反复出现」。
    #[test]
    fn 教训重复检测_同前缀三条产候选() {
        let 根 = 临时目录("教训重复-同前缀");
        // 写 3 条同前缀教训记录到 .上下文/格位/教训.jsonl
        let 格位目录 = 根.join(".上下文").join("格位");
        let 存储 = shihai_fu::模型存储::打开(&格位目录);
        // 40 字符固定前缀（甲乙丙丁戊己庚辛壬癸 × 4）
        let 前缀 =
            "甲乙丙丁戊己庚辛壬癸甲乙丙丁戊己庚辛壬癸甲乙丙丁戊己庚辛壬癸甲乙丙丁戊己庚辛壬癸";
        assert_eq!(前缀.chars().count(), 40);
        for 后缀 in ["一", "二", "三"] {
            let 内容 = format!("{前缀}{后缀}");
            let 记录 = shihai_fu::记录::新(shihai_fu::教训格位, &内容, "测试证据", "测试");
            存储.写记录(&记录).unwrap();
        }
        let 报告 = 扫描世界(&根);
        let 命中 = 报告.候选.iter().any(|候选| 候选.目标.contains("反复出现"));
        assert!(命中, "同前缀三条应产重复候选，实际候选: {:?}", 报告.候选);
        let _ = fs::remove_dir_all(&根);
    }

    /// ③ 教训重复模式检测：不同前缀各 1 条（<3）→ 不产候选。
    #[test]
    fn 教训重复检测_不同前缀不产候选() {
        let 根 = 临时目录("教训重复-不同前缀");
        let 格位目录 = 根.join(".上下文").join("格位");
        let 存储 = shihai_fu::模型存储::打开(&格位目录);
        // 两条不同前缀（前 40 字符不同）
        let 内容一 =
            "甲乙丙丁戊己庚辛壬癸甲乙丙丁戊己庚辛壬癸甲乙丙丁戊己庚辛壬癸甲乙丙丁戊己庚辛壬癸一";
        let 内容二 =
            "子丑寅卯辰巳午未申酉子丑寅卯辰巳午未申酉子丑寅卯辰巳午未申酉子丑寅卯辰巳午未申酉二";
        for 内容 in [内容一, 内容二] {
            let 记录 = shihai_fu::记录::新(shihai_fu::教训格位, 内容, "测试证据", "测试");
            存储.写记录(&记录).unwrap();
        }
        let 报告 = 扫描世界(&根);
        let 命中 = 报告.候选.iter().any(|候选| 候选.目标.contains("反复出现"));
        assert!(!命中, "不同前缀不应产重复候选，实际候选: {:?}", 报告.候选);
        let _ = fs::remove_dir_all(&根);
    }

    /// ④ 规模启发：源文件数 > 200 → 产候选，目标含「项目规模较大」。
    #[test]
    fn 规模启发_超两百产候选() {
        let 根 = 临时目录("规模启发-超两百");
        fs::create_dir_all(&根).unwrap();
        // 创建 201 个 .rs 文件（直接放根下，不进任何园目录，避免触发园无测试检测）
        for 序号 in 0..201 {
            fs::write(根.join(format!("文件{序号}.rs")), "// 占位").unwrap();
        }
        let 报告 = 扫描世界(&根);
        let 命中 = 报告
            .候选
            .iter()
            .any(|候选| 候选.目标.contains("项目规模较大"));
        assert!(命中, "201 个源文件应产规模候选，实际候选: {:?}", 报告.候选);
        let _ = fs::remove_dir_all(&根);
    }
}
