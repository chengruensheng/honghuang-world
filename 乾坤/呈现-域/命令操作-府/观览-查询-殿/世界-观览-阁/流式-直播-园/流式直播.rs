//! 流式-直播-园：事件流 + 模型观测 实时可读渲染（tail -f 模式）。
//! 生产化演示反馈：窗口里只见轮次/用量数字，看不到世界在做什么、模型返回了什么。
//! 本园双源 tail：事件流（阶段/工具/验收）+ 模型流水-观测.log（模型每轮回复原文），
//! 渲染成中文可读行——界主据此纠错、判断卡顿；30 秒无新事件输出空闲心跳。

use crate::工作区根;
use rizhi_fu::info;
use std::time::Duration;

/// 「世界 直播」：tail -f 事件流 + 模型观测日志（常驻不退出，Ctrl+C 停止）。
pub fn 世界直播() -> String {
    let 事件路径 = 工作区根().join(".上下文").join("事件流.jsonl");
    let 观测路径 = 工作区根().join("临时文件夹").join("模型流水-观测.log");
    info!("世界直播启动（事件流 + 模型观测，Ctrl+C 停止）");
    let mut 已见行数 = 0usize;
    let mut 观测位置 = 0usize; // 观测日志已读字节位置（上次 len 是合法 utf8 边界）
    let mut 上次输出 = std::time::Instant::now();
    loop {
        let mut 有新 = false;
        if let Ok(内容) = std::fs::read_to_string(&事件路径) {
            let 行们: Vec<&str> = 内容.lines().filter(|行| !行.trim().is_empty()).collect();
            if 行们.len() > 已见行数 {
                for 行 in &行们[已见行数..] {
                    if let Some(渲染) = 渲染事件(行) {
                        println!("{渲染}");
                        有新 = true;
                    }
                }
                已见行数 = 行们.len();
            }
        }
        if let Ok(内容) = std::fs::read_to_string(&观测路径) {
            if 内容.len() > 观测位置 {
                let 新段 = &内容[观测位置..];
                渲染观测段(新段, &mut 有新);
                观测位置 = 内容.len();
            }
        }
        if 有新 {
            上次输出 = std::time::Instant::now();
        } else if 上次输出.elapsed().as_secs() >= 30 {
            println!("…等待中（{} 秒无新事件：模型思考中，或网络卡顿）", 上次输出.elapsed().as_secs());
            上次输出 = std::time::Instant::now();
        }
        std::thread::sleep(Duration::from_millis(800));
    }
}

/// 渲染观测日志新增段：按「========== 模型调用 @ 毫秒 ==========」块拆分，
/// 只显示【回复】内容（完整提示词太长，回看用 读文件 临时文件夹/模型流水-观测.log）。
fn 渲染观测段(新段: &str, 有新: &mut bool) {
    for 块 in 新段.split("========== 模型调用 @ ") {
        let 块 = 块.trim();
        if 块.is_empty() {
            continue;
        }
        // 块头 = 毫秒时间戳（换行前）。
        let 时间戳 = 块
            .lines()
            .next()
            .and_then(|行| 行.trim_end_matches(" ==========").trim().parse::<u64>().ok())
            .unwrap_or(0);
        let 距今秒 = shihai_fu::当前毫秒().saturating_sub(时间戳) / 1000;
        let 时间 = 相对时间(距今秒);
        // 只取【回复】之后的内容（模型返回原文，含工具调用 JSON）。
        let 回复 = 块.split("【回复】").nth(1).unwrap_or("").trim();
        if !回复.is_empty() {
            let 摘要 = 截断(回复, 300);
            println!("[{时间}] 【模型回复】{摘要}");
            *有新 = true;
        }
    }
}

fn 截断(文本: &str, 上限: usize) -> String {
    let 字符们: Vec<char> = 文本.chars().collect();
    if 字符们.len() > 上限 {
        format!("{}…", 字符们[..上限].iter().collect::<String>())
    } else {
        文本.to_string()
    }
}

fn 相对时间(距今秒: u64) -> String {
    if 距今秒 < 60 {
        format!("{距今秒}s前")
    } else if 距今秒 < 3600 {
        format!("{}分前", 距今秒 / 60)
    } else {
        format!("{}小时前", 距今秒 / 3600)
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

#[cfg(test)]
mod 测试 {
    use super::{渲染观测段, 渲染事件};

    #[test]
    fn 渲染事件_工具调用可读() {
        let 行 = r#"{"时间戳":1786931587011,"类型":"工具调用","载荷":{"参数":"乾坤/甲园/甲.rs","失败":false,"工具":"写文件","结果长度":131,"轮次":3}}"#;
        let 渲染 = 渲染事件(行).expect("应渲染成功");
        assert!(渲染.contains("【工具:写文件】"), "应含工具名：{渲染}");
        assert!(渲染.contains("甲.rs"), "应含参数：{渲染}");
        assert!(渲染.contains("131"), "应含结果长度：{渲染}");
        assert!(渲染.contains("轮 3"), "应含轮次：{渲染}");
        assert!(渲染.contains("前"), "应含相对时间：{渲染}");
    }

    #[test]
    fn 渲染事件_验收结论可读() {
        let 行 = r#"{"时间戳":1786931587011,"类型":"验收结论","载荷":{"结论":"通过","要求id":"要求-9","终裁依据":"六准圣一致通过","尝试":0}}"#;
        let 渲染 = 渲染事件(行).expect("应渲染成功");
        assert!(渲染.contains("【验收】【要求-9】"), "应含要求id：{渲染}");
        assert!(渲染.contains("通过"), "应含结论：{渲染}");
        assert!(渲染.contains("六准圣一致通过"), "应含依据：{渲染}");
    }

    #[test]
    fn 渲染事件_损坏行跳过() {
        assert!(渲染事件("{{{ 半写行").is_none(), "损坏行应跳过不崩溃");
        assert!(渲染事件("").is_none(), "空行应跳过");
    }

    #[test]
    fn 渲染观测段_模型回复可读() {
        let 段 = "\n========== 模型调用 @ 1786931587011 ==========\n【用量】提示词=100 输出=50\n【用户】\n你好\n\n【回复】\n{\"id\":\"x\",\"choices\":[{\"message\":{\"content\":\"我来写测试\",\"tool_calls\":[{\"function\":{\"name\":\"写文件\"}}]}}]}";
        let mut 有新 = false;
        渲染观测段(段, &mut 有新);
        assert!(有新, "应识别到模型回复");
    }
}
