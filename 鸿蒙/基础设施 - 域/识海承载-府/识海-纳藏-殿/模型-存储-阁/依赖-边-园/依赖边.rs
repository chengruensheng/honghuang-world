//! 依赖 - 边 - 园：依赖图落盘 / 加载。

use crate::{依赖图, 工作区};
use rizhi_fu::debug;
use std::path::Path;

impl 依赖图 {
    /// 保存在工作区（.上下文/依赖图.json）。
    pub fn 保存在工作区(&self, 工作区: &工作区) -> Result<(), String> {
        self.保存(工作区.依赖图路径())
    }

    /// 保存到指定路径。
    pub fn 保存(&self, 路径: impl AsRef<Path>) -> Result<(), String> {
        let 文本 = serde_json::to_string_pretty(self)
            .map_err(|错误| format!("序列化依赖图失败: {错误}"))?;
        debug!(路径 = %路径.as_ref().display(), "依赖图已保存");
        std::fs::write(路径.as_ref(), 文本).map_err(|错误| format!("保存依赖图失败: {错误}"))
    }

    /// 从工作区加载（.上下文/依赖图.json）。
    pub fn 加载自工作区(工作区: &工作区) -> Result<依赖图, String> {
        Self::加载(工作区.依赖图路径())
    }

    /// 从指定路径加载（不存在则返回空图）。
    pub fn 加载(路径: impl AsRef<Path>) -> Result<依赖图, String> {
        let 路径 = 路径.as_ref();
        if !路径.exists() {
            return Ok(依赖图::default());
        }
        let 文本 =
            std::fs::read_to_string(路径).map_err(|错误| format!("读取依赖图失败: {错误}"))?;
        serde_json::from_str(&文本).map_err(|错误| format!("解析依赖图失败: {错误}"))
    }
}

#[cfg(test)]
mod 测试 {
    //! 依赖 - 边 - 园 单元测试：保存/加载往返、空路径兜底、
    //! 临时目录隔离、字段一致性、损坏 JSON 失败语义。

    use super::*;
    use crate::{符号档案, 结构节点};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 在系统临时目录生成唯一路径；约定清理：测试结束后不留残留。
    fn 临时路径(标签: &str) -> PathBuf {
        static 计数: AtomicU64 = AtomicU64::new(0);
        let n = 计数.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let 路径 = std::env::temp_dir().join(format!("shihai_依赖边_{pid}_{n}_{标签}.json"));
        let _ = std::fs::remove_file(&路径);
        路径
    }

    /// 构造一份带档案与结构根节点的"非默认"依赖图。
    fn 样例图(符号名: &str) -> 依赖图 {
        依赖图 {
            档案们: vec![符号档案::新(
                "示例项目",
                "示例府",
                "示例府/入口.rs",
                符号名,
                "pub fn 入口符号()",
                "fn 入口符号",
                "演示入口函数",
            )],
            结构树: 结构节点::新("根结构"),
        }
    }

    #[test]
    fn 加载_路径不存在_返回默认图() {
        let 路径 = 临时路径("默认");
        assert!(!路径.exists(), "前置：临时路径不应存在");
        let 图 = 依赖图::加载(&路径).expect("加载不存在的路径应成功");
        assert!(图.档案们.is_empty(), "默认图档案应为空");
    }

    #[test]
    fn 默认图保存再加载_结构一致() {
        let 路径 = 临时路径("默认往返");
        依赖图::default().保存(&路径).expect("默认图保存应成功");
        let 还原 = 依赖图::加载(&路径).expect("默认图加载应成功");
        assert!(还原.档案们.is_empty(), "往返后档案仍应为空");
        let 残留 = std::fs::read_to_string(&路径).expect("文件应已落盘");
        assert!(!残留.is_empty(), "默认图落盘应至少含字段骨架");
        let _ = std::fs::remove_file(&路径);
    }

    #[test]
    fn 含数据图保存再加载_字段全部还原() {
        let 路径 = 临时路径("字段");
        let 原 = 样例图("入口符号");
        原.保存(&路径).expect("样例图保存应成功");
        let 还原 = 依赖图::加载(&路径).expect("样例图加载应成功");

        assert_eq!(还原.档案们.len(), 1, "档案数应一致");
        let 档 = &还原.档案们[0];
        assert_eq!(档.项目, "示例项目");
        assert_eq!(档.模块, "示例府");
        assert_eq!(档.文件, "示例府/入口.rs");
        assert_eq!(档.符号, "入口符号");
        assert_eq!(档.代码, "pub fn 入口符号()");
        assert_eq!(档.签名, "fn 入口符号");
        assert_eq!(档.解释, "演示入口函数");
        assert_eq!(还原.结构树.名字, "根结构");
        let _ = std::fs::remove_file(&路径);
    }

    #[test]
    fn 临时目录隔离_两文件互不污染() {
        let 路径甲 = 临时路径("隔离甲");
        let 路径乙 = 临时路径("隔离乙");
        let 甲图 = 样例图("甲专属符号");
        let 乙图 = 样例图("乙专属符号");
        甲图.保存(&路径甲).expect("甲图保存应成功");
        乙图.保存(&路径乙).expect("乙图保存应成功");

        let 甲读 = 依赖图::加载(&路径甲).expect("甲图加载应成功");
        let 乙读 = 依赖图::加载(&路径乙).expect("乙图加载应成功");

        assert_eq!(甲读.档案们[0].符号, "甲专属符号");
        assert_eq!(乙读.档案们[0].符号, "乙专属符号");
        assert_ne!(甲读.档案们[0].符号, 乙读.档案们[0].符号, "两文件应互不污染");
        let _ = std::fs::remove_file(&路径甲);
        let _ = std::fs::remove_file(&路径乙);
    }

    #[test]
    fn 加载损坏json_返回错误而非默认值() {
        let 路径 = 临时路径("坏JSON");
        std::fs::write(&路径, "{not valid json").expect("写损坏JSON应成功");
        let 结果 = 依赖图::加载(&路径);
        assert!(结果.is_err(), "损坏JSON必须返回错误而非默认值兜底");
        let _ = std::fs::remove_file(&路径);
    }
}
