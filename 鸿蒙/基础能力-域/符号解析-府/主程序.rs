//! hm-symext —— 符号索引引擎入口（语言无关）
//!
//! 用法：`hm-symext [--项目根 <路径>]`
//! 不传 `--项目根` 时以当前工作目录为仓库根。

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let 仓库根 = 解析参数();
    let 引擎 = hm_symext::编排器::新(仓库根);

    match 引擎.运行() {
        Ok(索引) => {
            println!(
                "符号索引已生成：符号 {} / 边 {} / 悬空 {}",
                索引.统计.符号总数, 索引.统计.边总数, 索引.统计.悬空总数
            );
            for 状态 in &索引.统计.插件 {
                println!(
                    "  插件 {} — {}（符号 {} / 边 {}）：{}",
                    状态.语言, 状态.状态, 状态.符号数, 状态.边数, 状态.信息
                );
            }
            ExitCode::SUCCESS
        }
        Err(原因) => {
            eprintln!("符号索引生成失败：{原因}");
            ExitCode::from(1)
        }
    }
}

fn 解析参数() -> PathBuf {
    let 参数: Vec<String> = std::env::args().collect();
    let mut 仓库根 = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut i = 1;
    while i < 参数.len() {
        if 参数[i] == "--项目根" && i + 1 < 参数.len() {
            仓库根 = PathBuf::from(&参数[i + 1]);
            i += 2;
        }
        else {
            i += 1;
        }
    }

    仓库根
}
