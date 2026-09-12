//! 对外接口-殿/开发接口-阁/工作区-处理-园：项目工作区根（扫描根）的查询与设置。
//!
//! 项目工作区根 = 项目根目录：文件面板（传承殿/门禁/架构）扫描它，
//! 后续智能体产出（代码/文档/架构）也落在此根下。顶栏选择器据此展示与切换。

use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use crate::接口状态;

/// 工作区响应：当前项目工作区根（绝对路径）
#[derive(Debug, Serialize)]
pub struct 工作区响应 {
    pub 工作区: String,
}

/// 工作区设置请求体
#[derive(Debug, Deserialize)]
pub struct 工作区设置请求 {
    pub 工作区: String,
}

/// GET /api/workspace：返回当前项目工作区根
pub async fn 工作区查询<M: Send + Sync + 'static>(状态: 接口状态<M>) -> Json<工作区响应> {
    let 根 = 状态.扫描根.lock().expect("扫描根锁中毒").clone();
    Json(工作区响应 { 工作区: 根 })
}

/// POST /api/workspace：设置项目工作区根（校验目录存在，规范化为绝对路径）
pub async fn 工作区设置<M: Send + Sync + 'static>(
    状态: 接口状态<M>,
    Json(请求): Json<工作区设置请求>,
) -> Result<Json<工作区响应>, (StatusCode, String)> {
    let 路径 = 请求.工作区.trim().to_string();
    if 路径.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "工作区路径不能为空".into()));
    }
    let 目录 = std::path::Path::new(&路径);
    if !目录.is_dir() {
        return Err((StatusCode::BAD_REQUEST, format!("目录不存在或非目录: {路径}")));
    }
    // 规范化为绝对路径存储（后续文件扫描与内容读取都基于它）；去掉 Windows verbatim 前缀 \\?\
    let 绝对 = match std::fs::canonicalize(目录) {
        Ok(规范化) => {
            let s = 规范化.to_string_lossy().into_owned();
            match s.strip_prefix(r"\\?\") {
                Some(无前缀) => 无前缀.to_string(),
                None => s,
            }
        }
        Err(e) => {
            tracing::warn!("项目工作区路径规范化失败，回退到原路径: {e}");
            路径.clone()
        }
    };
    *状态.扫描根.lock().expect("扫描根锁中毒") = 绝对.clone();
    tracing::info!("项目工作区（扫描根）已切换: {绝对}");

    // 同步智能体沙箱（dev_workspace）与看板驱动台（五层协作驱动器执行器），
    // 让产出直接落项目根，而非独立沙箱子目录。任一台运行中禁止重装配，此时仅更新扫描根并明示。
    let 开发运行中 = 状态.开发执行台.运行中();
    let 看板运行中 = 状态.看板驱动台.运行中();
    if 开发运行中 || 看板运行中 {
        tracing::warn!("智能体/看板驱动执行中，本次仅切换扫描根，工作区待下次空闲后同步");
    } else {
        if let Some(回调) = &状态.重装配工作区 {
            match 回调(&绝对) {
                Ok(()) => tracing::info!("开发受理台工作区已同步: {绝对}"),
                Err(e) => tracing::warn!("开发受理台工作区同步失败: {e}"),
            }
        }
        if let Some(回调) = &状态.重装配看板驱动 {
            match 回调(&绝对) {
                Ok(()) => tracing::info!("看板驱动台工作区已同步: {绝对}"),
                Err(e) => tracing::warn!("看板驱动台工作区同步失败: {e}"),
            }
        }
    }

    Ok(Json(工作区响应 { 工作区: 绝对 }))
}
