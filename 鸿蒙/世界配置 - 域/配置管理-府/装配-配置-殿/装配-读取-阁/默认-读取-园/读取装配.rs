//! 读取装配配置：`.上下文/装配.json` → 结构化装配；缺省/损坏回默认装配。
//!
//! 阶段 4 Profile 装配（融合蓝图 §14.11）：世界启动装配的单一事实源。
//! 定位用环境变量 WORLD_WORKSPACE_ROOT（本府不依赖 shihai_fu，避免跨府循环依赖）。

use crate::类型_定义_殿::装配配置;
use rizhi_fu::{debug, warn};
use std::path::PathBuf;

/// 工作区根：环境变量优先，缺省当前目录。
fn 工作区根() -> PathBuf {
    std::env::var("WORLD_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default())
}

/// 装配文件路径：`.上下文/装配.json`（工作区根下）。
pub fn 装配文件路径() -> PathBuf {
    工作区根().join(".上下文").join("装配.json")
}

/// 读装配配置：文件存在且解析成功 → 该装配；否则回默认装配（并告警）。
pub fn 读装配() -> 装配配置 {
    let 路径 = 装配文件路径();
    match std::fs::read_to_string(&路径) {
        Ok(文本) => match serde_json::from_str::<装配配置>(&文本) {
            Ok(装配) => {
                debug!(路径 = %路径.display(), "装配配置已读取");
                装配
            }
            Err(错误) => {
                warn!(路径 = %路径.display(), 错误 = %错误, "装配配置解析失败，回默认装配");
                装配配置::default()
            }
        },
        Err(_) => {
            debug!(路径 = %路径.display(), "无装配文件，用默认装配");
            装配配置::default()
        }
    }
}

/// 装配是否启用某扩展。
pub fn 启用扩展(装配: &装配配置, 扩展: &crate::扩展开关) -> bool {
    装配.启用扩展.contains(扩展)
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::类型_定义_殿::{扩展开关, 阶段值, 默认扩展名};

    /// 无装配文件 → 默认装配（乙阶段，扩展全开）。
    #[test]
    fn 读装配_无文件回默认() {
        // 不设环境变量时指向临时目录，避免读到真实装配文件。
        std::env::set_var("WORLD_WORKSPACE_ROOT", std::env::temp_dir());
        let 装配 = 读装配();
        assert_eq!(装配.阶段, 阶段值::乙);
        assert!(装配.启用扩展.contains(&扩展开关::巡世));
        assert_eq!(装配.模型提供者, "http");
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }

    /// 启停判定。
    #[test]
    fn 启用扩展_判定() {
        let 甲装配 = 装配配置 {
            阶段: 阶段值::甲,
            启用扩展: vec![],
            ..装配配置::default()
        };
        assert!(!启用扩展(&甲装配, &扩展开关::巡世), "甲阶段不启巡世");
        let 乙装配 = 装配配置::default();
        assert!(启用扩展(&乙装配, &扩展开关::巡世), "乙阶段启巡世");
    }

    /// 缺省路径下 `.上下文/装配.json` 不存在时，`合法产物扩展名` 字段应回退到默认清单。
    /// 注：任务描述称「13 种」，但代码实情是默认清单含 14 种产物扩展名（`.rs`/`.md`/`.json`/`.toml`/`.yaml`/`.yml`/`.txt`/`.py`/`.sh`/`.ps1`/`.bat`/`.html`/`.css`/`.js`），本测试以代码实情为准。
    #[test]
    fn test_load_default_extensions_fallback() {
        // 隔离到临时目录，确保 `.上下文/装配.json` 不存在，走 Err(_) → 默认装配分支。
        std::env::set_var("WORLD_WORKSPACE_ROOT", std::env::temp_dir());
        let 装配 = 读装配();
        let 期望 = 默认扩展名();
        assert_eq!(
            装配.合法产物扩展名.len(),
            期望.len(),
            "缺省时合法产物扩展名数量应等于默认清单"
        );
        assert_eq!(
            装配.合法产物扩展名, 期望,
            "缺省时合法产物扩展名应回退到默认扩展名清单"
        );
        // 兜底：核心扩展名必须存在
        assert!(
            装配.合法产物扩展名.iter().any(|e| e == ".rs"),
            "缺省时应含 .rs"
        );
        assert!(
            装配.合法产物扩展名.iter().any(|e| e == ".md"),
            "缺省时应含 .md"
        );
        assert!(
            装配.合法产物扩展名.iter().any(|e| e == ".json"),
            "缺省时应含 .json"
        );
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }
}
