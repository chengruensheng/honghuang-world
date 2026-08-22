//! 三源读取 —— 读三源 jsonl 文件，装配为白箱事件列表。
//!
//! 三源（依据融合蓝图 §11.5.3 与 README "服务端契约 v3.1"）：
//! - `.上下文/事件流.jsonl`——事件流（天庭主流程）
//! - `.上下文/观测/记录.jsonl`——观测记录（jiance_fu 写入）
//! - `.上下文/记录.jsonl`——识海记录（格位/扫描/会话等）
//!
//! 每条原始行缺字段填默认值，统一装配为白箱六字段。读失败静默返回空（不阻断直播）。

use std::path::PathBuf;

use crate::{token用量, 事件源, 影响项, 白箱事件};

/// 三源路径根——优先读环境 `WORLD_WORKSPACE_ROOT`，否则用当前目录。
fn 上下文根() -> PathBuf {
    if let Ok(根) = std::env::var("WORLD_WORKSPACE_ROOT") {
        PathBuf::from(根).join(".上下文")
    } else {
        PathBuf::from(".上下文")
    }
}

/// 事件流文件路径：`.上下文/事件流.jsonl`。
pub fn 事件流路径() -> PathBuf {
    上下文根().join("事件流.jsonl")
}

/// 观测记录文件路径：`.上下文/观测/记录.jsonl`。
pub fn 观测记录路径() -> PathBuf {
    上下文根().join("观测").join("记录.jsonl")
}

/// 识海记录文件路径：`.上下文/记录.jsonl`。
pub fn 识海记录路径() -> PathBuf {
    上下文根().join("记录.jsonl")
}

/// 三源文件大小快照——SSE 增量检测用。
#[derive(Debug, Clone, Copy, Default)]
pub struct 三源大小 {
    pub 事件流: u64,
    pub 观测记录: u64,
    pub 识海记录: u64,
}

/// 取三源文件当前大小（字节）；文件不存在记 0。
pub fn 取三源大小() -> 三源大小 {
    三源大小 {
        事件流: 文件大小(&事件流路径()),
        观测记录: 文件大小(&观测记录路径()),
        识海记录: 文件大小(&识海记录路径()),
    }
}

fn 文件大小(路径: &PathBuf) -> u64 {
    std::fs::metadata(路径).map(|m| m.len()).unwrap_or(0)
}

/// 读 jsonl 文件指定字节范围 [起, 止)，返回行列表（每行一个 JSON 值）。
/// 起为零则从头读；止为零或超过文件大小则读到末尾。
fn 读范围(路径: &PathBuf, 起: u64, 止: u64) -> Vec<serde_json::Value> {
    let Ok(内容) = std::fs::read_to_string(路径) else {
        return Vec::new();
    };
    let 字节 = 内容.as_bytes();
    let 总长 = 字节.len() as u64;
    if 总长 == 0 || 起 >= 总长 {
        return Vec::new();
    }
    // 止=0 表示读到末尾
    let 止 = if 止 == 0 { 总长 } else { 止.min(总长) };
    let 止 = 止.max(起);
    // 按 utf8 边界对齐：起跳到下一个换行后，止跳到下一个换行后
    let 起字节 = 对齐行首(字节, 起 as usize);
    let 止字节 = 对齐行首(字节, 止 as usize).max(起字节);
    if 起字节 >= 止字节 {
        return Vec::new();
    }
    let 切片 = &内容[起字节..止字节];
    切片
        .lines()
        .filter_map(|行| {
            let 行 = 行.trim();
            if 行.is_empty() {
                None
            } else {
                serde_json::from_str(行).ok()
            }
        })
        .collect()
}

/// 把偏移对齐到下一行行首（找下一个 \n 后一位）；已在一行行首则不变。
fn 对齐行首(字节: &[u8], 偏移: usize) -> usize {
    if 偏移 == 0 {
        return 0;
    }
    if 偏移 >= 字节.len() {
        return 字节.len();
    }
    // 若偏移正好在行首（前一字节是 \n 或偏移为 0），直接返回
    if 字节[偏移 - 1] == b'\n' {
        return 偏移;
    }
    // 否则找下一个 \n
    match 字节[偏移..].iter().position(|&b| b == b'\n') {
        Some(相对) => 偏移 + 相对 + 1,
        None => 字节.len(),
    }
}

/// 读三源全部事件，合并为白箱事件列表（按 ts 升序）。
///
/// 观测记录可能很大（>500MB），只读最近 2MB 避免内存膨胀——监控界面看最近事件，不需要历史全部。
/// 事件流和识海记录通常较小，全量读取。
pub fn 读全部() -> Vec<白箱事件> {
    let mut 事件 = Vec::new();
    事件.extend(读事件流(0, 0));
    let 观测上限: u64 = 2 * 1024 * 1024; // 2MB
    let 观测大小 = 文件大小(&观测记录路径());
    let 观测起 = 观测大小.saturating_sub(观测上限);
    事件.extend(读观测记录(观测起, 0));
    事件.extend(读识海记录(0, 0));
    事件.sort_by_key(|e| e.ts);
    事件
}

/// 截断证据到 2000 字符——避免载荷字段（LLM 完整响应）膨胀内存。
fn 截断证据(证据: String) -> String {
    证据.chars().take(2000).collect()
}

/// 读最近 N 条事件（三源合并，按 ts 倒序）。
pub fn 读最近(条数: usize) -> Vec<白箱事件> {
    let mut 事件 = 读全部();
    事件.sort_by_key(|e| std::cmp::Reverse(e.ts));
    事件.truncate(条数);
    事件
}

/// 读事件流文件指定字节范围，装配为白箱事件。
pub fn 读事件流(起: u64, 止: u64) -> Vec<白箱事件> {
    let 原始 = 读范围(&事件流路径(), 起, 止);
    原始.into_iter().map(装配事件流).collect()
}

/// 读观测记录文件指定字节范围，装配为白箱事件。
pub fn 读观测记录(起: u64, 止: u64) -> Vec<白箱事件> {
    let 原始 = 读范围(&观测记录路径(), 起, 止);
    原始.into_iter().map(装配观测记录).collect()
}

/// 读识海记录文件指定字节范围，装配为白箱事件。
pub fn 读识海记录(起: u64, 止: u64) -> Vec<白箱事件> {
    let 原始 = 读范围(&识海记录路径(), 起, 止);
    原始.into_iter().map(装配识海记录).collect()
}

/// 装配事件流的一行为白箱事件。
/// 事件流原文已是事件结构，直接取字段；缺字段填默认。
fn 装配事件流(值: serde_json::Value) -> 白箱事件 {
    let ts = 取u64(&值, "ts")
        .or_else(|| 取u64(&值, "时间戳"))
        .unwrap_or(0);
    let 源 = 取字符串(&值, "源")
        .or_else(|| 取字符串(&值, "源府"))
        .unwrap_or_else(|| "事件流".to_string());
    let 动作 = 取字符串(&值, "动作")
        .or_else(|| 取字符串(&值, "类型"))
        .unwrap_or_else(|| "未知".to_string());
    let 影响 = 取影响(&值);
    let token = 取token(&值);
    let 耗时ms = 取u64(&值, "耗时ms")
        .or_else(|| 取u64(&值, "耗时"))
        .unwrap_or(0);
    let 证据 = 截断证据(取字符串(&值, "证据").unwrap_or_default());
    let 任务线id = 取字符串(&值, "任务线id")
        .or_else(|| 取字符串(&值, "任务线"))
        .unwrap_or_default();
    白箱事件 {
        ts,
        源,
        动作,
        影响,
        token,
        耗时ms,
        证据,
        任务线id,
    }
}

/// 装配观测记录的一行为白箱事件。
/// 观测记录由 jiance_fu 写入，字段为：时间戳 / 域 / 接口 / 角色 / 载荷 / 关联。
fn 装配观测记录(值: serde_json::Value) -> 白箱事件 {
    let ts = 取u64(&值, "时间戳")
        .or_else(|| 取u64(&值, "ts"))
        .unwrap_or(0);
    let 域 = 取字符串(&值, "域").unwrap_or_else(|| "未知".to_string());
    let 接口 = 取字符串(&值, "接口").unwrap_or_else(|| "未知".to_string());
    let 角色 = 取字符串(&值, "角色").unwrap_or_else(|| "未知".to_string());
    let 源 = format!("观测/{}·{}", 域, 角色);
    // 动作名按域映射为可读描述——界主看不懂接口名（如"模型连接-府::调用模型带工具"）。
    let 动作 = match 域.as_str() {
        "提示词" => "发送提示词".to_string(),
        "回复思考" => "模型思考".to_string(),
        "回复内容" => "模型回复".to_string(),
        "工具调用" => "工具调用".to_string(),
        "工具返回" => "工具返回".to_string(),
        _ => 接口,
    };
    let mut 影响 = Vec::new();
    let mut 任务线id = String::new();
    if let Some(关联) = 值.get("关联").and_then(|v| v.as_object()) {
        if let Some(要求) = 关联.get("要求").and_then(|v| v.as_str()) {
            影响.push(影响项::新("要求", 要求));
        }
        if let Some(任务线) = 关联.get("任务线").and_then(|v| v.as_str()) {
            影响.push(影响项::新("任务线", 任务线));
            任务线id = 任务线.to_string();
        }
    }
    // token / 耗时 从载荷.附加提取——观测记录顶层无此字段，附加平铺"总计/提示词/缓存命中/输出"。
    // token 数据源分布：回复思考/回复内容域的附加含完整 token 四档（模型调用消耗）；
    // 提示词/工具调用/工具返回域的附加无 token 字段（这些域不消耗 token，属正常）。
    // 若观测记录的附加无 token 字段，取附加token 返回默认零值，不阻断装配。
    let 附加 = 值.get("载荷").and_then(|v| v.get("附加"));
    let token = 附加.map(取附加token).unwrap_or_default();
    let 耗时ms = 附加
        .and_then(|v| v.get("耗时ms").or_else(|| v.get("耗时")))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let 证据 = 截断证据(提取可读证据(&值, &域));
    白箱事件 {
        ts,
        源,
        动作,
        影响,
        token,
        耗时ms,
        证据,
        任务线id,
    }
}

/// 从载荷.附加提取 token 用量——附加平铺"总计/提示词/缓存命中/输出"四档。
/// 字段名与白箱 token用量 不同（"缓存命中"对应"缓存"），此处做映射。
fn 取附加token(附加: &serde_json::Value) -> token用量 {
    if let Some(对象) = 附加.as_object() {
        token用量 {
            提示词: 取u64对象(对象, "提示词"),
            输出: 取u64对象(对象, "输出"),
            缓存: 取u64对象(对象, "缓存命中"),
            总计: 取u64对象(对象, "总计"),
        }
    } else {
        token用量::default()
    }
}

/// 按域提取人类可读证据——避免界主看到一坨原始 JSON。
///
/// 各域提取规则：
/// - 提示词：载荷.内容是 JSON 字符串（含 messages 数组），取末条 content 前 80 字。
/// - 回复思考 / 回复内容：载荷.内容是模型回复文本，跳过 think 块取正文前 150 字。
/// - 工具调用：载荷.内容是命令行或 JSON，JSON 含"命令"字段时格式化，否则取前 100 字。
/// - 工具返回：载荷.内容是文本（退出码 / 标准输出等），取前 150 字。
/// - 其他：取前 100 字。
fn 提取可读证据(值: &serde_json::Value, 域: &str) -> String {
    let 内容 = 值
        .get("载荷")
        .and_then(|v| v.get("内容"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    match 域 {
        "提示词" => 提取提示词证据(内容),
        "回复思考" | "回复内容" => 提取回复证据(内容),
        "工具调用" => 提取工具调用证据(内容),
        "工具返回" => 取前(内容, 150),
        _ => 取前(内容, 100),
    }
}

/// 提示词证据：内容是 JSON 字符串（含 messages 数组），取末条 content 前 80 字。
/// 格式："提示词 N 条消息，末条：xxx"。解析失败则取内容前 100 字兜底。
fn 提取提示词证据(内容: &str) -> String {
    let 解析: serde_json::Value = match serde_json::from_str(内容) {
        Ok(v) => v,
        Err(_) => return 取前(内容, 100),
    };
    let 消息们 = match 解析.get("messages").and_then(|v| v.as_array()) {
        Some(数组) => 数组,
        None => return 取前(内容, 100),
    };
    let 数 = 消息们.len();
    if 数 == 0 {
        return "提示词 0 条消息".to_string();
    }
    let 末内容 = 消息们[数 - 1]
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    format!("提示词 {数} 条消息，末条：{}", 取前(末内容, 80))
}

/// 回复证据：跳过 think 块取正文前 150 字。
/// think 块形如 `...` 后跟正文；无 think 块则原样取前 150 字。
fn 提取回复证据(内容: &str) -> String {
    let 正文 = 跳过think块(内容);
    取前(&正文, 150)
}

/// 跳过 `...` 块取正文。无 think 块则原样返回。
fn 跳过think块(内容: &str) -> String {
    if let Some(止) = 内容.find("") {
        内容[止 + "".len()..].trim_start().to_string()
    } else {
        内容.to_string()
    }
}

/// 工具调用证据：内容可能是命令行文本（如 `cargo ["build", "--workspace"]`）或 JSON。
/// JSON 含"命令"字段时格式化为"命令：xxx 参数：yyy"；否则取前 100 字。
fn 提取工具调用证据(内容: &str) -> String {
    if let Ok(解析) = serde_json::from_str::<serde_json::Value>(内容) {
        if let Some(命令) = 解析.get("命令").and_then(|v| v.as_str()) {
            let 参数 = 解析.get("参数").and_then(|v| v.as_str()).unwrap_or("");
            if 参数.is_empty() {
                return format!("命令：{命令}");
            }
            return format!("命令：{命令} 参数：{参数}");
        }
    }
    取前(内容, 100)
}

/// 取字符串前 N 个字符（按 char 边界，不截断多字节字符）。
fn 取前(文本: &str, n: usize) -> String {
    文本.chars().take(n).collect()
}

/// 装配识海记录的一行为白箱事件。
/// 识海记录字段较松散，按常见字段名兜底。
fn 装配识海记录(值: serde_json::Value) -> 白箱事件 {
    let ts = 取u64(&值, "ts")
        .or_else(|| 取u64(&值, "时间戳"))
        .or_else(|| 取u64(&值, "时刻"))
        .unwrap_or(0);
    let 源 = 取字符串(&值, "源")
        .or_else(|| 取字符串(&值, "府"))
        .unwrap_or_else(|| "识海".to_string());
    let 动作 = 取字符串(&值, "动作")
        .or_else(|| 取字符串(&值, "类型"))
        .or_else(|| 取字符串(&值, "操作"))
        .unwrap_or_else(|| "记录".to_string());
    let 影响 = 取影响(&值);
    let token = 取token(&值);
    let 耗时ms = 取u64(&值, "耗时ms")
        .or_else(|| 取u64(&值, "耗时"))
        .unwrap_or(0);
    let 证据 = 截断证据(
        取字符串(&值, "证据")
            .or_else(|| 取字符串(&值, "内容"))
            .unwrap_or_default(),
    );
    let 任务线id = 取字符串(&值, "任务线id")
        .or_else(|| 取字符串(&值, "任务线"))
        .unwrap_or_default();
    白箱事件 {
        ts,
        源,
        动作,
        影响,
        token,
        耗时ms,
        证据,
        任务线id,
    }
}

/// 装配影响项列表——从 JSON 值的 "影响" 数组或单条 "影响" 对象取。
fn 取影响(值: &serde_json::Value) -> Vec<影响项> {
    let mut 结果 = Vec::new();
    if let Some(数组) = 值.get("影响").and_then(|v| v.as_array()) {
        for 项 in 数组 {
            let 类型 = 取字符串(项, "类型").unwrap_or_else(|| "未知".to_string());
            let 名 = 取字符串(项, "名")
                .or_else(|| 取字符串(项, "路径"))
                .unwrap_or_default();
            let 变化 = 取字符串(项, "变化").unwrap_or_default();
            let 字节 = 取u64(项, "字节");
            结果.push(影响项 {
                类型,
                名,
                变化,
                字节,
            });
        }
    }
    结果
}

/// 装配 token 用量——从 JSON 值的 "token" 对象取四档。
fn 取token(值: &serde_json::Value) -> token用量 {
    if let Some(对象) = 值.get("token").and_then(|v| v.as_object()) {
        token用量 {
            提示词: 取u64对象(对象, "提示词"),
            输出: 取u64对象(对象, "输出"),
            缓存: 取u64对象(对象, "缓存"),
            总计: 取u64对象(对象, "总计"),
        }
    } else {
        token用量::default()
    }
}

fn 取u64(值: &serde_json::Value, 键: &str) -> Option<u64> {
    值.get(键).and_then(|v| v.as_u64())
}

fn 取u64对象(对象: &serde_json::Map<String, serde_json::Value>, 键: &str) -> u64 {
    对象.get(键).and_then(|v| v.as_u64()).unwrap_or(0)
}

fn 取字符串(值: &serde_json::Value, 键: &str) -> Option<String> {
    值.get(键).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// 取三源文件存在标志。
pub fn 三源就绪() -> crate::三源就绪 {
    crate::三源就绪 {
        事件流: 事件流路径().exists(),
        观测记录: 观测记录路径().exists(),
        识海记录: 识海记录路径().exists(),
    }
}

/// 取三源枚举值——供 SSE 推送标注来源。
pub fn 三源枚举() -> [事件源; 3] {
    [事件源::事件流, 事件源::观测记录, 事件源::识海记录]
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use std::io::Write;

    fn 写临时(路径: &PathBuf, 内容: &str) {
        if let Some(父) = 路径.parent() {
            std::fs::create_dir_all(父).unwrap();
        }
        let mut f = std::fs::File::create(路径).unwrap();
        f.write_all(内容.as_bytes()).unwrap();
    }

    #[test]
    fn 读范围对齐行首() {
        let 临时 = std::env::temp_dir().join("jiankong_fu_test_对齐.jsonl");
        写临时(&临时, "{\"ts\":1}\n{\"ts\":2}\n{\"ts\":3}\n");
        let 行 = 读范围(&临时, 0, 0);
        assert_eq!(行.len(), 3);
        // 从第 8 字节开始读（落在第二行中间），应从第二行行首开始
        let 行 = 读范围(&临时, 8, 0);
        assert_eq!(行.len(), 2);
        assert_eq!(行[0]["ts"], 2);
        let _ = std::fs::remove_file(&临时);
    }

    #[test]
    fn 装配事件流缺字段填默认() {
        let 值: serde_json::Value =
            serde_json::from_str(r#"{"ts":100,"源":"测试","动作":"测试动作"}"#).unwrap();
        let 事件 = 装配事件流(值);
        assert_eq!(事件.ts, 100);
        assert_eq!(事件.源, "测试");
        assert_eq!(事件.动作, "测试动作");
        assert!(事件.影响.is_empty());
        assert_eq!(事件.耗时ms, 0);
    }

    #[test]
    fn 装配观测记录从jiance_fu字段() {
        let 值: serde_json::Value = serde_json::from_str(
            r#"{"时间戳":200,"域":"工具调用","接口":"模型连接-府::调用模型","角色":"执行","关联":{"要求":"要求-1"}}"#,
        ).unwrap();
        let 事件 = 装配观测记录(值);
        assert_eq!(事件.ts, 200);
        assert_eq!(事件.源, "观测/工具调用·执行");
        // 动作名按域映射为可读描述（域=工具调用 → 动作=工具调用），不再暴露接口名。
        assert_eq!(事件.动作, "工具调用");
        assert_eq!(事件.影响.len(), 1);
        assert_eq!(事件.影响[0].类型, "要求");
        assert_eq!(事件.影响[0].名, "要求-1");
        // 无载荷时证据为空，token / 耗时为默认 0。
        assert!(事件.证据.is_empty());
        assert_eq!(事件.token.总计, 0);
        assert_eq!(事件.耗时ms, 0);
    }

    #[test]
    fn 装配观测记录提取任务线id() {
        let 值: serde_json::Value = serde_json::from_str(
            r#"{"时间戳":300,"域":"工具调用","接口":"调用","角色":"执行","关联":{"要求":"要求-1","任务线":"线A"}}"#,
        ).unwrap();
        let 事件 = 装配观测记录(值);
        assert_eq!(事件.任务线id, "线A");
        // 影响项里也保留任务线（兼容旧消费方）
        assert!(事件
            .影响
            .iter()
            .any(|i| i.类型 == "任务线" && i.名 == "线A"));
    }

    #[test]
    fn 装配事件流提取任务线id() {
        let 值: serde_json::Value =
            serde_json::from_str(r#"{"ts":100,"源":"s","动作":"a","任务线id":"线B"}"#).unwrap();
        let 事件 = 装配事件流(值);
        assert_eq!(事件.任务线id, "线B");

        let 值2: serde_json::Value =
            serde_json::from_str(r#"{"ts":100,"源":"s","动作":"a","任务线":"线C"}"#).unwrap();
        let 事件2 = 装配事件流(值2);
        assert_eq!(事件2.任务线id, "线C");

        // 缺字段默认空串（主线）
        let 值3: serde_json::Value =
            serde_json::from_str(r#"{"ts":100,"源":"s","动作":"a"}"#).unwrap();
        let 事件3 = 装配事件流(值3);
        assert_eq!(事件3.任务线id, "");
    }

    #[test]
    fn 装配识海记录提取任务线id() {
        let 值: serde_json::Value =
            serde_json::from_str(r#"{"ts":100,"府":"识海","动作":"扫描","任务线id":"线D"}"#)
                .unwrap();
        let 事件 = 装配识海记录(值);
        assert_eq!(事件.任务线id, "线D");

        let 值2: serde_json::Value =
            serde_json::from_str(r#"{"ts":100,"府":"识海","动作":"扫描","任务线":"线E"}"#).unwrap();
        let 事件2 = 装配识海记录(值2);
        assert_eq!(事件2.任务线id, "线E");
    }

    #[test]
    fn 取三源大小文件不存在为零() {
        let 大小 = 取三源大小();
        // 不假设文件存在，只验证不 panic
        let _ = 大小.事件流 + 大小.观测记录 + 大小.识海记录;
    }

    /// 动作名按域映射：五种已知域映射为可读描述，未知域保留原接口名。
    #[test]
    fn 装配观测记录动作名按域映射() {
        let 造 = |域: &str, 接口: &str| {
            let 文本 = format!(
                r#"{{"时间戳":1,"域":"{}","接口":"{}","角色":"执行"}}"#,
                域, 接口
            );
            装配观测记录(serde_json::from_str(&文本).unwrap())
        };
        assert_eq!(造("提示词", "模型连接-府::调用模型").动作, "发送提示词");
        assert_eq!(造("回复思考", "模型连接-府::调用模型").动作, "模型思考");
        assert_eq!(造("回复内容", "模型连接-府::调用模型").动作, "模型回复");
        assert_eq!(造("工具调用", "工具循环::执行工具").动作, "工具调用");
        assert_eq!(造("工具返回", "工具循环::执行工具").动作, "工具返回");
        // 未知域保留原接口名
        assert_eq!(造("其他", "某接口").动作, "某接口");
    }

    /// 提示词证据：从 messages 数组取末条 content 前 80 字，不暴露整个 JSON。
    #[test]
    fn 提取可读证据_提示词取末条消息() {
        let 值: serde_json::Value = serde_json::from_str(
            r#"{"域":"提示词","载荷":{"内容":"{\"messages\":[{\"content\":\"第一条\"},{\"content\":\"第二条末条\"}]}"}}"#,
        ).unwrap();
        let 证据 = 提取可读证据(&值, "提示词");
        assert!(证据.contains("提示词 2 条消息"), "证据={证据}");
        assert!(证据.contains("末条：第二条末条"), "证据={证据}");
        // 不含整个 JSON 的 messages 数组原文
        assert!(!证据.contains("\"messages\""), "证据不应含原始JSON：{证据}");
    }

    /// 提示词证据：内容非合法 JSON 时取前 100 字兜底。
    #[test]
    fn 提取可读证据_提示词非JSON兜底() {
        let 值: serde_json::Value =
            serde_json::from_str(r#"{"域":"提示词","载荷":{"内容":"不是JSON的纯文本"}}"#).unwrap();
        let 证据 = 提取可读证据(&值, "提示词");
        assert_eq!(证据, "不是JSON的纯文本");
    }

    /// 回复证据：跳过 think 块取正文前 150 字。
    #[test]
    fn 提取可读证据_回复跳过think块() {
        let 值: serde_json::Value =
            serde_json::from_str(r#"{"域":"回复内容","载荷":{"内容":"这是正文"}}"#).unwrap();
        let 证据 = 提取可读证据(&值, "回复内容");
        assert_eq!(证据, "这是正文");
        assert!(!证据.contains("think"), "证据不应含think块：{证据}");
    }

    /// 工具调用证据：命令行文本（非合法 JSON）取前 100 字。
    #[test]
    fn 提取可读证据_工具调用命令行() {
        let 值: serde_json::Value = serde_json::from_str(
            r#"{"域":"工具调用","载荷":{"内容":"cargo [\"build\", \"--workspace\", \"--lib\"]"}}"#,
        )
        .unwrap();
        let 证据 = 提取可读证据(&值, "工具调用");
        assert!(证据.contains("cargo"), "证据={证据}");
    }

    /// 工具调用证据：JSON 含"命令"字段时格式化为"命令：xxx 参数：yyy"。
    #[test]
    fn 提取可读证据_工具调用JSON命令() {
        let 值: serde_json::Value = serde_json::from_str(
            r#"{"域":"工具调用","载荷":{"内容":"{\"命令\":\"cargo\",\"参数\":\"build --workspace\"}"}}"#,
        ).unwrap();
        let 证据 = 提取可读证据(&值, "工具调用");
        assert_eq!(证据, "命令：cargo 参数：build --workspace");
    }

    /// 工具返回证据：取前 150 字。
    #[test]
    fn 提取可读证据_工具返回取前150() {
        let 值: serde_json::Value = serde_json::from_str(
            r#"{"域":"工具返回","载荷":{"内容":"退出码：Some(0)\n标准输出：构建成功\n"}}"#,
        )
        .unwrap();
        let 证据 = 提取可读证据(&值, "工具返回");
        assert!(证据.contains("退出码：Some(0)"), "证据={证据}");
        assert!(证据.contains("构建成功"), "证据={证据}");
    }

    /// token / 耗时从载荷.附加提取——附加平铺"总计/提示词/缓存命中/输出"。
    #[test]
    fn 装配观测记录从附加提取token与耗时() {
        let 值: serde_json::Value = serde_json::from_str(
            r#"{"时间戳":1,"域":"回复内容","接口":"调用","角色":"执行","载荷":{"内容":"回复","附加":{"总计":1934,"提示词":1791,"模型":"MiniMax-M3","缓存命中":128,"输出":143,"耗时ms":2103}}}"#,
        ).unwrap();
        let 事件 = 装配观测记录(值);
        assert_eq!(事件.token.总计, 1934);
        assert_eq!(事件.token.提示词, 1791);
        assert_eq!(事件.token.缓存, 128, "缓存命中应映射到缓存字段");
        assert_eq!(事件.token.输出, 143);
        assert_eq!(事件.耗时ms, 2103);
    }

    /// 附加无耗时字段时耗时为 0。
    #[test]
    fn 装配观测记录附加无耗时默认零() {
        let 值: serde_json::Value = serde_json::from_str(
            r#"{"时间戳":1,"域":"工具调用","接口":"工具循环::执行工具","角色":"执行","载荷":{"内容":"cargo build","附加":{"工具":"运行命令"}}}"#,
        ).unwrap();
        let 事件 = 装配观测记录(值);
        assert_eq!(事件.耗时ms, 0);
        assert_eq!(事件.token.总计, 0);
    }
}
