use hm_cognition::AgentRole;
use hm_contract::当前时间戳;
use hm_error::{Error, Result};
use tc_task::{
    TaskBoard, TaskStatus, DesignDoc, ImplementationDoc, VerificationDoc, VerificationRound,
    FinalAcceptanceDoc, 五行层级, 无解声明, 审核记录, 审核来源, 驳回原因,
};
use crate::循环驱动_殿::智能体;
use crate::协作驱动_殿::五层驱动_阁::错误追溯_阁::追溯器;
use super::构建核验::核验结论;
use super::阶段提示::任务快照;

/// 阶段产出：写文档闭包 + 下一状态 + 机器核验结论（仅当本轮模型宣告「通过」并被机器推翻时存在）
pub(crate) struct 阶段产出 {
    pub(crate) 写文档: Box<dyn FnOnce(&mut TaskBoard, u64) -> Result<()>>,
    pub(crate) 下一状态: TaskStatus,
    pub(crate) 核验: Option<核验结论>,
}

/// 自由文本强无解特征词。
///
/// 经全量历史设计文档（30+ 任务）扫描标定：命中者 100% 为加压（无解）任务，
/// 5 个正例任务（fib/sort/stack/cli/表达式求值）零命中——实测精确率 1.0。
/// 这些词描述「契约本身不可满足」，正常可解需求的设计文档不会出现。
const 无解强特征: &[&str] = &[
    "不可满足",
    "无法同时满足",
    "不可能被同时满足",
    "不可能满足",
    "硬无解",
    "无解命题",
];

/// 自由文本无解迹象识别：设计层把无解判定写在「边界定义/契约描述」自然语言里、
/// 却漏填结构化 `无解声明` 字段时兜底识别（补填声明并留痕，不静默改状态）。
fn 从文本识别无解(doc: &DesignDoc) -> Option<无解声明> {
    let mut 全集 = String::new();
    for v in doc.边界定义.values() {
        全集.push_str(v);
        全集.push('\n');
    }
    for c in &doc.契约 {
        全集.push_str(&c.描述);
        全集.push('\n');
        for m in &c.方法 {
            全集.push_str(&m.描述);
            全集.push('\n');
        }
    }
    let 命中 = 无解强特征.iter().find(|k| 全集.contains(**k))?;
    Some(无解声明 {
        契约名: doc
            .契约
            .first()
            .map(|c| c.契约名.clone())
            .unwrap_or_else(|| "（未指明）".into()),
        判定依据: format!(
            "系统兜底识别：设计文档自由文本出现强无解特征「{命中}」（设计层已作无解判定但未填结构化「无解声明」字段）"
        ),
        类型: "系统兜底识别".into(),
    })
}

/// 实现落盘核验：实现层（大罗金仙）声称「新建/修改」的文件必须真实存在于工作区。
///
/// 背景（2026-09-14 实证）：#51 连续 6 次跨层回退中，实现层整轮只调了 3 次 list_dir、零写操作，
/// 却输出格式完整的实现文档（含「工具调用」数组与「自检.通过: true」）。此前链路只校验文档**格式**、
/// 不校验**事实**，准圣只能靠人工比对目录才发现交付物根本不存在。此门直接查盘，当场判非法并打回重做。
///
/// 返回 `Some(原因)` 表示核验未通过；`None` 表示通过或本任务不适用。仅核「应存在」的变更：
/// 变更类型含「删除」的条目跳过存在性检查（清理由太乙金仙的残留核验门负责）。
fn 核验实现落盘(工作区根: Option<&str>, doc: &ImplementationDoc) -> Option<String> {
    let 根 = 工作区根?;
    if doc.代码变更.is_empty() {
        return None;
    }
    // 声称有代码变更却没有任何工具调用 —— 自相矛盾，必为编造
    if doc.工具调用.is_empty() {
        return Some(format!(
            "实现文档声称 {} 项代码变更，但「工具调用」数组为空：没有任何实际写盘动作。\
             请真实调用 write_file / edit_file 落盘后重新输出实现文档，不得只给文档。",
            doc.代码变更.len()
        ));
    }
    let 缺失: Vec<String> = doc
        .代码变更
        .iter()
        .filter(|变更| !变更.变更类型.contains("删除"))
        .map(|变更| 变更.文件路径.trim().to_string())
        .filter(|路径| 路径.is_empty() || !解析工作区路径(根, 路径).exists())
        .collect();
    if 缺失.is_empty() {
        return None;
    }
    Some(format!(
        "以下文件被声明为新建/修改，但在工作区中并不存在（判定为编造交付）：{}。\
         请真实调用 write_file / edit_file 把文件写进工作区，且路径须与文档中所写一致，然后重新输出实现文档。",
        缺失.join("、")
    ))
}

/// 把实现文档里的文件路径解析为工作区下的真实路径。
///
/// 模型可能写成 `工作区/a.rs`、`./a.rs` 或反斜杠分隔，统一归一化后再与工作区根拼接；
/// 绝对路径原样使用（写文件工具本身会拒绝越界，此处只管「文件在不在」）。
fn 解析工作区路径(工作区根: &str, 路径: &str) -> std::path::PathBuf {
    let 归一 = 路径.replace('\\', "/");
    let 归一 = 归一.trim().trim_start_matches("./");
    let 归一 = 归一.strip_prefix("工作区/").unwrap_or(归一);
    let 路径对象 = std::path::Path::new(归一);
    if 路径对象.is_absolute() {
        路径对象.to_path_buf()
    } else {
        std::path::Path::new(工作区根).join(路径对象)
    }
}

/// 解析阶段产出：提取 JSON → 注入 created_at → 反序列化为目标文档 → 返回 阶段产出。
///
/// `核验器` 是机器核验门的惰性求值入口：**仅当模型宣告「验收通过 / 审核通过」时才调用**，
/// 由系统真实执行编译与测试判定；判定不通过则推翻模型结论，改写文档并把任务打回实现层。
///
/// `工作区根` 供实现层落盘核验用（见 `核验实现落盘`）；未配置工作区（如单元测试）时跳过核验。
pub(crate) fn 解析并构造(
    角色: &AgentRole,
    状态: TaskStatus,
    答复: &str,
    核验器: &dyn Fn() -> 核验结论,
    工作区根: Option<&str>,
) -> Result<阶段产出> {
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
            let mut doc: DesignDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("设计文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            // 兜底：结构化「无解声明」为空时，扫描自由文本的强无解特征——
            // 设计层常把无解判定写在「边界定义」自然语言里（如鸽巢原理证明），却漏填结构化字段，
            // 导致下游读不到而继续实现（实测：无损压缩任务圣人写下完整信息论无解证明却填 null，
            // 终审又以「诚实标注 + 最优近似」为由判通过 → 造假）。此处按规则兜底，不依赖模型遵守格式。
            if doc.无解声明.is_none() {
                if let Some(声明) = 从文本识别无解(&doc) {
                    tracing::warn!("设计层未填「无解声明」字段但自由文本含强无解特征，系统兜底认定为无解：{}", 声明.判定依据);
                    doc.无解声明 = Some(声明);
                }
            }
            // 设计层判定契约不可满足（数学/信息论无解）时，据实上报 → 直接进「已确认无解」终态，
            // 不再把契约降级后进入实现（否则下游要么「降级实现后自报通过」造假，要么「拒绝实现后被验收反复打回」卡死）。
            let 下一状态 = if doc.无解声明.is_some() {
                TaskStatus::已确认无解
            } else {
                TaskStatus::待大罗金仙实现
            };
            Ok(阶段产出 {
                写文档: Box::new(move |看板, id| 看板.更新设计文档(id, doc)),
                下一状态,
                核验: None,
            })
        }
        AgentRole::大罗金仙 => {
            let doc: ImplementationDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("实现文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            // 落盘核验门：宣称的代码变更必须真实存在于工作区，否则判为「编造交付」直接打回重做
            if let Some(原因) = 核验实现落盘(工作区根, &doc) {
                return Err(Error::反序列化(format!("实现落盘核验未通过：{原因}")));
            }
            Ok(阶段产出 {
                写文档: Box::new(move |看板, id| 看板.更新实现文档(id, doc)),
                下一状态: TaskStatus::待准圣验收,
                核验: None,
            })
        }
        AgentRole::准圣 => {
            let mut doc: VerificationDoc = serde_json::from_value(值)
                .map_err(|e| Error::反序列化(format!("验收文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
            let 模型宣告通过 = doc.最终结果;
            // 机器核验门：模型宣告验收通过时，系统实跑编译与测试独立复核；
            // 不通过则推翻模型结论（改写文档 + 打回实现层），杜绝「自报通过」。
            let 核验 = if 模型宣告通过 {
                let 结论 = 核验器();
                if !结论.通过 {
                    tracing::warn!("机器核验推翻模型验收结论：{}", 结论.摘要);
                    doc.最终结果 = false;
                    doc.轮次.push(VerificationRound {
                        轮次: doc.轮次.len() as u32 + 1,
                        通过: false,
                        边界检查: true,
                        契约检查: true,
                        安全检查: true,
                        事实检查: false,
                        完整性检查: false,
                        问题: vec![结论.摘要.clone()],
                        建议: "按机器核验给出的真实编译/测试错误修正实现后重新提交验收".to_string(),
                    });
                }
                Some(结论)
            } else {
                None
            };
            let 下一状态 = if doc.最终结果 { TaskStatus::待道祖终审 } else { TaskStatus::待修复 };
            Ok(阶段产出 {
                写文档: Box::new(move |看板, id| 看板.更新验收文档(id, doc)),
                下一状态,
                核验,
            })
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
                // 机器核验门：最终审核宣告通过前同样实跑复核，不通过即视为「实现错误」驳回
                let 核验 = if 通过 { Some(核验器()) } else { None };
                let 机器否决 = 核验.as_ref().is_some_and(|c| !c.通过);
                if let Some(结论) = &核验 {
                    if !结论.通过 {
                        tracing::warn!("机器核验推翻模型最终审核结论：{}", 结论.摘要);
                    }
                }
                let (实际通过, 实际驳回原因, 实际评语) = if 机器否决 {
                    let 结论 = 核验.as_ref().map(|c| c.摘要.clone()).unwrap_or_default();
                    (false, Some(驳回原因::实现错误), format!("机器核验否决：{结论}（原模型评语：{评语}）"))
                } else {
                    (通过, 驳回原因, 评语)
                };
                let 记录 = 审核记录::新(当前时间戳(), 实际通过, 实际驳回原因, 实际评语, 审核来源::自动);
                let 下一状态 = if 实际通过 { TaskStatus::待清理 } else { TaskStatus::待修复 };
                Ok(阶段产出 {
                    写文档: Box::new(move |看板, id| 看板.更新审核记录(id, 记录)),
                    下一状态,
                    核验,
                })
            } else {
                let doc: FinalAcceptanceDoc = serde_json::from_value(值)
                    .map_err(|e| Error::反序列化(format!("终审文档解析失败: {e}；原始 JSON 前 200 字：{}", 截断(&json, 200))))?;
                let 下一状态 = if doc.通过 { TaskStatus::待人工验收 } else { TaskStatus::待修复 };
                Ok(阶段产出 {
                    写文档: Box::new(move |看板, id| 看板.更新终审文档(id, doc)),
                    下一状态,
                    核验: None,
                })
            }
        }
        AgentRole::太乙金仙 => {
            // 清理阶段无文档槽位（Task 未扩展字段）：校验顶层对象即可，推进到 清理完成
            if !值.is_object() {
                return Err(Error::反序列化(format!("清理记录 JSON 顶层必须是对象，前 200 字：{}", 截断(&json, 200))));
            }
            Ok(阶段产出 {
                写文档: Box::new(|_看板, _id| Ok(())),
                下一状态: TaskStatus::清理完成,
                核验: None,
            })
        }
    }
}

/// 解析阶段产出（带失败重试）：首次校验失败时，把失败原因原文回喂 LLM 修正重试，上限 2 次；
/// 重试仍失败才返回 Err（任务保持待承接可重试，不死等）。
///
/// 「失败」包含两类：JSON 无法解析，以及实现层落盘核验未通过（文档格式合法但事实不成立）。
pub(crate) fn 解析并构造带重试(
    角色: &AgentRole,
    状态: TaskStatus,
    答复: &str,
    阶段提示: &str,
    智能体: &智能体,
    核验器: &dyn Fn() -> 核验结论,
    工作区根: Option<&str>,
) -> Result<阶段产出> {
    let mut 当前答复 = 答复.to_string();
    let mut 重试 = 0;
    loop {
        match 解析并构造(角色, 状态, &当前答复, 核验器, 工作区根) {
            Ok(结果) => return Ok(结果),
            Err(错误) => {
                if 重试 >= 2 {
                    return Err(错误);
                }
                重试 += 1;
                let 修复提示 = format!(
                    "{}\n\n【修正要求】你上一轮输出未通过系统校验，具体问题：\n{}\n请按上述问题真实执行必要的工具调用，然后严格只输出一个 JSON 对象，不要输出任何解释文字，不要用 markdown 代码块包裹。重新输出。",
                    阶段提示, 错误
                );
                tracing::warn!("阶段产出校验失败，回喂错误重试（第 {}/2 次）：{}", 重试, 错误);
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
