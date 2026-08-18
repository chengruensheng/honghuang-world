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
        事件 {
            时间戳: crate::当前毫秒(),
            类型,
            载荷,
        }
    }
}

/// 事件流：append-only 落盘读写（.上下文/事件流.jsonl）。
#[derive(Clone)]
pub struct 事件流 {
    路径: std::path::PathBuf,
}

impl 事件流 {
    /// 在工作区根下打开（.上下文/事件流.jsonl）。
    pub fn 在工作区(工作区: &crate::工作区) -> 事件流 {
        事件流 {
            路径: 工作区.上下文目录().join("事件流.jsonl"),
        }
    }

    /// 追加一条事件（jsonl 一行，只追加不改写）。
    /// 进程级互斥锁：防并发写者行交错（2026-08-17 体检实锤：两进程 append 交错，
    /// 一条物理行拼入两个事件 JSON 致事件流损坏）。锁文件 create_new 原子抢锁，
    /// 陈旧锁（>30 秒）视为崩溃残留自动清理；最长等待 5 秒，仍失败则放弃本事件（可接受，不阻塞主流程）。
    pub fn 追加事件(
        &self, 类型: 事件类型, 载荷: serde_json::Value
    ) -> Result<事件, String> {
        let 事件 = 事件::新(类型, 载荷);
        let 行 = serde_json::to_string(&事件).map_err(|错误| format!("序列化事件失败: {错误}"))?;
        use std::io::Write;
        let 锁路径 = self.路径.with_extension("jsonl.lock");
        let _锁 = 抢事件流锁(&锁路径);
        let Some(锁) = _锁 else {
            rizhi_fu::warn!(路径 = %self.路径.display(), "事件流锁等待超时，放弃本事件（并发写者持续占用）");
            return Ok(事件);
        };
        let mut 文件 = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.路径)
            .map_err(|错误| format!("打开事件流失败: {错误}"))?;
        writeln!(文件, "{行}").map_err(|错误| format!("写事件流失败: {错误}"))?;
        drop(文件);
        drop(锁);
        let _ = std::fs::remove_file(&锁路径);
        Ok(事件)
    }

    /// 读事件流（从起点下标起，返回后续全部事件）。
    pub fn 读事件流(&self, 起点: usize) -> Result<Vec<事件>, String> {
        if !self.路径.exists() {
            return Ok(Vec::new());
        }
        let 内容 =
            std::fs::read_to_string(&self.路径).map_err(|错误| format!("读事件流失败: {错误}"))?;
        内容
            .lines()
            .filter(|行| !行.trim().is_empty())
            .skip(起点)
            .map(|行| {
                serde_json::from_str::<事件>(行).map_err(|错误| format!("解析事件失败: {错误}"))
            })
            .collect()
    }
}

/// 抢事件流写锁：create_new 原子抢锁；陈旧锁（>30 秒）视为崩溃残留清理重试；
/// 最长等待 5 秒，超时返回 None（调用方放弃本事件，不阻塞主流程）。
fn 抢事件流锁(锁路径: &std::path::Path) -> Option<std::fs::File> {
    let 开始 = std::time::Instant::now();
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(锁路径)
        {
            Ok(文件) => return Some(文件),
            Err(_) => {
                // 陈旧锁清理：持有者进程崩溃的残留。
                if let Ok(元) = std::fs::metadata(锁路径) {
                    if let Ok(修改) = 元.modified() {
                        if let Ok(龄) = 修改.elapsed() {
                            if 龄.as_secs() > 30 {
                                rizhi_fu::warn!(路径 = ?锁路径, "事件流锁已陈旧，清理重试");
                                let _ = std::fs::remove_file(锁路径);
                                continue;
                            }
                        }
                    }
                }
                if 开始.elapsed().as_secs() >= 5 {
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
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
                if let (Some(id), Some(状态)) =
                    (事件.载荷["要求id"].as_str(), 事件.载荷["状态"].as_str())
                {
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
        流.追加事件(事件类型::想法投递, serde_json::json!({"想法": "测试"}))
            .unwrap();
        流.追加事件(事件类型::工具调用, serde_json::json!({"工具": "写文件"}))
            .unwrap();

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
        流.追加事件(事件类型::要求入池, serde_json::json!({"id": "要求-1"}))
            .unwrap();
        // 再追加一条，旧事件保持不变（append-only）。
        流.追加事件(事件类型::验收结论, serde_json::json!({"结论": "通过"}))
            .unwrap();
        let 全部 = 流.读事件流(0).unwrap();
        assert_eq!(全部.len(), 2, "append-only 应累计两条：{全部:?}");
        assert_eq!(全部[0].载荷["id"], "要求-1", "旧事件不得被改写");
        assert_eq!(全部[1].载荷["结论"], "通过");
        let _ = std::fs::remove_dir_all(工作区.根路径());
    }

    #[test]
    fn 重放要求状态_取最后状态() {
        let 事件们 = vec![
            事件::新(
                事件类型::要求入池,
                serde_json::json!({"要求id": "要求-1", "状态": "待领"}),
            ),
            事件::新(
                事件类型::要求状态推进,
                serde_json::json!({"要求id": "要求-1", "状态": "设计中"}),
            ),
            事件::新(
                事件类型::要求状态推进,
                serde_json::json!({"要求id": "要求-1", "状态": "已存档"}),
            ),
            事件::新(
                事件类型::要求入池,
                serde_json::json!({"要求id": "要求-2", "状态": "待领"}),
            ),
        ];
        let 状态表 = 重放要求状态(&事件们);
        assert_eq!(
            状态表.get("要求-1").map(|状态| 状态.as_str()),
            Some("已存档"),
            "取最后一条推进状态"
        );
        assert_eq!(
            状态表.get("要求-2").map(|状态| 状态.as_str()),
            Some("待领"),
            "只入池未推进取入池状态"
        );
    }

    #[test]
    fn 重放要求状态_忽略非状态事件() {
        let 事件们 = vec![
            事件::新(事件类型::工具调用, serde_json::json!({"工具": "写文件"})),
            事件::新(
                事件类型::验收结论,
                serde_json::json!({"要求id": "要求-1", "结论": "通过"}),
            ),
        ];
        let 状态表 = 重放要求状态(&事件们);
        assert!(状态表.is_empty(), "工具调用与验收结论不产生要求状态");
    }

    /// 并发追加不损坏（2026-08-17 实锤修复：两进程 append 交错致一条物理行拼入两个事件）。
    /// 多线程并发追加 100 条，读回 100 条全部可解析且无拼接行。
    #[test]
    fn 事件流_并发追加不损坏() {
        let 工作区 = 临时工作区("并发追加");
        let 流 = 事件流::在工作区(&工作区);
        let 线程们: Vec<_> = (0..4)
            .map(|序号| {
                let 流 = 流.clone();
                std::thread::spawn(move || {
                    for 次 in 0..25 {
                        流.追加事件(
                            事件类型::工具调用,
                            serde_json::json!({"线程": 序号, "次": 次}),
                        )
                        .unwrap();
                    }
                })
            })
            .collect();
        for 线程 in 线程们 {
            线程.join().unwrap();
        }
        // 逐行校验：全部 100 行可独立解析（无拼接行）。
        let 内容 = std::fs::read_to_string(工作区.上下文目录().join("事件流.jsonl")).unwrap();
        let 行们: Vec<&str> = 内容.lines().filter(|行| !行.trim().is_empty()).collect();
        assert_eq!(行们.len(), 100, "并发 4×25 应恰好 100 行（无拼接/无丢失）");
        for 行 in 行们 {
            let 事件: 事件 =
                serde_json::from_str(行).unwrap_or_else(|错误| panic!("存在损坏行：{错误}：{行}"));
            assert_eq!(事件.类型, 事件类型::工具调用);
        }
        let _ = std::fs::remove_dir_all(工作区.根路径());
    }
}
