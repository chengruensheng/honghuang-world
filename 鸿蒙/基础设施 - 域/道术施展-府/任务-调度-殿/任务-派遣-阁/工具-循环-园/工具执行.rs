//! 工具 - 执行：执行单个工具调用 + 落盘护栏 + 参数摘要。

use crate::{列举目录, 寻找文件, 搜索内容, 写文件, 改文件, 读文件, 删文件, 校验命令护栏, 沙箱视图};
use moxing_fu::工具调用;
use shihai_fu::{工作区, 模型存储};
use std::path::PathBuf;

/// 单文件落盘内容上限（字节）：超限拒写，防一次性灌爆盘面。
pub(crate) const 落盘内容上限: usize = 512 * 1024;

/// 落盘护栏：不依赖模型自觉，系统侧强制。
/// 1) 内容为空拒写——防空文件静默破坏（空文件编译通过但内容全丢）；
/// 2) 内容超长拒写——防一次性灌爆盘面/上下文；
/// 3) 路径越界拒绝——从目标路径逐级上溯最近已存在祖先，规范化后必须位于工作区根内（防 ../ 逃逸）。
/// 工具模式与纯文本回退共用，保证两条落盘路径同等受约束。
pub fn 校验落盘(根: &PathBuf, 路径: &str, 内容: &str) -> Result<(), String> {
    if 路径.trim().is_empty() {
        return Err("拒写：路径为空".to_string());
    }
    if 内容.trim().is_empty() {
        return Err(format!("拒写空文件：{路径}（内容为空）"));
    }
    if 内容.len() > 落盘内容上限 {
        return Err(format!("拒写超长文件：{路径}（{} 字节，超上限 {落盘内容上限}）", 内容.len()));
    }
    let 根规范 = 根.canonicalize().map_err(|错误| format!("工作区根无法解析：{错误}"))?;
    let 绝对 = 根.join(路径);
    let mut 检查 = 绝对.as_path();
    loop {
        if let Ok(规范) = 检查.canonicalize() {
            if !规范.starts_with(&根规范) {
                return Err(format!(
                    "路径越界拒绝：{路径}（已解析到 {}，超出工作区根 {}）",
                    规范.display(),
                    根规范.display()
                ));
            }
            return Ok(());
        }
        let Some(父) = 检查.parent() else { break };
        if 父 == 检查 {
            break;
        }
        检查 = 父;
    }
    Err(format!("路径无法解析：{路径}（最近已存在祖先不在工作区根 {} 内）", 根规范.display()))
}

/// 工具护栏（guard 阶段）：执行前统一护栏，与 execute 解耦（对齐 DeepSeek 工具流水线）。
/// 本质：任何项目的通用安全护栏——落盘非空/非超长/路径不越界，命令不越权。
/// 读类工具（读文件/列举/寻找/搜索/读格位/查格位历史）天然只读，无护栏。
pub(crate) fn 工具护栏(根: &PathBuf, 调用名: &str, 参数: &serde_json::Value) -> Result<(), String> {
    match 调用名 {
        "写文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 内容 = 参数["内容"].as_str().ok_or("缺参数 内容")?;
            校验落盘(根, 路径, 内容)
        }
        "改文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 新文 = 参数["新文"].as_str().ok_or("缺参数 新文")?;
            校验落盘(根, 路径, 新文)
        }
        "运行命令" => {
            let 命令 = 参数["命令"].as_str().ok_or("缺参数 命令")?;
            let 参数们 = 参数["参数们"].as_array().map(|数组| {
                数组.iter().filter_map(|值| 值.as_str()).collect::<Vec<_>>()
            }).unwrap_or_default();
            let 超时毫秒 = 参数.get("超时毫秒").and_then(|值| 值.as_u64());
            校验命令护栏(命令, &参数们, 超时毫秒)
        }
        _ => Ok(()),
    }
}

/// 执行单个工具调用，映射到 手脚-施展-殿 的真实函数。
pub(crate) fn 执行工具(
    调用: &工具调用,
    根: &PathBuf,
    写入文件们: &mut Vec<(String, u64)>,
) -> Result<String, String> {
    let 参数: serde_json::Value = serde_json::from_str(&调用.参数)
        .map_err(|错误| format!("参数解析失败: {错误}；原文：{}", 调用.参数))?;

    // guard 阶段：统一护栏（与 execute 解耦）。
    工具护栏(根, &调用.名字, &参数)?;

    match 调用.名字.as_str() {
        "写文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 内容 = 参数["内容"].as_str().ok_or("缺参数 内容")?;
            let 绝对 = 根.join(路径);
            写文件(绝对.to_str().ok_or("路径含非 UTF-8 字符")?, 内容)?;
            写入文件们.push((路径.to_string(), 内容.len() as u64));
            Ok(format!("已写入 {路径}（{} 字节）", 内容.len()))
        }
        "读文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 绝对 = 根.join(路径);
            let 内容 = 读文件(绝对.to_str().ok_or("路径含非 UTF-8 字符")?)?;
            Ok(format!("【文件：{路径}】\n{内容}"))
        }
        "改文件" => {
            let 路径 = 参数["路径"].as_str().ok_or("缺参数 路径")?;
            let 旧文 = 参数["旧文"].as_str().ok_or("缺参数 旧文")?;
            let 新文 = 参数["新文"].as_str().ok_or("缺参数 新文")?;
            let 绝对 = 根.join(路径);
            改文件(绝对.to_str().ok_or("路径含非 UTF-8 字符")?, 旧文, 新文)?;
            写入文件们.push((路径.to_string(), 新文.len() as u64));
            Ok(format!("已改写 {路径}"))
        }
        "删文件" => {
            let 路径们 = 参数["路径们"].as_array().ok_or("缺参数 路径们")?;
            let 文本们 = 路径们.iter().filter_map(|值| 值.as_str()).collect::<Vec<_>>();
            if 文本们.is_empty() {
                return Err("路径们 为空".to_string());
            }
            let 绝对的 = 文本们.iter().map(|路径| 根.join(路径)).collect::<Vec<_>>();
            let 字符串们 = 绝对的.iter().map(|路径| 路径.to_string_lossy().into_owned()).collect::<Vec<String>>();
            let 参数们 = 字符串们.iter().map(|路径| 路径.as_str()).collect::<Vec<_>>();
            删文件(&参数们)?;
            Ok(format!("已删除 {} 个文件", 参数们.len()))
        }
        "列举目录" => {
            let 路径 = 参数["路径"].as_str().unwrap_or("");
            let 绝对 = 根.join(路径);
            let 条目们 = 列举目录(绝对.to_str().ok_or("路径含非 UTF-8 字符")?)?;
            let mut 行 = String::new();
            for 条目 in 条目们 {
                行.push_str(&format!("{}{}（{} 字节）\n", 条目.名称, if 条目.是目录 { "/" } else { "" }, 条目.字节数));
            }
            Ok(行)
        }
        "寻找文件" => {
            let 根参 = 参数["根"].as_str().unwrap_or("");
            let 模式 = 参数["模式"].as_str().ok_or("缺参数 模式")?;
            let 绝对 = 根.join(根参);
            let 文件们 = 寻找文件(绝对.to_str().ok_or("路径含非 UTF-8 字符")?, 模式)?;
            if 文件们.is_empty() {
                return Ok("（未找到匹配文件）".to_string());
            }
            Ok(文件们.iter().take(100).cloned().collect::<Vec<_>>().join("\n"))
        }
        "搜索内容" => {
            let 根参 = 参数["根"].as_str().unwrap_or("");
            let 字面串 = 参数["字面串"].as_str().ok_or("缺参数 字面串")?;
            let 绝对 = 根.join(根参);
            let 命中们 = 搜索内容(绝对.to_str().ok_or("路径含非 UTF-8 字符")?, 字面串)?;
            if 命中们.is_empty() {
                return Ok("（未检索到命中）".to_string());
            }
            let mut 行 = String::new();
            for 命中 in 命中们.iter().take(60) {
                行.push_str(&format!("{}:{} {}\n", 命中.路径, 命中.行号, 命中.行内容.trim()));
            }
            Ok(行)
        }
        "运行命令" => {
            let 命令 = 参数["命令"].as_str().ok_or("缺参数 命令")?;
            let 参数们 = 参数["参数们"].as_array().map(|数组| {
                数组.iter().filter_map(|值| 值.as_str()).collect::<Vec<_>>()
            }).unwrap_or_default();
            let 超时毫秒 = 参数.get("超时毫秒").and_then(|值| 值.as_u64());
            let 工作目录 = 参数["工作目录"].as_str().map(|相对| 根.join(相对).to_string_lossy().into_owned());
            // 沙箱执行：命令在隔离视图内跑，构建物落视图内，越界写入自动回滚，真实盘面零污染。
            let 沙箱 = 沙箱视图::打开当前(根);
            let 回执 = 沙箱.运行(命令, &参数们, 工作目录.as_deref(), 超时毫秒)?;
            let 结果 = &回执.结果;
            let mut 输出 = format!(
                "退出码：{:?}\n标准输出：\n{}\n标准错误：\n{}",
                结果.退出码, 结果.标准输出, 结果.标准错误
            );
            if 回执.越界数 > 0 {
                输出.push_str(&format!(
                    "\n【沙箱已拦截并回滚 {} 处越界写入，真实工作区未被改动】\n{}",
                    回执.越界数, 回执.越界详情
                ));
            }
            if let Some(超时) = 超时毫秒 {
                输出.push_str(&format!("\n【超时上限：{} 毫秒】\n", 超时));
            }
            Ok(输出)
        }
        "读格位" => {
            let 格位名 = 参数["格位名"].as_str().ok_or("缺参数 格位名")?;
            let 上限 = 参数["上限"].as_u64().unwrap_or(20).min(200) as usize;
            let 工作区 = 工作区::定位();
            let 存储 = 模型存储::在工作区(&工作区);
            let 链头 = 存储.读链头集(格位名)?;
            let 取条 = 链头.len().saturating_sub(上限);
            let 窗口 = &链头[取条..];
            let mut 输出 = format!("【格位：{格位名}】链头 {} 条（返回 {} 条）\n", 链头.len(), 窗口.len());
            for (序号, 记录) in 窗口.iter().enumerate() {
                输出.push_str(&format!(
                    "#{} [ts={} 来源={:?}] {}\n",
                    序号 + 1 + 取条, 记录.时间戳, 记录.来源, 记录.内容
                ));
                if !记录.证据.is_empty() {
                    输出.push_str(&format!("    证据：{}\n", 记录.证据));
                }
            }
            Ok(输出)
        }
        "查格位历史" => {
            let 格位名 = 参数["格位名"].as_str().ok_or("缺参数 格位名")?;
            let 起始 = 参数["起始"].as_u64().unwrap_or(0) as usize;
            let 上限 = 参数["上限"].as_u64().unwrap_or(50).min(500) as usize;
            let 工作区 = 工作区::定位();
            let 存储 = 模型存储::在工作区(&工作区);
            let 全部 = 存储.读格位(格位名)?;
            let 终 = (起始 + 上限).min(全部.len());
            if 起始 >= 全部.len() {
                return Ok(format!("【格位：{格位名}】共 {} 条，偏移 {起始} 越界", 全部.len()));
            }
            let 窗口 = &全部[起始..终];
            let mut 输出 = format!("【格位：{格位名}】共 {} 条，返回第 {}..{} 条（{} 条）\n", 全部.len(), 起始, 终, 窗口.len());
            for (序号, 记录) in 窗口.iter().enumerate() {
                let 失效 = if 记录.失效 { " [失效]" } else { "" };
                输出.push_str(&format!(
                    "#{} [ts={} 来源={:?}]{} {}\n",
                    起始 + 序号 + 1, 记录.时间戳, 记录.来源, 失效, 记录.内容
                ));
                if !记录.证据.is_empty() {
                    输出.push_str(&format!("    证据：{}\n", 记录.证据));
                }
            }
            Ok(输出)
        }
        _ => Err(format!("未知工具：{}", 调用.名字)),
    }
}

/// 工具参数摘要：只取轻量字段（路径/命令等），不打印大内容，供日志定位是哪一步卡住。
pub(crate) fn 参数摘要(调用: &工具调用) -> String {
    let Ok(参数) = serde_json::from_str::<serde_json::Value>(&调用.参数) else {
        return 调用.参数.chars().take(80).collect();
    };
    match 调用.名字.as_str() {
        "写文件" | "读文件" | "改文件" => 参数["路径"].as_str().unwrap_or("?").to_string(),
        "删文件" => serde_json::to_string(&参数["路径们"]).unwrap_or_default(),
        "列举目录" => 参数["路径"].as_str().unwrap_or("（根）").to_string(),
        "寻找文件" => format!(
            "根={} 模式={}",
            参数["根"].as_str().unwrap_or("（根）"),
            参数["模式"].as_str().unwrap_or("?")
        ),
        "搜索内容" => format!(
            "根={} 字面串={}",
            参数["根"].as_str().unwrap_or("（根）"),
            参数["字面串"].as_str().unwrap_or("?")
        ),
        "运行命令" => format!(
            "{} {:?}",
            参数["命令"].as_str().unwrap_or("?"),
            参数["参数们"].as_array().map(|数组| 数组.iter().filter_map(|项| 项.as_str()).collect::<Vec<_>>()).unwrap_or_default()
        ),
        "读格位" => format!("格位={} 上限={}", 参数["格位名"].as_str().unwrap_or("?"), 参数["上限"].as_u64().unwrap_or(20)),
        "查格位历史" => format!(
            "格位={} 起始={} 上限={}",
            参数["格位名"].as_str().unwrap_or("?"),
            参数["起始"].as_u64().unwrap_or(0),
            参数["上限"].as_u64().unwrap_or(50)
        ),
        _ => 调用.参数.chars().take(80).collect(),
    }
}
