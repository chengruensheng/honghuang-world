//! 命令入口执行：解析 → 校验 → 分发 → 输出，返回退出码
use crate::{解析调用, 校验调用, 分发调用, 结果, 呈现文本, 呈现JSON, 全流程总览};

pub fn 执行() -> i32 {
    let 输入: Vec<String> = std::env::args().skip(1).collect();
    let 调用 = 解析调用(输入);

    if 调用.域.is_empty() {
        println!("{}", 全流程总览());
        return 0;
    }

    let 环境令牌 = std::env::var("WORLD_AI_TOKEN").ok();
    if let Err(理由) = 校验调用(&调用, 环境令牌.as_deref()) {
        eprintln!("{理由}");
        return 1;
    }

    match 分发调用(&调用) {
        结果::成功(内容) => {
            if 调用.要JSON {
                println!("{}", 呈现JSON(&内容));
            } else {
                println!("{}", 呈现文本(&内容));
            }
            0
        }
        结果::失败(内容) => {
            eprintln!("{内容}");
            1
        }
    }
}

/// 工作区根：WORLD_WORKSPACE_ROOT 环境变量，回退当前目录。
pub fn 工作区根() -> std::path::PathBuf {
    std::env::var("WORLD_WORKSPACE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default())
}

/// 读模型配置：.env → 配置管理府 → 模型连接府类型转换。
pub fn 读模型配置() -> moxing_fu::模型配置 {
    let 根 = 工作区根();
    let 配置 = peizhi_fu::读模型配置(根.join(".env").to_str().unwrap_or(""));
    moxing_fu::模型配置 { 密钥: 配置.密钥, 地址: 配置.地址, 模型: 配置.模型 }
}

/// 状态目录：.上下文/状态（想法池 / 要求队列 / 验收历史）。
pub fn 状态目录() -> std::path::PathBuf {
    工作区根().join(".上下文").join("状态")
}

/// 打开识海存储（落 .上下文/格位/）。
pub fn 打开存储() -> shihai_fu::模型存储 {
    shihai_fu::模型存储::打开(工作区根().join(".上下文").join("格位"))
}
