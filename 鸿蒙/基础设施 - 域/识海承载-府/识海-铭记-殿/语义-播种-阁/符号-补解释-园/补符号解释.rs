//! 符号 - 补解释 - 园：对依赖图里解释为空的 pub 符号，唤起 LLM 读源码补一句语义，写回依赖图。

use crate::{依赖图, 工作区};
use jiance_fu::{观测角色, 进入观测};
use moxing_fu::{对话消息, 模型配置, 调用模型};
use rizhi_fu::{info, warn};
use std::collections::BTreeMap;
use std::path::Path;

/// 补符号解释：按府分组，每组一次 LLM 调用，批量补该府空解释符号的语义，写回依赖图。
/// 返回补全的解释数量。
pub fn 补符号解释(根目录: &Path, 配置: &模型配置) -> Result<usize, String> {
    // 白箱观测：依赖图补语义进入归因角色（世界级维护，无要求关联）。
    let _观测守卫 = 进入观测(观测角色::归因, None, None, None);
    let 工作区 = 工作区::新(根目录);
    let mut 图 = 依赖图::加载自工作区(&工作区)?;

    // 按府收集空解释符号（文件、符号、签名）
    let mut 府符号: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    for 档案 in &图.档案们 {
        if !档案.解释.is_empty() {
            continue;
        }
        府符号.entry(档案.模块.clone()).or_default().push((
            档案.文件.clone(),
            档案.符号.clone(),
            档案.签名.clone(),
        ));
    }
    if 府符号.is_empty() {
        info!("无空解释符号，跳过补语义");
        return Ok(0);
    }

    let mut 总数 = 0;
    for (府, 符号们) in &府符号 {
        let 证据 = 收集府证据(根目录, 符号们);
        let 提示 = 渲染补解释提示(府, 符号们, &证据);
        let 回复 = match 调用模型(配置, &[对话消息::用户(&提示)], moxing_fu::精简上限)
        {
            Ok((回复, _用量)) => 回复,
            Err(错误) => {
                warn!(府, "补解释调用失败：{错误}");
                continue;
            }
        };
        let 映射 = 解析解释(&回复);
        let mut 命中 = 0;
        for 档案 in &mut 图.档案们 {
            if 档案.模块 == *府 && 档案.解释.is_empty() {
                if let Some(解释) = 映射.get(&档案.符号) {
                    档案.解释 = 解释.clone();
                    命中 += 1;
                }
            }
        }
        info!(府, 命中, "该府补解释完成");
        总数 += 命中;
    }

    图.保存在工作区(&工作区)?;
    info!(总数, "补符号解释完成");
    Ok(总数)
}

/// 收集某府空解释符号所在文件的文件头（前 30 行），去重，作为 LLM 证据。
fn 收集府证据(根目录: &Path, 符号们: &[(String, String, String)]) -> String {
    let mut 文件们: Vec<String> = 符号们.iter().map(|(文件, _, _)| 文件.clone()).collect();
    文件们.sort();
    文件们.dedup();
    let mut 证据 = String::new();
    for 文件 in &文件们 {
        let 绝对 = 根目录.join(文件);
        let Ok(内容) = std::fs::read_to_string(&绝对) else {
            continue;
        };
        let 头: String = 内容.lines().take(30).collect::<Vec<_>>().join("\n");
        证据.push_str(&format!("【{文件}】\n{头}\n\n"));
    }
    证据
}

/// 渲染补解释提示：符号清单 + 源码文件头，要求逐行输出「符号名：解释」。
fn 渲染补解释提示(
    府: &str, 符号们: &[(String, String, String)], 证据: &str
) -> String {
    let 清单: String = 符号们
        .iter()
        .map(|(文件, 符号, 签名)| format!("{符号}（{文件} · {签名}）"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "你是阴（守护进程，旁路观察者）。下面是一个府里缺少语义解释的 pub 符号，请为每个符号补一句不超过 30 字的中文解释，说明它做什么。\n\n\
         【府】{府}\n\n\
         【符号清单】（符号 · 文件 · 签名）\n{清单}\n\n\
         【源码证据】（文件头）\n{证据}\n\n\
         【输出要求】\n\
         1. 每行「符号名：一句解释」，只输出这些行，不要多余文字；\n\
         2. 解释必须来自证据或符号签名，严禁编造；\n\
         3. 证据不足的符号输出「符号名：不确定」。"
    )
}

/// 解析 LLM 输出「符号名：解释」行，返回符号名 → 解释的映射。
fn 解析解释(文本: &str) -> BTreeMap<String, String> {
    let mut 映射 = BTreeMap::new();
    for 行 in 文本.lines() {
        let 行 = 行.trim();
        if 行.is_empty() {
            continue;
        }
        let 全角 = 行.find('：');
        let 半角 = 行.find(':');
        let 分隔 = 全角.or(半角);
        let (符号, 解释) = match 分隔 {
            Some(位置) => {
                let 宽 = if 全角 == Some(位置) {
                    '：'.len_utf8()
                } else {
                    1
                };
                (
                    行[..位置].trim().to_string(),
                    行[位置 + 宽..].trim().to_string(),
                )
            }
            None => continue,
        };
        if 符号.is_empty() || 解释.is_empty() || 解释 == "不确定" {
            continue;
        }
        映射.insert(符号, 解释);
    }
    映射
}
