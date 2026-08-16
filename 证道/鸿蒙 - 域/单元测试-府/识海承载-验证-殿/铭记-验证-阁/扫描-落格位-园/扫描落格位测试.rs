//! 扫描-落格位-园 · 测试：扫描 use / pub use 依赖边，提取符号与波及。

#[cfg(test)]
mod 测试 {
    use shihai_fu::扫描依赖边;
    use std::path::{Path, PathBuf};

    fn 造临时项目() -> PathBuf {
        let 根 = std::env::temp_dir().join(format!("依赖图测试-{}", shihai_fu::当前毫秒()));
        let 府 = 根.join("道术施展-府");
        std::fs::create_dir_all(&府).unwrap();
        std::fs::write(府.join("Cargo.toml"), "[package]\nname = \"道术施展-府\"\n").unwrap();
        std::fs::write(
            府.join("读取.rs"),
            "/// 读取文件内容。\npub fn 读文件(路径: &str) -> Result<String, String> {\n    std::fs::read_to_string(路径)\n}\n",
        )
        .unwrap();
        std::fs::write(
            府.join("调用.rs"),
            "use crate::读文件;\n\npub fn 调用读文件() -> String {\n    读文件(\"x\").unwrap_or_default()\n}\n",
        )
        .unwrap();
        根
    }

    fn 清理(根: &Path) {
        let _ = std::fs::remove_dir_all(根);
    }

    #[test]
    fn 提取符号与波及() {
        let 根 = 造临时项目();
        let 图 = 扫描依赖边(&根).unwrap();
        let 档案们 = 图.查符号("读文件");
        assert_eq!(档案们.len(), 1);
        let 档案 = &档案们[0];
        assert!(档案.文件.contains("读取.rs"));
        assert_eq!(档案.模块, "道术施展-府");
        assert!(档案.波及.iter().any(|路径| 路径.contains("调用.rs")));
        清理(&根);
    }

    #[test]
    fn 结构下探命中与回退() {
        let 根 = std::env::temp_dir().join(format!("结构下探测试-{}", shihai_fu::当前毫秒()));
        let 府 = 根.join("鸿蒙/基础设施 - 域/道术施展-府");
        let 园 = 府.join("任务-调度-殿/任务-派遣-阁/派发-落单-园");
        std::fs::create_dir_all(&园).unwrap();
        std::fs::write(府.join("Cargo.toml"), "[package]\nname = \"道术施展-府\"\n").unwrap();
        std::fs::write(园.join("派发落单.rs"), "pub fn 派遣() {}").unwrap();

        let 图 = 扫描依赖边(&根).unwrap();
        // 命中府名 → 只下探该府的殿/阁/园
        let 下探 = 图.下探(&["道术施展-府".to_string()]);
        assert!(下探.contains("道术施展-府"));
        assert!(下探.contains("任务-调度-殿"));
        assert!(下探.contains("任务-派遣-阁"));
        assert!(下探.contains("派发-落单-园"));
        // 未命中府名 → 回退渲染全部府
        let 回退 = 图.下探(&["不存在-府".to_string()]);
        assert!(回退.contains("道术施展-府"));

        let _ = std::fs::remove_dir_all(&根);
    }
}
