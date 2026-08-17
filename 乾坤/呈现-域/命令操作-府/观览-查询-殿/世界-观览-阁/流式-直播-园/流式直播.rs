//! 流式-直播-园：事件流实时可读渲染（tail -f 模式）。
//! 生产化演示反馈：窗口里只见轮次/用量数字，看不到世界在做什么。
//! 本园把 .上下文/事件流.jsonl 的每条事件渲染成中文可读行（时间+阶段+工具+参数摘要+结论），
//! 常驻输出——界主开窗口跑「世界 直播」，即见世界自主开发的完整过程。

use crate::工作区根;
use rizhi_fu::info;
use std::time::Duration;

/// 「世界 直播」：tail -f 事件流，渲染新事件为可读行（常驻不退出，Ctrl+C 停止）。
pub fn 世界直播() -> String {
    let 路径 = 工作区根().join(".上下文").join("事件流.jsonl");
    info!("世界直播启动（tail -f 事件流，Ctrl+C 停止）");
    let mut 已见行数 = 0usize;
    loop {
        if let Ok(内容) = std::fs::read_to_string(&路径) {
            let 行们: Vec<&str> = 内容.lines().filter(|行| !行.trim().is_empty()).collect();
            if 行们.len() > 已见行数 {
                for 行 in &行们[已见行数..] {
                    if let Some(渲染) = 渲染事件(行) {
                        println!("{渲染}");
                    }
                }
                已见行数 = 行们.len();
            }
        }
        std::thread::sleep(Duration::from_millis(800));
    }
}

/// 事件 → 中文可读行。解析失败返回 None（跳过旧/半写行）。
/// 时间用相对秒数（距今 X 秒），避免时区换算。
fn 渲染事件(行: &str) -> Option<String> {
    let 值: serde_json::Value = serde_json::from_str(行).ok()?;
    let 时间戳 = 值["时间戳"].as_u64().unwrap_or(0);
    let 距今秒 = shihai_fu::当前毫秒().saturating_sub(时间戳) / 1000;
    let 时间 = if 距今秒 < 60 {
        format!("{距今秒}s前")
    } else if 距今秒 < 3600 {
        format!("{}分前", 距今秒 / 60)
    } else {
        format!("{}小时前", 距今秒 / 3600)
    };
    let 类型 = 值["类型"].as_str().unwrap_or("?");
    let 载荷 = &值["载荷"];
    let 主体 = 渲染载荷(类型, 载荷);
    Some(format!("[{时间}] {主体}"))
}

/// 载荷渲染：按事件类型把载荷字段拼成一句话（关键字段截断 100 字符防刷屏）。
fn 渲染载荷(类型: &str, 载荷: &serde_json::Value) -> String {
    let 取 = |键: &str| 载荷.get(键).and_then(|v| v.as_str()).unwrap_or("");
    let 截 = |文本: &str| -> String {
        let 字符们: Vec<char> = 文本.chars().collect();
        if 字符们.len() > 100 {
            format!("{}…", 字符们[..100].iter().collect::<String>())
        } else {
            文本.to_string()
        }
    };
    match 类型 {
        "想法投递" => format!("【界主想法】{}{}", 截(取("内容")), 想法id尾(取("想法id"))),
        "要求入池" => format!("【要求入池】{}{} [{}]", 截(取("方向")), 要求id尾(取("要求id")), 取("状态")),
        "设计上呈" => format!("【设计上呈】{}{}（拆解 {} 项）", 要求id尾(取("要求id")), 截(取("摘要")), 载荷["拆解数"].as_u64().unwrap_or(0)),
        "工具调用" => {
            let 工具 = 取("工具");
            let 参数 = 截(取("参数"));
            let 失败 = 载荷["失败"].as_bool().unwrap_or(false);
            let 结果长度 = 载荷["结果长度"].as_u64().unwrap_or(0);
            let 轮次 = 载荷["轮次"].as_u64().unwrap_or(0);
            if 失败 {
                format!("【工具:{}】✗ {}{}（轮 {}）", 工具, 参数, 结果标注(结果长度), 轮次)
            } else {
                format!("【工具:{}】{}{}（轮 {}）", 工具, 参数, 结果标注(结果长度), 轮次)
            }
        }
        "验收结论" => format!(
            "【验收】{}{} 结论={} 依据:{}",
            要求id尾(取("要求id")),
            尝试标注(载荷["尝试"].as_u64().unwrap_or(0)),
            取("结论"),
            截(取("终裁依据"))
        ),
        "要求状态推进" => format!("【状态】{}{} → {}", 要求id尾(取("要求id")), 截(取("状态")), 截(取("说明"))),
        "失败沉淀" => format!("【失败】{}{} 依据:{}", 要求id尾(取("要求id")), 尝试标注(载荷["尝试"].as_u64().unwrap_or(0)), 截(取("终裁依据"))),
        "版本存档" => format!("【定档】{} {}", 截(取("版本号")), 截(取("说明"))),
        "想法状态推进" => format!("【想法】{}{} → {}", 想法id尾(取("想法id")), 截(取("状态")), 截(取("说明"))),
        _ => format!("【{类型}】{}", 截(&载荷.to_string())),
    }
}

fn 结果标注(结果长度: u64) -> String {
    if 结果长度 > 0 {
        format!("（返回 {} 字符）", 结果长度)
    } else {
        String::new()
    }
}

fn 尝试标注(尝试: u64) -> String {
    if 尝试 > 0 {
        format!("（尝试 {}）", 尝试 + 1)
    } else {
        String::new()
    }
}

fn 想法id尾(想法id: &str) -> String {
    if 想法id.is_empty() {
        String::new()
    } else {
        format!("（{}）", 想法id.chars().rev().take(8).collect::<String>().chars().rev().collect::<String>())
    }
}

fn 要求id尾(要求id: &str) -> String {
    if 要求id.is_empty() {
        String::new()
    } else {
        format!("【{}】", 要求id)
    }
}
