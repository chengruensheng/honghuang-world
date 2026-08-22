//! 白箱装配 —— 事件契约与白箱六字段类型定义。
//!
//! 依据：融合蓝图-设计稿.md §9.3 白箱六字段契约。
//! 每条进入直播的事件必须含六字段：ts / 源 / 动作 / 影响 / token / 耗时ms（证据可选）。
//! 缺一即白箱泄漏——服务端拒推此事件，客户端视为"直播黑洞"。

#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};

/// 白箱六字段事件——直播与回放的最小契约单元。
///
/// 字段名按 §9.3 中文契约，序列化后前端按字面字段名解析。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct 白箱事件 {
    /// 时刻（毫秒，u64）。
    pub ts: u64,
    /// 来源庭（含维度+域+府，如 "鸿蒙/道术施展-府"）。
    pub 源: String,
    /// 动作名（动词 + 步骤号，如 "工具循环-7"）。
    pub 动作: String,
    /// 影响清单（白箱核心：格位/文件/状态等变化）。
    pub 影响: Vec<影响项>,
    /// token 用量四档。
    pub token: token用量,
    /// 该动作的执行时长（毫秒）。
    #[serde(rename = "耗时ms")]
    pub 耗时ms: u64,
    /// 证据（可选但建议带：原始 token / 文件 hash / 命令行 等）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub 证据: String,
    /// 任务线id——分裂流的分流键。空串表示主线/未分组。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub 任务线id: String,
}

/// 影响项——白箱影响清单的一条。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct 影响项 {
    /// 类型（格位 / 文件 / 状态 / 队列 等）。
    pub 类型: String,
    /// 名（格位名 / 文件路径 / 状态键 等）。
    pub 名: String,
    /// 变化（"+421" / "+1 条" / 189000 字节 等，可空）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub 变化: String,
    /// 字节数（可空）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub 字节: Option<u64>,
}

/// token 用量四档。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct token用量 {
    /// 提示词 token。
    #[serde(default)]
    pub 提示词: u64,
    /// 输出 token。
    #[serde(default)]
    pub 输出: u64,
    /// 缓存 token。
    #[serde(default)]
    pub 缓存: u64,
    /// 总计 token。
    #[serde(default)]
    pub 总计: u64,
}

/// 事件来源——三源白箱。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum 事件源 {
    /// `.上下文/事件流.jsonl`——天庭主流程（想法/要求/设计/实现/验收/版本）。
    #[serde(rename = "event")]
    事件流,
    /// `.上下文/观测/记录.jsonl`——观测记录（jiance_fu 写入）。
    #[serde(rename = "model")]
    观测记录,
    /// `.上下文/记录.jsonl`——识海记录（格位/扫描/会话等非天庭主流程）。
    #[serde(rename = "shihai")]
    识海记录,
}

impl 事件源 {
    /// 字面标识（用于 SSE payload 的 source 字段）。
    pub fn 字面(self) -> &'static str {
        match self {
            事件源::事件流 => "event",
            事件源::观测记录 => "model",
            事件源::识海记录 => "shihai",
        }
    }
}

/// SSE 推送的一条外层载荷——`data: {source, ts, ev}`。
#[derive(Debug, Clone, Serialize)]
pub struct SSE载荷 {
    /// 来源（event / model / shihai）。
    pub source: &'static str,
    /// 推送时刻（毫秒）。
    pub ts: u64,
    /// 白箱六字段事件。
    pub ev: 白箱事件,
}

/// 任务索引项——按 `_task_id` 聚合后的任务卡。
#[derive(Debug, Clone, Serialize)]
pub struct 任务索引项 {
    /// 任务 id（要求 id 或事件关联的任务线）。
    pub id: String,
    /// 摘要（动作或方向文本前 80 字）。
    pub 摘要: String,
    /// 状态（待领 / 待实现 / 实现中 / 已通过 / 已打回 / 未知）。
    pub 状态: String,
    /// 阶段（甲 / 乙 / 空）。
    pub 阶段: String,
    /// 该任务下的事件数。
    pub 事件数: usize,
    /// 该任务下的事件 ts 列表（升序）。
    pub 时间线: Vec<u64>,
    /// 累计 token 总计。
    pub 累计token: u64,
    /// 累计耗时毫秒。
    pub 累计耗时ms: u64,
}

/// 任务索引响应——`GET /api/tasks`。
#[derive(Debug, Clone, Serialize)]
pub struct 任务索引 {
    /// 任务列表（按最近活动倒序）。
    pub 任务: Vec<任务索引项>,
}

/// 世界状态快照——`GET /api/snapshot`。
#[derive(Debug, Clone, Serialize)]
pub struct 世界快照 {
    /// 当前想法——优先从事件流"想法投递"事件推断内容摘要，回退到 zhuangtai_fu 状态共享 id。可空。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub 当前想法: String,
    /// 当前要求——优先从事件流"要求入池"事件推断方向摘要，回退到 zhuangtai_fu 状态共享 id。可空。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub 当前要求: String,
    /// 当前阶段（v15甲 / v15乙 / 空）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub 当前阶段: String,
    /// 最近事件 ts（毫秒，0 表示无事件）。
    pub 最近事件ts: u64,
    /// 最近事件数（直播首帧条数）。
    pub 最近事件数: usize,
    /// 服务启动时刻（毫秒）。
    pub 启动时刻: u64,
}

/// 健康检查响应——`GET /api/health`。
#[derive(Debug, Clone, Serialize)]
pub struct 健康状态 {
    /// 状态（"ok" / "degraded"）。
    pub 状态: String,
    /// 服务运行时长（秒）。
    pub 运行秒: u64,
    /// 三源文件是否存在。
    pub 三源就绪: 三源就绪,
}

/// 三源就绪标志。
#[derive(Debug, Clone, Serialize)]
pub struct 三源就绪 {
    /// 事件流文件存在。
    pub 事件流: bool,
    /// 观测记录文件存在。
    pub 观测记录: bool,
    /// 识海记录文件存在。
    pub 识海记录: bool,
}

/// 拓扑段——分裂流的一段（串行 / 并行 / 汇流）。
///
/// 依据：融合蓝图-设计稿.md §13.d.6 数据贯通。
#[derive(Debug, Clone, Serialize)]
pub struct 拓扑段 {
    /// 段类型：串行 / 并行 / 汇流。
    pub 类型: String,
    /// 段起始 ts（毫秒）。
    pub ts: u64,
    /// 该段涉及的活跃任务线id列表（有序去重，主线记为 "主线"）。
    pub 线: Vec<String>,
    /// 该段内的白箱事件（按 ts 升序）。
    pub 事件: Vec<白箱事件>,
}

/// 拓扑——分裂流的段列表。
#[derive(Debug, Clone, Serialize)]
pub struct 拓扑 {
    /// 拓扑段列表（按 ts 升序）。
    pub 段: Vec<拓扑段>,
}

/// 步骤组件——步骤内的一条事件（LLM 思考 / 工具调用 / 其他）。
///
/// 依据：融合蓝图-设计稿.md §13.c 步骤流。
#[derive(Debug, Clone, Serialize)]
pub struct 步骤组件 {
    /// 组件类型：llm / tool / other。
    pub 类型: String,
    /// 动作名。
    pub 动作: String,
    /// ts（毫秒）。
    pub ts: u64,
    /// 耗时（毫秒）。
    pub 耗时ms: u64,
    /// token 总计。
    pub token: u64,
}

/// 步骤——§13.c 步骤流的一步。
#[derive(Debug, Clone, Serialize)]
pub struct 步骤 {
    /// 步骤号（从 1 起）。
    pub 步骤号: usize,
    /// 标题（动作前 60 字）。
    pub 标题: String,
    /// 开始 ts（毫秒）。
    pub 开始ts: u64,
    /// 耗时（毫秒，组件累加）。
    pub 耗时ms: u64,
    /// token 累加。
    pub token累加: u64,
    /// 组件列表。
    pub 组件: Vec<步骤组件>,
}

impl 白箱事件 {
    /// 构造一条最小白箱事件——缺字段填默认值（0 / 空）。
    pub fn 新(ts: u64, 源: impl Into<String>, 动作: impl Into<String>) -> Self {
        白箱事件 {
            ts,
            源: 源.into(),
            动作: 动作.into(),
            影响: Vec::new(),
            token: token用量::default(),
            耗时ms: 0,
            证据: String::new(),
            任务线id: String::new(),
        }
    }

    /// 链式追加影响项。
    pub fn 追加影响(mut self, 项: 影响项) -> Self {
        self.影响.push(项);
        self
    }

    /// 链式设置 token 用量。
    pub fn 设token(mut self, 用量: token用量) -> Self {
        self.token = 用量;
        self
    }

    /// 链式设置耗时。
    pub fn 设耗时(mut self, 毫秒: u64) -> Self {
        self.耗时ms = 毫秒;
        self
    }

    /// 链式设置证据。
    pub fn 设证据(mut self, 证据: impl Into<String>) -> Self {
        self.证据 = 证据.into();
        self
    }

    /// 链式设置任务线id——分裂流的分流键。空串表示主线/未分组。
    pub fn 设任务线id(mut self, id: impl Into<String>) -> Self {
        self.任务线id = id.into();
        self
    }
}

impl 影响项 {
    /// 构造一条影响项。
    pub fn 新(类型: impl Into<String>, 名: impl Into<String>) -> Self {
        影响项 {
            类型: 类型.into(),
            名: 名.into(),
            变化: String::new(),
            字节: None,
        }
    }

    /// 链式设置变化描述。
    pub fn 设变化(mut self, 变化: impl Into<String>) -> Self {
        self.变化 = 变化.into();
        self
    }

    /// 链式设置字节数。
    pub fn 设字节(mut self, 字节: u64) -> Self {
        self.字节 = Some(字节);
        self
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 白箱事件序列化字段名契约() {
        let 事件 = 白箱事件::新(1787038400123, "鸿蒙/道术施展-府", "工具循环-7")
            .追加影响(影响项::新("格位", "调用").设变化("+1 条").设字节(189000))
            .设token(token用量 {
                提示词: 1234,
                输出: 567,
                缓存: 89,
                总计: 1890,
            })
            .设耗时(2103)
            .设证据("原始命令行");

        let 文本 = serde_json::to_string(&事件).unwrap();
        let 值: serde_json::Value = serde_json::from_str(&文本).unwrap();
        assert_eq!(值["ts"], 1787038400123_u64);
        assert_eq!(值["源"], "鸿蒙/道术施展-府");
        assert_eq!(值["动作"], "工具循环-7");
        assert_eq!(值["影响"][0]["类型"], "格位");
        assert_eq!(值["影响"][0]["名"], "调用");
        assert_eq!(值["影响"][0]["变化"], "+1 条");
        assert_eq!(值["影响"][0]["字节"], 189000);
        assert_eq!(值["token"]["提示词"], 1234);
        assert_eq!(值["token"]["输出"], 567);
        assert_eq!(值["token"]["缓存"], 89);
        assert_eq!(值["token"]["总计"], 1890);
        assert_eq!(值["耗时ms"], 2103);
        assert_eq!(值["证据"], "原始命令行");
    }

    #[test]
    fn 缺字段填默认值() {
        let 事件 = 白箱事件::新(0, "源", "动作");
        let 文本 = serde_json::to_string(&事件).unwrap();
        let 值: serde_json::Value = serde_json::from_str(&文本).unwrap();
        assert_eq!(值["影响"].as_array().unwrap().len(), 0);
        assert_eq!(值["token"]["提示词"], 0);
        assert_eq!(值["耗时ms"], 0);
        // 证据为空时 skip_serializing_if 不输出
        assert!(值.get("证据").is_none() || 值["证据"].is_null());
    }

    #[test]
    fn 事件源字面对应() {
        assert_eq!(事件源::事件流.字面(), "event");
        assert_eq!(事件源::观测记录.字面(), "model");
        assert_eq!(事件源::识海记录.字面(), "shihai");
    }

    #[test]
    fn 任务线id序列化与默认() {
        // 设了任务线id 序列化带该字段
        let 事件 = 白箱事件::新(100, "源", "动作").设任务线id("线A");
        let 文本 = serde_json::to_string(&事件).unwrap();
        let 值: serde_json::Value = serde_json::from_str(&文本).unwrap();
        assert_eq!(值["任务线id"], "线A");

        // 未设任务线id skip_serializing_if 不输出
        let 事件2 = 白箱事件::新(100, "源", "动作");
        let 文本2 = serde_json::to_string(&事件2).unwrap();
        let 值2: serde_json::Value = serde_json::from_str(&文本2).unwrap();
        assert!(值2.get("任务线id").is_none() || 值2["任务线id"].is_null());

        // 旧数据缺任务线id 反序列化默认空串
        let 旧 = r#"{"ts":1,"源":"s","动作":"a","影响":[],"token":{"提示词":0,"输出":0,"缓存":0,"总计":0},"耗时ms":0}"#;
        let 反: 白箱事件 = serde_json::from_str(旧).unwrap();
        assert_eq!(反.任务线id, "");
    }
}
