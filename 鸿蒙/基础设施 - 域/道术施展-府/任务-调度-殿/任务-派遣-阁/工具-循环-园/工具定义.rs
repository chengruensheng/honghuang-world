//! 工具 - 定义：全部工具 schema（与 手脚-施展-殿 一一对应，OpenAI 兼容）。
//!
//! v1 起改为 manifest 驱动：从 工具清单::清单() 投影 OpenAI schema，
//! 字段映射见 工具清单::manifest_转_openai。函数签名（pub fn 全部工具定义() -> Vec<工具定义>）
//! 与返回类型 Vec<工具定义> 保持历史兼容；内容来源由 manifest 单一事实驱动，
//! 编排元数据（tags / 副作用 / 限流）见 工具清单。

use moxing_fu::工具定义;

use super::工具清单::清单_转_openai;

/// 全部工具定义（与 手脚-施展-殿 一一对应，OpenAI 兼容 schema）。
///
/// v1：内容来源从 manifest 投影（工具清单::清单()），保证 13 个工具同步；
/// v0 历史版本曾硬编码 10 个 vec![…]，现已统一到 manifest 单一事实源。
pub fn 全部工具定义() -> Vec<工具定义> {
    清单_转_openai(&super::工具清单::清单())
}
