use hm_cognition::AgentRole;
use hm_contract::当前时间戳;
use hm_error::{Error, Result};
use tc_task::{
    TaskBoard, TaskStatus, DesignDoc, ImplementationDoc, VerificationDoc,
    FinalAcceptanceDoc, 五行层级, 审核记录, 审核来源, 驳回原因,
};
use crate::循环驱动_殿::智能体;
use crate::协作驱动_殿::五层驱动_阁::错误追溯_阁::追溯器;
use super::阶段提示::任务快照;

/// 解析阶段产出：提取 JSON → 注入 created_at → 反序列化为目标文档 → 返回（写文档闭包, 下一状态）
fn 解析并构造(
    角色: &AgentRole,
    状态: TaskStatus,
    答复: &str,
) -> Result<(Box<dyn FnOnce(&mut TaskBoard, u64) -> Result<()>>, TaskStatus)> {
    // LLM 常在 JSON 前包裹 <think>...</think> 思考标签或 ```json 代码块，
    // 提取json 会从第一个 { 开始匹配，可能抓到 think 内部的碎片 JSON 而非真正的阶段产出。
    // 先剥离这些杂质再提取，避免误抓导致的反序列化失败回喂重试浪费轮次。
    let 净化答复 = 剥离杂质标签(答复);
    let json = 提取json(&净化答复)
        .ok_or_else(|| Error::反序列化(format!("阶段产出无 JSON（答复前 200 字：{}）", 截断(答复, 200))))?;
    let mut 值: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| Error::反序列化(format!("阶段产出非合法 JSON: {e}；前 200 字：{}", 截断(&json, 200))))?;
    // 阶段文档的 created_at 为必填字段，由驱动器统一注入
    if let serde_json::Value::Object(map) = &mut 值 {
        map.insert("created_at".to_string(), serde_json::json!(当前时间戳()));
    } else {
        return Err(Error::反序列化(format!("阶段产出 JSON 顶层必须是对象，前 200 字：{}", 截断(&json, 200))));
    }
    match 角色 {
        AgentRole::圣人 => {
            let doc: DesignDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("设计文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            Ok((Box::new(move |看板, id| 看板.更新设计文档(id, doc)), TaskStatus::待大罗金仙实现))
        }
        AgentRole::大罗金仙 => {
            let doc: ImplementationDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("实现文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            Ok((Box::new(move |看板, id| 看板.更新实现文档(id, doc)), TaskStatus::待准圣验收))
        }
        AgentRole::准圣 => {
            let doc: VerificationDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("验收文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            let 下一状态 = if doc.最终结果 { TaskStatus::待道祖终审 } else { TaskStatus::待修复 };
            Ok((Box::new(move |看板, id| 看板.更新验收文档(id, doc)), 下一状态))
        }
        AgentRole::道祖 => {
            // 道祖同角色承担「终审」与「最终审核」两阶段，靠当前状态区分
            if matches!(状态, TaskStatus::待人工验收 | TaskStatus::人工验收中) {
                // 最终审核：产出审核记录（审核时间/来源由驱动器注入，LLM 只给结论三要素）
                let 通过 = 值
                    .get("通过")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| Error::反序列化(format!("审核记录缺「通过」布尔字段，前 200 字：{}", 截断(&json, 200))))?;
                let 驳回原因 = match 值.get("驳回原因") {
                    None | Some(serde_json::Value::Null) => None,
                    Some(v) => Some(serde_json::from_value::<驳回原因>(v.clone()).map_err(|e| {
                        Error::反序列化(format!("审核记录「驳回原因」非法: {e}；前 200 字：{}", 截断(&json, 200)))
                    })?),
                };
                if 通过 && 驳回原因.is_some() {
                    return Err(Error::反序列化("审核记录「通过=true」时「驳回原因」必须为 null".to_string()));
                }
                if !通过 && 驳回原因.is_none() {
                    return Err(Error::反序列化("审核记录「通过=false」时必须给出「驳回原因」".to_string()));
                }
                let 评语 = 值
                    .get("评语")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let 记录 = 审核记录::新(当前时间戳(), 通过, 驳回原因, 评语, 审核来源::自动);
                let 下一状态 = if 通过 { TaskStatus::待清理 } else { TaskStatus::待修复 };
                Ok((Box::new(move |看板, id| 看板.更新审核记录(id, 记录)), 下一状态))
            } else {
                let doc: FinalAcceptanceDoc = serde_json::from_value(值)
                    .map_err(|e| Error::反序列化(format!("终审文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
                let 下一状态 = if doc.通过 { TaskStatus::待人工验收 } else { TaskStatus::待修复 };
                Ok((Box::new(move |看板, id| 看板.更新终审文档(id, doc)), 下一状态))
            }
        }
        AgentRole::太乙金仙 => {
            // 清理阶段无文档槽位（Task 未扩展字段）：校验顶层对象即可，推进到 清理完成
            if !值.is_object() {
                return Err(Error::反序列化(format!("清理记录 JSON 顶层必须是对象，前 200 字：{}", 截断(&json, 200))));
            }
            Ok((Box::new(|_看板, _id| Ok(())), TaskStatus::清理完成))
        }
    }
}

/// 解析阶段产出（带失败重试）：首次解析失败时，把 serde 错误原文回喂 LLM 修正重试，上限 2 次；
/// 重试仍失败才返回 Err（任务保持待承接可重试，不死等）。
pub(crate) fn 解析并构造带重试(
    角色: &AgentRole,
    状态: TaskStatus,
    答复: &str,
    阶段提示: &str,
    智能体: &智能体,
) -> Result<(Box<dyn FnOnce(&mut TaskBoard, u64) -> Result<()>>, TaskStatus)> {
    let mut 当前答复 = 答复.to_string();
    let mut 重试 = 0;
    loop {
        match 解析并构造(角色, 状态, &当前答复) {
            Ok(结果) => return Ok(结果),
            Err(错误) => {
                if 重试 >= 2 {
                    return Err(错误);
                }
                重试 += 1;
                let 修复提示 = format!(
                    "{}\n\n【修正要求】你上一轮输出的内容无法解析为合法 JSON，具体错误：\n{}\n请严格只输出一个 JSON 对象，不要输出任何解释文字，不要用 markdown 代码块包裹。重新输出。",
                    阶段提示, 错误
                );
                tracing::warn!("阶段产出解析失败，回喂错误重试（第 {}/2 次）：{}", 重试, 错误);
                当前答复 = 智能体.运行(修复提示)?;
            }
        }
    }
}

/// 剥离 LLM 输出中的杂质标签（...、```json...```），
/// 避免 提取json 误抓 think 内部的碎片 JSON。
/// 支持大小写不敏感匹配、多段出现；保留标签外的正文内容。
/// 注意：<think>/``` 均为纯 ASCII，可直接在 UTF-8 字节流上匹配，
/// 避免 to_lowercase 改变多字节字符长度导致的偏移错位。
pub(crate) fn 剥离杂质标签(文本: &str) -> String {
    let mut 结果 = String::with_capacity(文本.len());
    let bytes = 文本.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // 检测 <think> 开始标签（7 字节纯 ASCII，大小写不敏感）
        if i + 7 <= len && bytes[i] == b'<' {
            let 候选 = &bytes[i..i + 7];
            if 候选.eq_ignore_ascii_case(b"<think>") {
                // 找 </think> 结束标签（8 字节纯 ASCII）
                if let Some(相对偏移) = 文本[i + 7..].find("</think>") {
                    // 跳过整个 <think>...</think> 块
                    i += 7 + 相对偏移 + 8;
                    continue;
                }
                // 无闭合标签：跳过 <think> 标记本身
                i += 7;
                continue;
            }
        }
        // 检测 ``` 代码块围栏（3 字节纯 ASCII）
        if i + 3 <= len && &bytes[i..i + 3] == b"```" {
            if let Some(相对偏移) = 文本[i + 3..].find("```") {
                let 块内容 = &文本[i + 3..i + 3 + 相对偏移];
                // 去掉首行可能的语言标识（如 "json\n"）
                let 内容起始 = if let Some(换行位置) = 块内容.find('\n') {
                    let 首行 = &块内容[..换行位置];
                    if 首行.trim().chars().all(|c| c.is_ascii_alphabetic()) {
                        换行位置 + 1
                    } else {
                        0
                    }
                } else {
                    0
                };
                结果.push_str(&块内容[内容起始..]);
                i += 3 + 相对偏移 + 3;
                continue;
            }
        }
        // 安全推进：UTF-8 首字节决定字符宽度，避免截断多字节字符
        let 字符宽度 = utf8_char_width(bytes[i]);
        let end = (i + 字符宽度).min(len);
        结果.push_str(&文本[i..end]);
        i = end;
    }
    结果
}

/// UTF-8 首字节 → 字符字节宽度（无效前缀当 1 字节处理）
fn utf8_char_width(首字节: u8) -> usize {
    match 首字节 {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1, // 无效续字节或孤立字节，安全跳过 1
    }
}

pub(crate) fn 提取json(文本: &str) -> Option<String> {
    let 开始 = 文本.find('{')?;
    let mut 深度 = 0i32;
    let mut 在字符串 = false;
    let mut 转义 = false;
    for (i, ch) in 文本[开始..].char_indices() {
        if 在字符串 {
            if 转义 {
                转义 = false;
            } else if ch == '\\' {
                转义 = true;
            } else if ch == '"' {
                在字符串 = false;
            }
            continue;
        }
        match ch {
            '"' => 在字符串 = true,
            '{' => 深度 += 1,
            '}' => {
                深度 -= 1;
                if 深度 == 0 {
                    return Some(文本[开始..=开始 + i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn 截断(文本: &str, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        文本.to_string()
    } else {
        let 头部: String = 文本.chars().take(上限).collect();
        format!("{头部}…")
    }
}

/// 验收不通过时的错误根源追溯（纯规则，不用 LLM）：
/// 用结构化设计/实现文档 + 验收答复判定根源层级，返回（根源层级, 错误描述）
pub(crate) fn 追溯根源(快照: &任务快照, 验收答复: &str) -> (五行层级, String) {
    let 现象 = 提取错误现象(验收答复);
    let 追溯器 = 追溯器::新();
    let 包 = 追溯器.追溯(
        快照.uuid,
        &现象,
        快照.设计对象.as_ref(),
        快照.实现对象.as_ref(),
        &快照.描述,
    );
    let 描述 = if 包.错误描述.is_empty() { 现象 } else { 包.错误描述 };
    (包.根源层级, 描述)
}

/// 从验收答复 JSON 提取错误现象（轮次问题/建议 文本）
fn 提取错误现象(答复: &str) -> String {
    let Some(json) = 提取json(答复) else {
        return 截断(答复, 200).to_string();
    };
    let Ok(值) = serde_json::from_str::<serde_json::Value>(&json) else {
        return 截断(答复, 200).to_string();
    };
    let mut 片段 = Vec::new();
    if let Some(轮次) = 值.get("轮次").and_then(|v| v.as_array()) {
        for 轮 in 轮次 {
            if let Some(问题) = 轮.get("问题").and_then(|v| v.as_array()) {
                for q in 问题 {
                    if let Some(s) = q.as_str() {
                        片段.push(s.to_string());
                    }
                }
            }
            if let Some(建议) = 轮.get("建议").and_then(|v| v.as_str()) {
                片段.push(建议.to_string());
            }
        }
    }
    if 片段.is_empty() {
        截断(答复, 200).to_string()
    } else {
        片段.join("；")
    }
}
