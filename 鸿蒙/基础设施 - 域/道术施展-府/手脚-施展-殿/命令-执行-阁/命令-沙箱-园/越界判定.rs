//! 越界判定：真实绝对路径是否在工作区根目录子树之内，不在则标记越界及具体逃逸目标。
//!
//! 任务背景：模型生成路径可能逃出工作区根（如硬编码绝对路径指向系统盘 / 外部盘 / 其他项目），
//! 必须在沙箱运行前预先判定并阻断，禁止污染真实盘面。
//!
//! 手段：
//! 1. 工作区根 canonicalize：解析符号链接、`.` / `..`、相对路径，统一为绝对规范路径；
//! 2. 待校验路径 canonicalize：同样解析；
//! 3. starts_with 比对（基于规范化后的绝对路径，避免「`..` 逃逸」与「符号链接逃逸」漏判）；
//! 4. 任一 canonicalize 失败 → 视为越界（路径不存在 / 无权限读取），报告逃逸目标。

use shihai_fu::世界结果;

use rizhi_fu::warn;
use std::path::{Path, PathBuf};

/// 越界判定结果：在内 / 越界 + 逃逸目标。
#[derive(Clone, Debug, PartialEq)]
pub struct 越界判定结果 {
    /// 是否在工作区根子树之内（true = 在内可放行，false = 越界需阻断）。
    pub 在内: bool,
    /// 越界时填入：原绝对路径或诊断信息（保留具体逃逸目标，未做模糊化）。
    pub 越界目标: Option<String>,
}

/// 判定真实绝对路径是否在工作区根目录子树之内。
/// - 工作区根 / 待校验路径 必须存在且可读（否则 canonicalize 失败 → 视为越界）。
/// - 大小写敏感（Windows 路径比较在 canonicalize 后统一由文件系统决定）。
pub fn 判定路径越界(工作区根: &Path, 待校验路径: &Path) -> 越界判定结果 {
    let 规范根 = match 工作区根.canonicalize() {
        Ok(路径) => 路径,
        Err(错误) => {
            warn!("工作区根规范化失败：{}：{错误}", 工作区根.display());
            return 越界判定结果 {
                在内: false,
                越界目标: Some(format!(
                    "工作区根无法规范化（不存在或不可达）：{}",
                    工作区根.display()
                )),
            };
        }
    };
    let 规范待 = match 待校验路径.canonicalize() {
        Ok(路径) => 路径,
        Err(错误) => {
            return 越界判定结果 {
                在内: false,
                越界目标: Some(format!(
                    "待校验路径无法规范化（不存在或不可达）：{}：{错误}",
                    待校验路径.display()
                )),
            };
        }
    };
    if 规范待.starts_with(&规范根) {
        越界判定结果 { 在内: true, 越界目标: None }
    } else {
        越界判定结果 {
            在内: false,
            越界目标: Some(规范待.to_string_lossy().into_owned()),
        }
    }
}

/// 越界文本拼接：把越界判定结果序列化为单行可读文本，供日志 / CLI 直接打印。
/// 在内返回 `在内`，越界返回 `越界：目标=...`。
pub fn 越界判定文本(结果: &越界判定结果) -> String {
    match (&结果.在内, &结果.越界目标) {
        (true, _) => "在内".to_string(),
        (false, Some(目标)) => format!("越界：目标={目标}"),
        (false, None) => "越界：目标未知".to_string(),
    }
}

/// 内部辅助：取规范化绝对路径（暴露供其他园复用；不直接判越界，只做 canonicalize 解析）。
pub fn 规范化路径(路径: &Path) -> 世界结果<PathBuf> {
    路径
        .canonicalize()
        .map_err(|错误| format!("路径规范化失败：{}：{错误}", 路径.display()))
}