//! 文件查询-阁/文件-处理-园：文件清单与内容读取。
//!
//! 分类规则（互斥，按优先级）：
//! 1. 架构：文件名含「架构」或「蓝图」；
//! 2. 门禁：文件名含「门禁」；
//! 3. 传承殿：文档类扩展名（md/html/txt）。
//!
//! 扫描根取数据服务状态.扫描根（默认 ./），递归收集，跳过编译产物与版本控制目录。

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use crate::数据服务状态;

/// 单个文件条目
#[derive(Debug, Clone, Serialize)]
pub struct 文件条目 {
    /// 文件名（含扩展名）
    pub 名称: String,
    /// 相对扫描根的路径（用 / 分隔）
    pub 路径: String,
}

/// 文件清单响应：三类文件互斥归类
#[derive(Debug, Serialize)]
pub struct 文件清单响应 {
    pub 传承殿: Vec<文件条目>,
    pub 门禁: Vec<文件条目>,
    pub 架构: Vec<文件条目>,
}

/// 内容读取请求参数
#[derive(Debug, Deserialize)]
pub struct 内容请求 {
    pub 路径: String,
}

/// 内容读取响应
#[derive(Debug, Serialize)]
pub struct 内容响应 {
    pub 路径: String,
    pub 内容: String,
}

/// 传承殿文档扩展名（仅纯文档，代码文件不归传承殿）
const 文档清单扩展名: &[&str] = &["md", "html", "txt"];

/// 内容读取可读扩展名（含门禁代码/架构图可能的后缀）
const 可读扩展名: &[&str] = &["md", "html", "txt", "toml", "json", "rs", "py", "js", "css"];

/// 跳过目录（编译产物 / 版本控制 / 依赖）
const 跳过目录: &[&str] = &["target", ".git", "node_modules"];

/// 内容读取大小上限（字节）
const 内容上限: usize = 128 * 1024;

/// 分类键：1=架构 2=门禁 3=传承殿
const 类架构: u8 = 1;
const 类门禁: u8 = 2;
const 类传承殿: u8 = 3;

/// GET /api/files：扫描根下三类文件清单
pub async fn 文件清单接口(状态: State<数据服务状态>) -> Json<文件清单响应> {
    let 根 = 状态.扫描根.lock().expect("扫描根锁中毒").clone();
    let mut 响应 = 文件清单响应 { 传承殿: Vec::new(), 门禁: Vec::new(), 架构: Vec::new() };
    递归收集(&根, "", &mut 响应, 0);
    Json(响应)
}

/// 递归收集文件并归类（深度上限 8，防目录过深）
fn 递归收集(根: &str, 相对: &str, 响应: &mut 文件清单响应, 深度: usize) {
    if 深度 > 8 {
        return;
    }
    let 目录 = if 相对.is_empty() {
        std::path::PathBuf::from(根)
    } else {
        std::path::Path::new(根).join(相对)
    };
    let 条目 = match std::fs::read_dir(&目录) {
        Ok(e) => e,
        Err(_) => return,
    };
    for 项 in 条目.flatten() {
        let 名 = 项.file_name().to_string_lossy().into_owned();
        let 相对路径 = if 相对.is_empty() { 名.clone() } else { format!("{相对}/{名}") };
        let 类型 = 项.file_type();
        if let Ok(t) = 类型 {
            if t.is_dir() {
                if 跳过目录.iter().any(|跳| 跳 == &名.as_str()) {
                    continue;
                }
                递归收集(根, &相对路径, 响应, 深度 + 1);
            } else if t.is_file() {
                if let Some(类) = 归类(&名, &相对路径) {
                    match 类 {
                        类架构 => 响应.架构.push(文件条目 { 名称: 名.clone(), 路径: 相对路径.clone() }),
                        类门禁 => 响应.门禁.push(文件条目 { 名称: 名.clone(), 路径: 相对路径.clone() }),
                        _ => 响应.传承殿.push(文件条目 { 名称: 名.clone(), 路径: 相对路径.clone() }),
                    }
                }
            }
        }
    }
}

/// 分类规则（互斥，按优先级）：架构 > 门禁 > 文档（传承殿）
fn 归类(名: &str, _路径: &str) -> Option<u8> {
    if 名.contains("架构") || 名.contains("蓝图") {
        return Some(类架构);
    }
    if 名.contains("门禁") {
        return Some(类门禁);
    }
    let 扩 = 名.rsplit('.').next()?.to_ascii_lowercase();
    if 文档清单扩展名.contains(&扩.as_str()) {
        return Some(类传承殿);
    }
    None
}

/// GET /api/files/content?路径=xxx：读取单个文件内容（限文档类扩展名与大小）
pub async fn 文件内容接口(
    状态: State<数据服务状态>,
    Query(请求): Query<内容请求>,
) -> Result<Json<内容响应>, StatusCode> {
    let 相对 = 请求.路径.trim().trim_start_matches(['/', '\\']);
    // 防目录穿越：拒绝含 .. 的路径
    if 相对.is_empty() || 相对.split(['/', '\\']).any(|段| 段 == "..") {
        return Err(StatusCode::BAD_REQUEST);
    }
    // 仅文档类扩展名可读
    let 扩 = 相对.rsplit('.').next().map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    if !可读扩展名.contains(&扩.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let 根 = 状态.扫描根.lock().expect("扫描根锁中毒");
    let 全路径 = std::path::Path::new(根.as_str()).join(相对);
    // canonicalize 解析符号链接与 .. 等，验证最终真实路径仍在扫描根内（防符号链接逃逸）
    let 规范根 = std::path::Path::new(根.as_str()).canonicalize().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let 规范路径 = 全路径.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    if !规范路径.starts_with(&规范根) {
        tracing::warn!("文件读取路径逃逸拦截: {:?} 不在 {:?} 内", 规范路径, 规范根);
        return Err(StatusCode::FORBIDDEN);
    }
    let 内容 = std::fs::read_to_string(&规范路径).map_err(|_| StatusCode::NOT_FOUND)?;
    if 内容.len() > 内容上限 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    Ok(Json(内容响应 { 路径: 相对.to_string(), 内容 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 架构优先归类() {
        assert_eq!(归类("架构全览图.html", ""), Some(类架构));
        assert_eq!(归类("完整架构蓝图.md", ""), Some(类架构));
        assert_eq!(归类("任务看板与上下文架构-设计表.md", ""), Some(类架构));
    }

    #[test]
    fn 门禁归类() {
        assert_eq!(归类("patch_门禁v146.py", ""), Some(类门禁));
        assert_eq!(归类("清理门禁与验收包-核对表.md", ""), Some(类门禁));
    }

    #[test]
    fn 传承殿仅收文档() {
        assert_eq!(归类("维护文档.md", ""), Some(类传承殿));
        assert_eq!(归类("架构图.html", ""), Some(类架构), "含架构应优先归架构");
        // 代码文件不归传承殿
        assert_eq!(归类("启动模块.rs", ""), None);
        assert_eq!(归类("Cargo.toml", ""), None);
    }
}
