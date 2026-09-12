//! hm-docgen —— 代码文档生成入口（知识图谱第 3 阶段）
//!
//! 用法：`hm-docgen [--索引 <json路径>] [--输出 <目录>] [--范围 <路径前缀>] [--叙述 模板|模型]`
//! 默认读 `.传承/图谱/知识图谱/符号索引.json`，产出到 `.传承/图谱/知识图谱/代码文档/`。
//!
//! 叙述默认走模板（离线、确定性）；`--叙述 模型` 改走大模型，凭据由环境提供
//! （`LLM_API_KEY` / `LLM_BASE_URL` / `LLM_MODEL`，可选 `LLM_FALLBACK_*` 作故障转移）。

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use hm_content::LLM池;
use hm_docgen::{生成配置, 模型叙述器, 编排器};

/// 叙述方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum 叙述方式 {
    模板,
    模型,
}

fn main() -> ExitCode {
    let (配置, 方式) = 解析参数();
    let 编排 = 装配(配置, 方式);

    match 编排.运行() {
        Ok(结果) => {
            println!(
                "代码文档已生成：{} 篇 / 符号 {} / 边 {}（叙述器 {}，降级 {} 篇）",
                结果.篇数, 结果.符号数, 结果.边数, 结果.叙述器, 结果.降级篇数
            );
            println!(
                "  事实校验：疑似幻觉 {} 篇；自检{}：{}",
                结果.可疑篇数(),
                if 结果.自检.通过 { "通过" } else { "未通过" },
                结果.自检.说明
            );
            ExitCode::SUCCESS
        }
        Err(原因) => {
            eprintln!("代码文档生成失败：{原因}");
            ExitCode::from(1)
        }
    }
}

/// 装配编排器：模型后端不可用时整批回落模板叙述器（设计 §5.2 降级矩阵）。
/// 回落只打日志不改结果，报告里的「叙述器」一行会如实写明实际用的哪一个。
fn 装配(配置: 生成配置, 方式: 叙述方式) -> 编排器 {
    if 方式 == 叙述方式::模板 {
        return 编排器::新(配置);
    }
    match LLM池::从环境() {
        Ok(池) if 池.可用() => 编排器::带叙述器(配置, Arc::new(模型叙述器::新(Arc::new(池)))),
        Ok(_) => {
            eprintln!("模型后端不可用（LLM 池无有效供应商），整批回落模板叙述器");
            编排器::新(配置)
        }
        Err(原因) => {
            eprintln!("模型后端不可用（{原因}），整批回落模板叙述器");
            编排器::新(配置)
        }
    }
}

/// 默认符号索引路径 / 默认代码文档输出目录（收敛硬编码路径，供 CLI 缺省参数复用）
const 默认索引路径: &str = ".传承/图谱/知识图谱/符号索引.json";
const 默认输出目录: &str = ".传承/图谱/知识图谱/代码文档";

fn 解析参数() -> (生成配置, 叙述方式) {
    let 参数: Vec<String> = std::env::args().collect();
    let mut 索引路径 = PathBuf::from(默认索引路径);
    let mut 输出目录 = PathBuf::from(默认输出目录);
    let mut 范围 = None;
    let mut 方式 = 叙述方式::模板;
    let mut i = 1;
    while i < 参数.len() {
        match 参数[i].as_str() {
            "--索引" if i + 1 < 参数.len() => {
                索引路径 = PathBuf::from(&参数[i + 1]);
                i += 2;
            }
            "--输出" if i + 1 < 参数.len() => {
                输出目录 = PathBuf::from(&参数[i + 1]);
                i += 2;
            }
            "--范围" if i + 1 < 参数.len() => {
                范围 = Some(参数[i + 1].clone());
                i += 2;
            }
            "--叙述" if i + 1 < 参数.len() => {
                方式 = match 参数[i + 1].as_str() {
                    "模型" => 叙述方式::模型,
                    _ => 叙述方式::模板,
                };
                i += 2;
            }
            _ => i += 1,
        }
    }

    (
        生成配置 {
            索引路径,
            输出目录,
            范围,
        },
        方式,
    )
}
