//! 追加 - 落流 - 园：append-only 事件流，世界的一切所见所为皆为事件。
//!
//! 事件流是「经历记忆」的事实源：只追加、不改写、不删除，与「事件」格位（语义归纳）分工——
//! 事件流记细粒度事实，事件格位记粗粒度语义。对齐 DeepSeek「Every run is traceable」。

use serde::{Deserialize, Serialize};

/// 事件类型（本质：任何项目的通用状态变更类别）。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum 事件类型 {
    想法投递,
    要求入池,
    要求状态推进,
    设计上呈,
    工具调用,
    验收结论,
    版本存档,
    失败沉淀,
    进化留痕,
}

/// 事件：append-only 事实源的一条记录。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 事件 {
    pub 时间戳: u64,
    pub 类型: 事件类型,
    pub 载荷: serde_json::Value,
}

impl 事件 {
    /// 构造一条事件（时间戳 = 当前毫秒）。
    pub fn 新(类型: 事件类型, 载荷: serde_json::Value) -> 事件 {
        事件 { 时间戳: crate::当前毫秒(), 类型, 载荷 }
    }
}

/// 事件流：append-only 落盘读写（.上下文/事件流.jsonl）。
pub struct 事件流 {
    路径: std::path::PathBuf,
}

impl 事件流 {
    /// 在工作区根下打开（.上下文/事件流.jsonl）。
    pub fn 在工作区(工作区: &crate::工作区) -> 事件流 {
        事件流 { 路径: 工作区.上下文目录().join("事件流.jsonl") }
    }

    /// 追加一条事件（jsonl 一行，只追加不改写）。
    pub fn 追加事件(&self, 类型: 事件类型, 载荷: serde_json::Value) -> Result<事件, String> {
        let 事件 = 事件::新(类型, 载荷);
        let 行 = serde_json::to_string(&事件).map_err(|错误| format!("序列化事件失败: {错误}"))?;
        use std::io::Write;
        let mut 文件 = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.路径)
            .map_err(|错误| format!("打开事件流失败: {错误}"))?;
        writeln!(文件, "{行}").map_err(|错误| format!("写事件流失败: {错误}"))?;
        Ok(事件)
    }

    /// 读事件流（从起点下标起，返回后续全部事件）。
    pub fn 读事件流(&self, 起点: usize) -> Result<Vec<事件>, String> {
        if !self.路径.exists() {
            return Ok(Vec::new());
        }
        let 内容 = std::fs::read_to_string(&self.路径).map_err(|错误| format!("读事件流失败: {错误}"))?;
        内容
            .lines()
            .filter(|行| !行.trim().is_empty())
            .skip(起点)
            .map(|行| serde_json::from_str::<事件>(行).map_err(|错误| format!("解析事件失败: {错误}")))
            .collect()
    }
}

/// 从事件流重放要求状态：按「要求入池」「要求状态推进」事件重建每个要求的最终状态。
/// 本质：事件流是 append-only 事实源，可从中重建现状（回放一致性），对齐 DeepSeek「可追溯」。
/// 返回 要求id → 最终状态（Debug 文本）的映射；同一要求取最后一条事件的状态。
pub fn 重放要求状态(事件们: &[事件]) -> std::collections::HashMap<String, String> {
    let mut 状态表 = std::collections::HashMap::new();
    for 事件 in 事件们 {
        match 事件.类型 {
            事件类型::要求入池 | 事件类型::要求状态推进 => {
                if let (Some(id), Some(状态)) = (事件.载荷["要求id"].as_str(), 事件.载荷["状态"].as_str()) {
                    状态表.insert(id.to_string(), 状态.to_string());
                }
            }
            _ => {}
        }
    }
    状态表
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 临时工作区(名: &str) -> crate::工作区 {
        let 根 = std::env::temp_dir().join(format!("事件流-{名}-{}", crate::当前毫秒()));
        let 工作区 = crate::工作区::新(&根);
        工作区.初始化().unwrap();
        工作区
    }

    #[test]
    fn 事件流_追加与读取() {
        let 工作区 = 临时工作区("追加读取");
        let 流 = 事件流::在工作区(&工作区);
        流.追加事件(事件类型::想法投递, serde_json::json!({"想法": "测试"})).unwrap();
        流.追加事件(事件类型::工具调用, serde_json::json!({"工具": "写文件"})).unwrap();

        let 事件们 = 流.读事件流(0).unwrap();
        assert_eq!(事件们.len(), 2, "应读回两条事件：{事件们:?}");
        assert_eq!(事件们[0].类型, 事件类型::想法投递);
        assert_eq!(事件们[1].类型, 事件类型::工具调用);
        assert_eq!(事件们[1].载荷["工具"], "写文件");

        // 起点偏移：从 1 起只读第二条。
        let 后续 = 流.读事件流(1).unwrap();
        assert_eq!(后续.len(), 1);
        assert_eq!(后续[0].类型, 事件类型::工具调用);
        let _ = std::fs::remove_dir_all(工作区.根路径());
    }

    #[test]
    fn 事件流_追加不改写() {
        let 工作区 = 临时工作区("追加不改写");
        let 流 = 事件流::在工作区(&工作区);
        流.追加事件(事件类型::要求入池, serde_json::json!({"id": "要求-1"})).unwrap();
        // 再追加一条，旧事件保持不变（append-only）。
        流.追加事件(事件类型::验收结论, serde_json::json!({"结论": "通过"})).unwrap();
        let 全部 = 流.读事件流(0).unwrap();
        assert_eq!(全部.len(), 2, "append-only 应累计两条：{全部:?}");
        assert_eq!(全部[0].载荷["id"], "要求-1", "旧事件不得被改写");
        assert_eq!(全部[1].载荷["结论"], "通过");
        let _ = std::fs::remove_dir_all(工作区.根路径());
    }

    #[test]
    fn 重放要求状态_取最后状态() {
        let 事件们 = vec![
            事件::新(事件类型::要求入池, serde_json::json!({"要求id": "要求-1", "状态": "待领"})),
            事件::新(事件类型::要求状态推进, serde_json::json!({"要求id": "要求-1", "状态": "设计中"})),
            事件::新(事件类型::要求状态推进, serde_json::json!({"要求id": "要求-1", "状态": "已存档"})),
            事件::新(事件类型::要求入池, serde_json::json!({"要求id": "要求-2", "状态": "待领"})),
        ];
        let 状态表 = 重放要求状态(&事件们);
        assert_eq!(状态表.get("要求-1").map(|状态| 状态.as_str()), Some("已存档"), "取最后一条推进状态");
        assert_eq!(状态表.get("要求-2").map(|状态| 状态.as_str()), Some("待领"), "只入池未推进取入池状态");
    }

    #[test]
    fn 重放要求状态_忽略非状态事件() {
        let 事件们 = vec![
            事件::新(事件类型::工具调用, serde_json::json!({"工具": "写文件"})),
            事件::新(事件类型::验收结论, serde_json::json!({"要求id": "要求-1", "结论": "通过"})),
        ];
        let 状态表 = 重放要求状态(&事件们);
        assert!(状态表.is_empty(), "工具调用与验收结论不产生要求状态");
    }
}
