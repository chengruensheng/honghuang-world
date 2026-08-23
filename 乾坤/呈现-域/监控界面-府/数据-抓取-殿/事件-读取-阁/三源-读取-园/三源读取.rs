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

/// 三源路径根——按优先级解析：
/// 1. 环境变量 `WORLD_WORKSPACE_ROOT`（部署时显式注入，优先级最高）
/// 2. 可执行文件路径上溯四层，验证含 Cargo.toml 即项目根
/// 3. 当前目录（兜底）
fn 上下文根() -> PathBuf {
    if let Ok(根) = std::env::var("WORLD_WORKSPACE_ROOT") {
        return PathBuf::from(根).join(".上下文");
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut p = exe.as_path();
        for _ in 0..6 {
            if let Some(父) = p.parent() {
                p = 父;
            } else {
                break;
            }
        }
        if p.join("Cargo.toml").exists() {
            return p.join(".上下文");
        }
    }
    PathBuf::from(".上下文")
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
pub fn 读全部() -> Vec<白箱事件> {
    let mut 事件 = Vec::new();
    事件.extend(读事件流(0, 0));
    事件.extend(读观测记录(0, 0));
    事件.extend(读识海记录(0, 0));
    事件.sort_by_key(|e| e.ts);
    事件
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
    let 证据 = 取字符串(&值, "证据").unwrap_or_default();
    let 任务线id = 取字符串(&值, "任务线id")
        .or_else(|| 取字符串(&值, "任务线"))
        .unwrap_or_default();
    let 轮次 = 值.get("载荷").and_then(|p| p.get("轮次")).and_then(|v| v.as_u64());
    白箱事件 {
        ts,
        源,
        动作,
        影响,
        token,
        耗时ms,
        证据,
        任务线id,
        轮次,
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
    let 动作 = 接口;
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
    let token = 取token(&值);
    let 证据 = 取字符串(&值, "载荷")
        .or_else(|| {
            值.get("载荷")
                .and_then(|v| v.get("内容"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    let 轮次 = 值.get("载荷").and_then(|p| p.get("轮次")).and_then(|v| v.as_u64());
    白箱事件 {
        ts,
        源,
        动作,
        影响,
        token,
        耗时ms: 0,
        证据,
        任务线id,
        轮次,
    }
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
    let 证据 = 取字符串(&值, "证据")
        .or_else(|| 取字符串(&值, "内容"))
        .unwrap_or_default();
    let 任务线id = 取字符串(&值, "任务线id")
        .or_else(|| 取字符串(&值, "任务线"))
        .unwrap_or_default();
    let 轮次 = 值.get("载荷").and_then(|p| p.get("轮次")).and_then(|v| v.as_u64());
    白箱事件 {
        ts,
        源,
        动作,
        影响,
        token,
        耗时ms,
        证据,
        任务线id,
        轮次,
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
            缓存写: 取u64对象(对象, "缓存写"),
            推理: 取u64对象(对象, "推理"),
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
        assert_eq!(事件.动作, "模型连接-府::调用模型");
        assert_eq!(事件.影响.len(), 1);
        assert_eq!(事件.影响[0].类型, "要求");
        assert_eq!(事件.影响[0].名, "要求-1");
    }

    #[test]
    fn 取三源大小文件不存在为零() {
        let 大小 = 取三源大小();
        // 不假设文件存在，只验证不 panic
        let _ = 大小.事件流 + 大小.观测记录 + 大小.识海记录;
    }
}
