//! 终裁：六准圣分维独立审验 → 鸿钧终裁。
//!
//! 三段流程：① 机械前置门槛（编译失败/无产物/空洞产物 → 直接打回）
//!           ② 六准圣 LLM 分维独立审验（业务六维：业务正确性/数据完整性/性能并发/安全副作用/异常兼容/用户体验）
//!           ③ 鸿钧终裁（综合六准圣意见，仅在有争议时调用 LLM）
//!
//! 消除三类真实缺口：
//! 1. 非 RS 必需文件误判孤儿——准圣「模块接入」维可豁免测试/文档/凭证类非核心产物；
//! 2. 核查型已达标任务误打回——准圣「完整性」维可识别纯审计/核查/盘点型产物类别；
//! 3. 路径偏离误判——准圣「依赖合理」维可识别正反斜杠/大小写/相对路径等价写法。
//!
//! 机械事实优先（2026-08-17 改造）：准圣提示词注入【产物内容摘要】——对 .rs 产物机械提取
//! pub 符号签名与测试函数名（真实文件事实），治「无法直接读取文件、凭字节增量推断」的猜谜式审验；
//! 准圣只审语义（是否达成验收标准），事实层由机械摘要兜底。
//!
//! 降级：无 LLM 配置或无要求书时跳过 ②③，走规则兜底（路径相符 + 模块接入），
//! 保证现有 `验收裁决(要求id, 产物们, 耗时秒, 涉及文件, 失败说明)` 签名测试继续通过。

use crate::类型_定义_殿::*;
use jiance_fu::{观测角色, 进入观测};
use moxing_fu::{对话消息, 常规上限, 提取对象, 模型配置, 用量};
use rizhi_fu::{error, info, warn};
use serde::{Deserialize, Serialize};

use super::模块树::接入模块树;

// ── 准圣维度 ──

/// 准圣审验维度（六维独立，各管一摊；业务语义六维，对应设计稿 §1.5.1 六准圣）。
/// 本质：审验一个软件交付物的六个通用业务方面。命名/接入/依赖等工程规范走机械门槛，规则从格位读。
/// 兼容遗留：历史旧记录曾用工程六维（完整性/健壮性/命名规范/模块接入/依赖合理/可测试性），
/// 反序列化旧记录时映射为 兼容遗留(原名)，保证验收.jsonl 全量可读（追问进度/流水观览不因旧档失败）。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum 准圣维度 {
    /// 红云·业务正确性：产物是否达成要求目标与验收标准。
    业务正确性,
    /// 镇元子·数据完整性：落盘/状态/队列数据是否完整一致，无半写/丢失。
    数据完整性,
    /// 鲲鹏·性能并发：是否引入阻塞、死锁、并发污染。
    性能并发,
    /// 神农·安全副作用：是否越权改动涉及范围外文件、是否破坏现有功能。
    安全副作用,
    /// 冥河·异常兼容：失败分支/错误处理/边界输入是否兜住。
    异常兼容,
    /// 轩辕·用户体验：界主可见的命令输出/状态呈现是否真实清晰。
    用户体验,
    /// 历史遗留维度（旧六维兼容，仅读不审）。
    兼容遗留(String),
}

impl<'de> Deserialize<'de> for 准圣维度 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let 名 = String::deserialize(deserializer)?;
        Ok(match 名.as_str() {
            "业务正确性" => 准圣维度::业务正确性,
            "数据完整性" => 准圣维度::数据完整性,
            "性能并发" => 准圣维度::性能并发,
            "安全副作用" => 准圣维度::安全副作用,
            "异常兼容" => 准圣维度::异常兼容,
            "用户体验" => 准圣维度::用户体验,
            _ => 准圣维度::兼容遗留(名),
        })
    }
}

impl 准圣维度 {
    pub fn 名称(&self) -> &'static str {
        match self {
            准圣维度::业务正确性 => "红云·业务正确性准圣",
            准圣维度::数据完整性 => "镇元子·数据完整性准圣",
            准圣维度::性能并发 => "鲲鹏·性能并发准圣",
            准圣维度::安全副作用 => "神农·安全副作用准圣",
            准圣维度::异常兼容 => "冥河·异常兼容准圣",
            准圣维度::用户体验 => "轩辕·用户体验准圣",
            准圣维度::兼容遗留(_) => "历史遗留维度准圣",
        }
    }

    pub fn 角色提示(&self) -> &'static str {
        match self {
            准圣维度::业务正确性 => concat!(
                "你的职司：审验产物是否达成要求目标与验收标准。\n",
                "重点：逐条核对验收标准的每一条是否被实现；产物逻辑是否与要求方向一致；",
                "是否存在张冠李戴、答非所问。"
            ),
            准圣维度::数据完整性 => concat!(
                "你的职司：审验落盘/状态/队列数据是否完整一致。\n",
                "重点：是否有半写/丢失；序列化与反序列化是否一致；状态迁移是否原子；",
                "数据文件是否损坏；同实体是否产生不一致的重复记录。"
            ),
            准圣维度::性能并发 => concat!(
                "你的职司：审验是否引入阻塞、死锁、并发污染。\n",
                "重点：是否阻塞主循环；锁争用/target-dir/状态文件争用；无限循环；",
                "不必要的 O(n²) 或全量重算。"
            ),
            准圣维度::安全副作用 => concat!(
                "你的职司：审验是否越权改动、是否破坏现有功能。\n",
                "重点：是否改动涉及范围外文件；是否删除/覆盖他人产物；是否引入破坏性副作用；",
                "命令是否越权。"
            ),
            准圣维度::异常兼容 => concat!(
                "你的职司：审验失败分支/错误处理/边界输入是否兜住。\n",
                "重点：空输入/空集合/空文件是否兜底；路径不存在/权限拒绝是否降级；",
                "错误是否向上传播；边界是否溢出。"
            ),
            准圣维度::用户体验 => concat!(
                "你的职司：审验界主可见的输出/呈现是否真实清晰。\n",
                "重点：命令输出是否真实反映状态；提示是否清晰无歧义；是否静默失败无反馈。"
            ),
            准圣维度::兼容遗留(_) => "历史遗留维度（仅兼容旧记录读取，不参与审验）。",
        }
    }

    pub fn 所有() -> [准圣维度; 6] {
        [
            准圣维度::业务正确性,
            准圣维度::数据完整性,
            准圣维度::性能并发,
            准圣维度::安全副作用,
            准圣维度::异常兼容,
            准圣维度::用户体验,
        ]
    }
}

// ── 数据结构 ──

/// 准圣审验意见：一次审验的产出。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 准圣意见 {
    pub 维度: 准圣维度,
    pub 结论: 验收结论,
    /// 0-100 分。
    pub 评分: u8,
    pub 关键问题: String,
    pub 改进建议: Vec<String>,
}

/// 终裁回执：三段流程的完整产物（供定档/历史/可观测）。
/// 验收回执字段 flatten 平铺到顶层：旧读取方按 `验收回执` 解析自动兼容（忽略多出字段）。
/// 新增字段全部 #[serde(default)]：历史旧回执（仅验收回执字段）反序列化不失败，老记录可读。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 终裁回执 {
    /// 验收回执（flatten 平铺：结论/产物/耗时秒 等字段在顶层，供下游定档/历史兼容）。
    #[serde(flatten)]
    pub 验收: 验收回执,
    /// 六准圣独立意见。
    #[serde(default)]
    pub 准圣意见们: Vec<准圣意见>,
    /// 鸿钧终裁依据（一句话）。
    #[serde(default)]
    pub 终裁依据: String,
    /// 全流程累计 token 用量。
    #[serde(default)]
    pub 用量: 用量,
}

// ── 机械前置门槛 ──

/// 机械前置门槛：仅做绝对硬性检查。
/// 通过 → 返回 None 进入准圣审验；不通过 → 返回 Some(打回终裁) 立即终结。
fn 机械前置门槛(
    要求id: &str,
    产物们: &[产物条目],
    耗时秒: f64,
    失败说明: Option<&str>,
) -> Option<终裁回执> {
    // 1. 编译失败：产物不可信，直接打回。
    if let Some(说明) = 失败说明 {
        warn!(要求 = 要求id, "终裁打回：编译失败");
        return Some(终裁回执 {
            验收: 验收回执 {
                要求id: 要求id.to_string(),
                结论: 验收结论::打回,
                验收意见: Some(说明.to_string()),
                产物: vec![],
                耗时秒,
            },
            准圣意见们: vec![],
            终裁依据: "编译失败，产物不可信".to_string(),
            用量: 用量::default(),
        });
    }

    // 2. 无产物：不在此拦截——执行成功但无产物属于「现状已达标无需改动」的语义裁决，
    //    按设计稿 §11.2 规则 5 交六准圣 LLM 判断（机械门槛只防三硬伤）。
    //    执行失败且无产物已由上一分支（编译失败/失败说明）拦截，此处不会误放行空转。
    debug_assert!(失败说明.is_none(), "失败说明应已在上方分支处理");

    // 3. 路径越界：产物路径必须是工作区内的相对路径，不得含 .. 或为绝对路径。
    //    本质：任何项目的通用安全护栏，防产物逃逸涉及范围、写到工作区之外。
    let 根 = shihai_fu::工作区::定位();
    let 有越界 = 产物们.iter().any(|产物| {
        let 路径 = 产物.路径.replace('\\', "/");
        // 绝对路径：以 / 开头，或首段为盘符（如 C:）。
        if 路径.starts_with('/') {
            return true;
        }
        let 段们: Vec<&str> = 路径.split('/').collect();
        if 段们.first().is_some_and(|段| 段.ends_with(':')) {
            return true;
        }
        // 上跳段：.. 逃逸工作区。
        段们.contains(&"..")
    });
    if 有越界 {
        warn!(要求 = 要求id, "终裁打回：产物路径越界");
        return Some(终裁回执 {
            验收: 验收回执 {
                要求id: 要求id.to_string(),
                结论: 验收结论::打回,
                验收意见: Some(
                    "实现层：产物路径越界（含 .. 或绝对路径，逃逸工作区）".to_string(),
                ),
                产物: vec![],
                耗时秒,
            },
            准圣意见们: vec![],
            终裁依据: "产物路径越界".to_string(),
            用量: 用量::default(),
        });
    }

    // 4. 空洞产物：产物必须真实存在且非空（防空文件静默破坏）。
    let 有空洞 = 产物们.iter().any(|产物| {
        let 绝对 = 根.根路径().join(&产物.路径);
        std::fs::metadata(&绝对)
            .map(|元| 元.len() == 0)
            .unwrap_or(true)
    });
    if 有空洞 {
        warn!(要求 = 要求id, "终裁打回：空洞产物");
        return Some(终裁回执 {
            验收: 验收回执 {
                要求id: 要求id.to_string(),
                结论: 验收结论::打回,
                验收意见: Some("实现层：产物为空文件或不存在".to_string()),
                产物: vec![],
                耗时秒,
            },
            准圣意见们: vec![],
            终裁依据: "空洞产物".to_string(),
            用量: 用量::default(),
        });
    }

    None
}

// ── 规则兜底（无 LLM 时） ──

/// 路径相符检查（统一分隔符、不区分大小写）。
/// 涉及文件为空时一律通过（兜底产物清单）。
fn 路径相符(涉及文件: &[String], 产物们: &[产物条目]) -> bool {
    if 涉及文件.is_empty() {
        return true;
    }
    let 规整涉及: Vec<String> = 涉及文件
        .iter()
        .map(|f| f.replace('\\', "/").to_lowercase())
        .collect();
    产物们.iter().any(|产物| {
        let 产物路径 = 产物.路径.replace('\\', "/").to_lowercase();
        规整涉及
            .iter()
            .any(|项| 产物路径 == *项 || 产物路径.contains(项.as_str()))
    })
}

/// 规则兜底（无 LLM 配置时）：路径相符 + 模块接入。
/// 保留旧机械裁决语义，使现有 验收裁决(无要求书) 测试继续通过。
fn 规则兜底(
    要求id: &str,
    产物们: &[产物条目],
    耗时秒: f64,
    涉及文件: &[String],
) -> 终裁回执 {
    let 路径_ok = 路径相符(涉及文件, 产物们);
    // 无产物保守打回：降级兜底无 LLM 语义能力，无法判断「现状已达标无需产物」——
    // 该语义裁决（设计稿 §11.2 规则 5）只属于六准圣 LLM，降级时不确定即打回。
    if 产物们.is_empty() {
        warn!(要求 = 要求id, "终裁打回：无产物（降级兜底保守）");
        return 终裁回执 {
            验收: 验收回执 {
                要求id: 要求id.to_string(),
                结论: 验收结论::打回,
                验收意见: Some("实现层：无产物（降级兜底无 LLM 语义判断）".to_string()),
                产物: vec![],
                耗时秒,
            },
            准圣意见们: vec![],
            终裁依据: "无产物，降级兜底保守打回".to_string(),
            用量: 用量::default(),
        };
    }
    let 根 = shihai_fu::工作区::定位();
    let 有孤儿 = 产物们
        .iter()
        .any(|产物| !接入模块树(根.根路径(), &产物.路径));
    let 结论 = if 路径_ok && !有孤儿 {
        验收结论::通过
    } else {
        验收结论::打回
    };
    let 意见 = if 结论 == 验收结论::打回 {
        Some(if 有孤儿 {
            "实现层：产物未接入模块树".to_string()
        } else {
            "实现层：产物路径偏离涉及范围".to_string()
        })
    } else {
        None
    };
    info!(要求 = 要求id, 结论 = ?结论, "规则兜底裁决");
    终裁回执 {
        验收: 验收回执 {
            要求id: 要求id.to_string(),
            结论,
            验收意见: 意见,
            产物: 产物们.to_vec(),
            耗时秒,
        },
        准圣意见们: vec![],
        终裁依据: "无LLM配置，走规则兜底".to_string(),
        用量: 用量::default(),
    }
}

// ── 单准圣审验 ──

/// 涉及路径现状：涉及路径文件是否存在、字节数，以及与「执行前基线」的对比——
/// 供审验型要求「现状已达标无需产物」核验真伪，并给准圣「改前 → 改后」增量证据，
/// 防「产物=当前盘面=涉及现状」的大小相等被误判为未变（2026-08-16 修复）。
fn 涉及路径现状(涉及路径: &[String]) -> String {
    if 涉及路径.is_empty() {
        return "（无涉及路径）".to_string();
    }
    let 根 = shihai_fu::工作区::定位();
    let 基线 = shihai_fu::读执行基线(&根);
    let 基线空 = 基线.指纹们.is_empty();
    涉及路径
        .iter()
        .map(|p| {
            let 相对 = p.replace('\\', "/");
            let 当前 = match std::fs::metadata(根.根路径().join(&相对)) {
                Ok(元) if 元.len() > 0 => format!("{}字节", 元.len()),
                Ok(_) => "空文件".to_string(),
                Err(_) => "不存在".to_string(),
            };
            let 基线侧 = if 基线空 {
                "（无执行前基线记录）".to_string()
            } else {
                match 基线.指纹们.get(&相对) {
                    Some(指纹) => format!("执行前基线 {}字节", 指纹.大小),
                    None => "执行前基线无此文件（本轮新增）".to_string(),
                }
            };
            format!("- {相对}（当前{当前}；{基线侧}）")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 现实事实：审验前执行层已机械验证的硬事实，防准圣以 Rust 默认假设脑补推翻（设计稿 §11.2 规则 8）。
const 现实事实: &str = concat!(
    "- 构建与测试均已通过：产物已通过 cargo build --workspace --lib 编译与 cargo test --workspace --lib 全量测试（含测试编译）；任一失败会直接打回，不会进入本审验。\n",
    "- 库根事实：各 crate 库根由自身 Cargo.toml [lib] path 决定（如 乾坤/呈现-域/命令操作-府 的根是 入口.rs，不是 lib.rs；不存在 lib.rs 属正常设计）。\n",
    "- 模块树：按 #[path = \"子目录/模块.rs\"] pub mod 逐级声明接入；「未见 lib.rs」不等于「未接入模块树」。"
);

/// 产物内容摘要：对 .rs 产物机械提取 pub 符号签名与测试函数名（真实文件事实）。
/// 治「无法直接读取文件、凭字节增量推断」的猜谜式审验——准圣只审语义，事实层由机械摘要兜底。
/// 单文件符号上限 20 条、总摘要上限 2000 字符，防提示词膨胀。
fn 产物内容摘要(产物们: &[产物条目]) -> String {
    const 单文件符号上限: usize = 20;
    const 总摘要上限: usize = 2_000;
    let 根 = shihai_fu::工作区::定位();
    let mut 摘要们 = Vec::new();
    for 产物 in 产物们 {
        if !产物.路径.ends_with(".rs") {
            continue;
        }
        let Ok(内容) = std::fs::read_to_string(根.根路径().join(&产物.路径)) else {
            continue;
        };
        let 总行数 = 内容.lines().count();
        let 行们: Vec<&str> = 内容.lines().collect();
        let mut 符号们 = Vec::new();
        let mut 测试们 = Vec::new();
        let mut 索引 = 0usize;
        while 索引 < 行们.len() {
            let 行 = 行们[索引].trim();
            if 行.starts_with("#[test]") {
                // 下一行取 fn 名
                if let Some(下一行) = 行们.get(索引 + 1) {
                    let 名 = 下一行
                        .trim()
                        .trim_start_matches("fn ")
                        .split('(')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !名.is_empty() {
                        测试们.push(名);
                    }
                }
                索引 += 1;
                continue;
            }
            if 行.starts_with("pub fn")
                || 行.starts_with("pub struct")
                || 行.starts_with("pub enum")
                || 行.starts_with("pub trait")
                || 行.starts_with("pub const")
                || 行.starts_with("pub type")
                || 行.starts_with("pub use")
            {
                let 签名 = 行.split(" {").next().unwrap_or(行).to_string();
                符号们.push(签名);
            }
            索引 += 1;
        }
        if 符号们.is_empty() && 测试们.is_empty() {
            摘要们.push(format!(
                "- {}（{}行，无 pub 符号与测试）",
                产物.路径, 总行数
            ));
            continue;
        }
        let 符号段 = 符号们
            .iter()
            .take(单文件符号上限)
            .cloned()
            .collect::<Vec<_>>()
            .join("；");
        let 测试段 = 测试们
            .iter()
            .take(单文件符号上限)
            .cloned()
            .collect::<Vec<_>>()
            .join("、");
        let mut 条目 = format!("- {}（{}行）\n    符号：{}", 产物.路径, 总行数, 符号段);
        if !测试段.is_empty() {
            条目.push_str(&format!("\n    测试：{}", 测试段));
        }
        摘要们.push(条目);
    }
    if 摘要们.is_empty() {
        return "（无非 rs 产物，无内容摘要）".to_string();
    }
    let mut 合并 = 摘要们.join("\n");
    if 合并.chars().count() > 总摘要上限 {
        合并 = 合并.chars().take(总摘要上限).collect::<String>();
        合并.push_str("…（摘要截断，按需 读文件 工具回读）");
    }
    合并
}

/// 单准圣审验：模型一次调用 → 该维度意见。
fn 单准圣审验(
    要求书: &要求书,
    产物们: &[产物条目],
    维度: &准圣维度,
    配置: &模型配置,
) -> Result<(准圣意见, 用量), String> {
    let 产物清单 = 产物们
        .iter()
        .map(|p| {
            format!(
                "- {} ({}字节, {}, 变化类型={})",
                p.路径, p.字节数, p.类别, p.变化类型
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let 涉及 = if 要求书.约束.涉及路径.is_empty() {
        "（无）".to_string()
    } else {
        要求书.约束.涉及路径.join(", ")
    };
    let 涉及现状 = 涉及路径现状(&要求书.约束.涉及路径);
    let 内容摘要 = 产物内容摘要(产物们);

    let 提示 = format!(
        "你是{dim}\n\n{role}\n\n【要求方向】{dir}\n【验收标准】{std}\n【涉及路径】{path}\n【涉及现状】\n{cur}\n【产物清单】\n{prod}\n【产物内容摘要】（机械提取自产物真实文件，可据此核对实现细节，不必假设无法读取文件）\n{sum}\n\n【现实事实】（执行层已机械验证，勿以假设推翻）\n{fact}\n\n请基于以上事实独立审验，仅输出 JSON（无 Markdown、无解释、无 think）：\n{{\"结论\":\"通过|打回\",\"评分\":0-100,\"关键问题\":\"一句话\",\"改进建议\":[\"建议1\",\"建议2\"]}}\n\n判定原则：\n- 通过：你的维度内未发现阻断性问题，可改进但不影响本轮交付。\n- 打回：你的维度内存在必须修复的阻断性问题。\n- 产物清单为空时（设计稿 §11.2 规则 5/7：现状已达标无需产物的语义裁决）：\n  - 要求方向属审验/核查类（含 审验/核对/检查/验证/达标/核查 措辞）、要求书允许「已达标则不写文件」、且涉及路径文件均存在非空 → 判「通过」（评分 60-85，按实际核验程度给分）。\n  - 要求方向属实现/新增/开发类且无产物 → 判「打回」。\n  - 涉及路径文件缺失或为空 → 判「打回」。\n- 改进建议最多 3 条；无则输出空数组。",
        dim = 维度.名称(),
        role = 维度.角色提示(),
        dir = 要求书.方向,
        std = 要求书.验收标准,
        path = 涉及,
        cur = 涉及现状,
        prod = 产物清单,
        sum = 内容摘要,
        fact = 现实事实,
    );

    // 成本分级（生产化 3.5）：准圣意见 JSON（结论/评分/关键问题/建议 ≤3 条）体量小，
    // 输出上限从 常规上限 降至 精简上限——六准圣 6 次调用合计省 ~4 倍输出 token。
    let (回复, 用量) = 调用模型(配置, &[对话消息::用户(提示)], moxing_fu::精简上限)?;
    let 干净 = 提取对象(&回复).map_err(|错误| format!("准圣审验解析失败: {错误}"))?;
    let 解析: serde_json::Value =
        serde_json::from_str(&干净).map_err(|错误| format!("准圣审验解析失败: {错误}"))?;
    let 结论 = match 解析["结论"].as_str() {
        Some("通过") => 验收结论::通过,
        _ => 验收结论::打回,
    };
    let 评分 = 解析["评分"].as_u64().unwrap_or(50).min(100) as u8;
    let 关键问题 = 解析["关键问题"].as_str().unwrap_or("").to_string();
    let 改进建议 = 解析["改进建议"]
        .as_array()
        .map(|项们| {
            项们
                .iter()
                .filter_map(|项| 项.as_str().map(|s| s.to_string()))
                .take(3)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok((
        准圣意见 {
            维度: 维度.clone(),
            结论,
            评分,
            关键问题,
            改进建议,
        },
        用量,
    ))
}

/// 调用模型：薄包装，便于测试时 mock。
fn 调用模型(
    配置: &模型配置,
    消息们: &[对话消息],
    输出上限: u32,
) -> Result<(String, 用量), String> {
    moxing_fu::调用模型(配置, 消息们, 输出上限)
}

// ── 六准圣审验 ──

/// 六准圣分维独立审验：6 次独立模型调用，任意准圣失败仅记 warn 不影响其他准圣。
pub fn 六准圣审验(
    要求书: &要求书,
    产物们: &[产物条目],
    配置: &模型配置,
) -> (Vec<准圣意见>, 用量) {
    let mut 意见们 = Vec::with_capacity(6);
    let mut 累计用量 = 用量::default();
    for 维度 in 准圣维度::所有().iter() {
        match 单准圣审验(要求书, 产物们, 维度, 配置) {
            Ok((意见, 用量)) => {
                info!(
                    要求 = %要求书.id, 维度 = 维度.名称(),
                    结论 = ?意见.结论, 评分 = 意见.评分,
                    "准圣审验完成"
                );
                累计用量.加(&用量);
                意见们.push(意见);
            }
            Err(错误) => {
                warn!(
                    要求 = %要求书.id, 维度 = 维度.名称(),
                    错误 = %错误, "准圣审验失败，记为打回"
                );
                意见们.push(准圣意见 {
                    维度: 维度.clone(),
                    结论: 验收结论::打回,
                    评分: 0,
                    关键问题: format!("审验失败：{错误}"),
                    改进建议: vec!["重试本轮验收".to_string()],
                });
            }
        }
    }
    (意见们, 累计用量)
}

// ── 鸿钧终裁 ──

/// 鸿钧终裁：综合六准圣意见 → 最终结论。
/// - 6 准圣一致通过 → 通过
/// - 6 准圣一致打回 → 打回
/// - 争议 → 调用鸿钧 LLM 综合判断
fn 鸿钧终裁(
    要求书: &要求书,
    产物们: &[产物条目],
    意见们: &[准圣意见],
    配置: &模型配置,
) -> Result<(验收结论, String, 用量), String> {
    // 白箱观测：终裁是鸿钧动作（栈顶覆盖验收档；无争议时无 LLM 调用，仅机械结论）。
    let _观测守卫 = 进入观测(观测角色::鸿钧, None, Some(要求书.id.clone()), None);
    let 通过数 = 意见们.iter().filter(|o| o.结论 == 验收结论::通过).count();
    let 总数 = 意见们.len();
    if 通过数 == 总数 && 总数 > 0 {
        return Ok((
            验收结论::通过,
            "六准圣一致通过".to_string(),
            用量::default(),
        ));
    }
    if 通过数 == 0 && 总数 > 0 {
        return Ok((
            验收结论::打回,
            "六准圣一致打回".to_string(),
            用量::default(),
        ));
    }

    // 争议：调用鸿钧综合判断
    let 意见文本 = 意见们
        .iter()
        .map(|o| {
            format!(
                "{}: {:?} (评分 {}) - {}",
                o.维度.名称(),
                o.结论,
                o.评分,
                o.关键问题
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let 产物清单 = 产物们
        .iter()
        .map(|p| format!("- {} ({}字节, 变化类型={})", p.路径, p.字节数, p.变化类型))
        .collect::<Vec<_>>()
        .join("\n");
    let 内容摘要 = 产物内容摘要(产物们);

    let 提示 = format!(
        "你是鸿钧道祖，负责验收终裁。\n六准圣意见存在争议（{pass}/{total} 通过），请综合判断。\n\n【要求方向】{dir}\n【验收标准】{std}\n\n【六准圣意见】\n{opinion}\n\n【产物清单】\n{prod}\n【产物内容摘要】（机械提取自产物真实文件）\n{sum}\n\n【现实事实】（执行层已机械验证，勿以假设推翻）\n{fact}\n\n仅输出 JSON：\n{{\"结论\":\"通过|打回\",\"依据\":\"一句话综合理由\"}}\n\n判定原则：\n- 打回：至少 1 个准圣的打回意见为阻断性（接口契约错/编译/孤儿/涉及文件缺失）。\n- 通过：所有打回意见仅为改进建议、非阻断性。\n- 审验型要求且产物为空、要求书允许「已达标则不写文件」、涉及路径文件均存在非空时，「产物缺失」不作为阻断性（设计稿 §11.2 规则 7）。",
        pass = 通过数,
        total = 总数,
        dir = 要求书.方向,
        std = 要求书.验收标准,
        opinion = 意见文本,
        prod = 产物清单,
        sum = 内容摘要,
        fact = 现实事实,
    );

    let (回复, 用量) = moxing_fu::调用模型(配置, &[对话消息::用户(提示)], 常规上限)?;
    let 干净 = 提取对象(&回复).map_err(|错误| format!("鸿钧终裁解析失败: {错误}"))?;
    let 解析: serde_json::Value =
        serde_json::from_str(&干净).map_err(|错误| format!("鸿钧终裁解析失败: {错误}"))?;
    let 结论 = match 解析["结论"].as_str() {
        Some("通过") => 验收结论::通过,
        _ => 验收结论::打回,
    };
    let 依据 = 解析["依据"].as_str().unwrap_or("").to_string();
    Ok((结论, 依据, 用量))
}

// ── 综合意见 ──

/// 综合六准圣打回意见为一句话验收意见（仅打回时返回）。
fn 综合意见文本(
    意见们: &[准圣意见], 结论: &验收结论, 依据: &str
) -> Option<String> {
    if *结论 == 验收结论::通过 {
        return None;
    }
    let 打回项们: Vec<&准圣意见> = 意见们.iter().filter(|o| o.结论 == 验收结论::打回).collect();
    if 打回项们.is_empty() {
        return Some(依据.to_string());
    }
    let 关键问题 = 打回项们
        .iter()
        .map(|o| format!("{}: {}", o.维度.名称(), o.关键问题))
        .collect::<Vec<_>>()
        .join("；");
    Some(format!("准圣打回：{关键问题}"))
}

// ── 终裁主入口 ──

/// 终裁主入口（完整三段，要求书版）。
///
/// 流程：① 机械前置门槛 → ② 六准圣 LLM 审验 → ③ 鸿钧终裁。
/// 无 LLM 配置或无要求书时自动降级为规则兜底。
pub fn 终裁裁决(
    要求书: &要求书,
    产物们: &[产物条目],
    耗时秒: f64,
    涉及文件: &[String],
    失败说明: Option<&str>,
    配置: Option<&模型配置>,
) -> 终裁回执 {
    终裁裁决_无名(
        &要求书.id,
        Some(要求书),
        产物们,
        耗时秒,
        涉及文件,
        失败说明,
        配置,
    )
}

/// 终裁主入口（无名版，允许无要求书——兼容旧 验收裁决 签名）。
///
/// 流程：① 机械前置门槛 → ② 六准圣 LLM 审验 → ③ 鸿钧终裁。
/// 无 LLM 配置或无要求书时自动降级为规则兜底。
pub fn 终裁裁决_无名(
    要求id: &str,
    要求书: Option<&要求书>,
    产物们: &[产物条目],
    耗时秒: f64,
    涉及文件: &[String],
    失败说明: Option<&str>,
    配置: Option<&模型配置>,
) -> 终裁回执 {
    // 白箱观测：验收阶段进入验收角色（栈顶覆盖主政的鸿钧档；无要求书时退化为空要求）。
    let _观测守卫 = 进入观测(观测角色::验收, None, Some(要求id.to_string()), None);
    // ① 机械前置门槛
    if let Some(打回) = 机械前置门槛(要求id, 产物们, 耗时秒, 失败说明) {
        return 打回;
    }

    // ② 降级判定：无 LLM 配置或无要求书 → 规则兜底（兼容旧测试签名）
    let (要求书, 配置) = match (要求书, 配置) {
        (Some(书), Some(配)) => (书, 配),
        _ => return 规则兜底(要求id, 产物们, 耗时秒, 涉及文件),
    };

    // ③ 六准圣分维独立审验
    let (意见们, 用量1) = 六准圣审验(要求书, 产物们, 配置);

    // ④ 鸿钧终裁
    let (终裁结论, 终裁依据, 用量2) = match 鸿钧终裁(要求书, 产物们, &意见们, 配置) {
        Ok(t) => t,
        Err(错误) => {
            error!(要求 = %要求书.id, 错误 = %错误, "鸿钧终裁失败，回退规则兜底");
            return 规则兜底(要求id, 产物们, 耗时秒, 涉及文件);
        }
    };

    // ⑤ 累计用量 + 构造终裁回执
    let mut 用量 = 用量1;
    用量.加(&用量2);

    let 意见文本 = 综合意见文本(&意见们, &终裁结论, &终裁依据);

    info!(
        要求 = %要求书.id,
        结论 = ?终裁结论,
        准圣通过 = 意见们.iter().filter(|o| o.结论 == 验收结论::通过).count(),
        准圣总数 = 意见们.len(),
        "终裁完成"
    );

    终裁回执 {
        验收: 验收回执 {
            要求id: 要求id.to_string(),
            结论: 终裁结论,
            验收意见: 意见文本,
            产物: 产物们.to_vec(),
            耗时秒,
        },
        准圣意见们: 意见们,
        终裁依据,
        用量,
    }
}

// ── 测试 ──

#[cfg(test)]
mod 测试 {
    use super::*;

    /// 本 crate 测试进程级 env 互斥锁：并行测试下 WORLD_WORKSPACE_ROOT 串行使用
    ///（cargo test 各 crate 独立进程，crate 内一把锁即可；证道 侧另有全局 隔离设施 共享锁）。
    static 测试环境锁: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn 产物内容摘要_提取符号与测试() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 根 = std::env::temp_dir().join(format!("终裁摘要测试-{}", std::process::id()));
        std::fs::create_dir_all(&根).unwrap();
        let 文件 = 根.join("样例.rs");
        std::fs::write(
            &文件,
            "pub fn 呈现世界昼夜() -> String { String::new() }\npub struct 时段时间 {}\n#[test]\nfn 验证_时段边界() {}\nfn 内部函数() {}",
        )
        .unwrap();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 产物们 = vec![产物条目 {
            路径: "样例.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 100,
            变化类型: "新增".to_string(),
        }];
        let 摘要 = 产物内容摘要(&产物们);
        assert!(摘要.contains("呈现世界昼夜"), "应含 pub fn 签名");
        assert!(摘要.contains("时段时间"), "应含 pub struct 签名");
        assert!(摘要.contains("验证_时段边界"), "应含 #[test] 函数名");
        assert!(!摘要.contains("内部函数"), "非 pub 函数不应入摘要");
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 产物内容摘要_非rs与空文件不崩() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 根 = std::env::temp_dir().join(format!("终裁摘要测试2-{}", std::process::id()));
        std::fs::create_dir_all(&根).unwrap();
        std::fs::write(根.join("说明.md"), "# 说明").unwrap();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 产物们 = vec![产物条目 {
            路径: "说明.md".to_string(),
            类别: "文档".to_string(),
            字节数: 10,
            变化类型: "新增".to_string(),
        }];
        let 摘要 = 产物内容摘要(&产物们);
        assert_eq!(摘要, "（无非 rs 产物，无内容摘要）");
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 机械门槛_无产物放行交准圣() {
        // 执行成功但无产物 = 现状已达标无需改动，机械门槛不拦，交六准圣 LLM 判断（设计稿 §11.2 规则 5）。
        let 回执 = 机械前置门槛("r1", &[], 0.0, None);
        assert!(回执.is_none(), "执行成功无产物不应被机械门槛打回");
    }

    #[test]
    fn 机械门槛_编译失败直接打回() {
        let 回执 = 机械前置门槛("r1", &[], 0.0, Some("cargo build 失败")).expect("应直接打回");
        assert_eq!(回执.验收.结论, 验收结论::打回);
        assert_eq!(回执.验收.验收意见.as_deref(), Some("cargo build 失败"));
    }

    #[test]
    fn 机械门槛_上跳路径越界打回() {
        let 产物们 = vec![产物条目 {
            路径: "../越界.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 1,
            变化类型: "新增".to_string(),
        }];
        let 回执 = 机械前置门槛("r1", &产物们, 0.0, None).expect("上跳路径应直接打回");
        assert_eq!(回执.验收.结论, 验收结论::打回);
        assert!(回执
            .验收
            .验收意见
            .as_deref()
            .unwrap_or("")
            .contains("路径越界"));
    }

    #[test]
    fn 机械门槛_绝对路径越界打回() {
        let 产物们 = vec![产物条目 {
            路径: "C:/temp/越界.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 1,
            变化类型: "新增".to_string(),
        }];
        let 回执 = 机械前置门槛("r1", &产物们, 0.0, None).expect("绝对路径应直接打回");
        assert_eq!(回执.验收.结论, 验收结论::打回);
        assert!(回执
            .验收
            .验收意见
            .as_deref()
            .unwrap_or("")
            .contains("路径越界"));
    }

    #[test]
    fn 机械门槛_正常相对路径放行() {
        let 产物们 = vec![产物条目 {
            路径: "鸿蒙/基础设施 - 域/入口.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 1,
            变化类型: "新增".to_string(),
        }];
        let 回执 = 机械前置门槛("r1", &产物们, 0.0, None);
        if let Some(终裁) = 回执 {
            assert!(!终裁
                .验收
                .验收意见
                .as_deref()
                .unwrap_or("")
                .contains("路径越界"))
        }
    }

    #[test]
    fn 路径相符_统一分隔符与大小写() {
        let 涉及 = vec!["鸿蒙/基础设施 - 域/天庭治理-府/入口.rs".to_string()];
        let 产物们 = vec![
            产物条目 {
                路径: "鸿蒙\\基础设施 - 域\\天庭治理-府\\入口.rs".to_string(),
                类别: "代码".to_string(),
                字节数: 100,
                变化类型: "修改".to_string(),
            },
            产物条目 {
                路径: "其他文件.rs".to_string(),
                类别: "代码".to_string(),
                字节数: 100,
                变化类型: "新增".to_string(),
            },
        ];
        assert!(路径相符(&涉及, &产物们), "反斜杠应等价为正斜杠");
    }

    #[test]
    fn 路径相符_涉及为空时通过() {
        let 产物们 = vec![产物条目 {
            路径: "任意路径.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 1,
            变化类型: "修改".to_string(),
        }];
        assert!(路径相符(&[], &产物们));
    }

    #[test]
    fn 涉及路径现状_存在缺失与空文件() {
        // 审验型要求产物为空时，准圣依赖「涉及现状」核验真伪（设计稿 §11.2 规则 7）。
        let 根 = std::env::temp_dir().join(format!("涉及现状测试-{}", shihai_fu::当前毫秒()));
        std::fs::create_dir_all(&根).unwrap();
        std::fs::write(根.join("存在的.rs"), "pub fn 有内容() {}\n").unwrap();
        std::fs::write(根.join("空的.rs"), "").unwrap();
        let 锁 = 测试环境锁.lock().unwrap_or_else(|e| e.into_inner());
        let 旧根 = std::env::var("WORLD_WORKSPACE_ROOT").ok();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 现状 = 涉及路径现状(&[
            "存在的.rs".to_string(),
            "空的.rs".to_string(),
            "缺失的.rs".to_string(),
        ]);
        match 旧根 {
            Some(值) => std::env::set_var("WORLD_WORKSPACE_ROOT", 值),
            None => std::env::remove_var("WORLD_WORKSPACE_ROOT"),
        }
        drop(锁);
        let _ = std::fs::remove_dir_all(&根);
        assert!(
            现状.contains("存在的.rs（当前22字节"),
            "应有真实字节数：{现状}"
        );
        assert!(现状.contains("空的.rs（当前空文件"), "空文件应标注：{现状}");
        assert!(现状.contains("缺失的.rs（当前不存在"), "缺失应标注：{现状}");
        assert!(
            现状.contains("无执行前基线记录"),
            "无执行基线时应明示：{现状}"
        );
    }

    #[test]
    fn 涉及路径现状_与执行前基线对比() {
        // 执行前基线落盘后，审验材料应给出「改前 → 改后」增量证据，防准圣误判未变。
        let 根 = std::env::temp_dir().join(format!("涉及现状基线测试-{}", shihai_fu::当前毫秒()));
        std::fs::create_dir_all(根.join(".上下文").join("状态")).unwrap();
        std::fs::write(
            根.join("流式历法.rs"),
            "pub fn 呈现世界历法() {} // 改动后\n",
        )
        .unwrap();
        let 基线 = shihai_fu::文件索引 {
            指纹们: {
                let mut 图 = std::collections::BTreeMap::new();
                图.insert(
                    "流式历法.rs".to_string(),
                    shihai_fu::文件指纹 {
                        大小: 100, 修改: 1
                    },
                );
                图.insert(
                    "新增园.rs".to_string(),
                    shihai_fu::文件指纹 {
                        大小: 50, 修改: 2
                    },
                );
                图
            },
        };
        shihai_fu::保存执行基线(&shihai_fu::工作区::新(&根), &基线).unwrap();
        let 锁 = 测试环境锁.lock().unwrap_or_else(|e| e.into_inner());
        let 旧根 = std::env::var("WORLD_WORKSPACE_ROOT").ok();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 现状 = 涉及路径现状(&[
            "流式历法.rs".to_string(),
            "新增园.rs".to_string(),
            "缺失园.rs".to_string(),
        ]);
        match 旧根 {
            Some(值) => std::env::set_var("WORLD_WORKSPACE_ROOT", 值),
            None => std::env::remove_var("WORLD_WORKSPACE_ROOT"),
        }
        drop(锁);
        let _ = std::fs::remove_dir_all(&根);
        assert!(现状.contains("流式历法.rs（当前"), "应有当前字节数：{现状}");
        assert!(
            现状.contains("执行前基线 100字节"),
            "修改文件应展示改前基线：{现状}"
        );
        assert!(
            现状.contains("新增园.rs（当前不存在；执行前基线 50字节"),
            "基线存在但当前缺失应明示：{现状}"
        );
        assert!(
            现状.contains("缺失园.rs（当前不存在；执行前基线无此文件（本轮新增）"),
            "基线无此文件应标注本轮新增：{现状}"
        );
    }

    #[test]
    fn 综合意见_全通过时为_none() {
        let 意见们 = vec![准圣意见 {
            维度: 准圣维度::业务正确性,
            结论: 验收结论::通过,
            评分: 90,
            关键问题: "OK".to_string(),
            改进建议: vec![],
        }];
        assert!(综合意见文本(&意见们, &验收结论::通过, "OK").is_none());
    }

    #[test]
    fn 综合意见_有打回时拼接关键问题() {
        let 意见们 = vec![
            准圣意见 {
                维度: 准圣维度::业务正确性,
                结论: 验收结论::打回,
                评分: 30,
                关键问题: "缺一项验收标准".to_string(),
                改进建议: vec![],
            },
            准圣意见 {
                维度: 准圣维度::异常兼容,
                结论: 验收结论::通过,
                评分: 80,
                关键问题: "OK".to_string(),
                改进建议: vec![],
            },
        ];
        let 文本 = 综合意见文本(&意见们, &验收结论::打回, "依据").expect("应返回文本");
        assert!(文本.contains("业务正确性准圣"));
        assert!(文本.contains("缺一项验收标准"));
    }

    #[test]
    fn 终裁回执_序列化后旧验收回执可解析() {
        // flatten 兼容性保证：完整终裁回执落盘后，历史读取方按旧 验收回执 解析不得失败。
        let 回执 = 终裁回执 {
            验收: 验收回执 {
                要求id: "r1".to_string(),
                结论: 验收结论::通过,
                验收意见: None,
                产物: vec![产物条目 {
                    路径: "入口.rs".to_string(),
                    类别: "代码".to_string(),
                    字节数: 10,
                    变化类型: "修改".to_string(),
                }],
                耗时秒: 0.0,
            },
            准圣意见们: vec![准圣意见 {
                维度: 准圣维度::业务正确性,
                结论: 验收结论::通过,
                评分: 90,
                关键问题: "OK".to_string(),
                改进建议: vec![],
            }],
            终裁依据: "六准圣一致通过".to_string(),
            用量: 用量::default(),
        };
        let 行 = serde_json::to_string(&回执).expect("终裁回执应可序列化");
        assert!(行.contains("准圣意见们"), "明细应随落盘：{行}");
        assert!(行.contains("终裁依据"), "终裁依据应随落盘：{行}");
        // 旧读取方按 验收回执 解析：顶层字段齐全则成功，多出字段被忽略。
        let 旧: 验收回执 = serde_json::from_str(&行).expect("旧读取方解析完整终裁回执行不得失败");
        assert_eq!(旧.要求id, "r1");
        assert_eq!(旧.结论, 验收结论::通过);
        assert_eq!(旧.产物.len(), 1);
    }

    #[test]
    fn 终裁裁决_无名_无_llm降级为规则兜底() {
        // 真实临时目录构造含模块树的产物
        let 根 = std::env::temp_dir().join(format!("终裁降级测试-{}", shihai_fu::当前毫秒()));
        let 园 = 根.join("观览-查询-殿/世界-观览-阁/流式-读取-园");
        std::fs::create_dir_all(&园).unwrap();
        std::fs::write(根.join("Cargo.toml"), "[package]\nname = \"测试-府\"\n").unwrap();
        std::fs::write(
            根.join("入口.rs"),
            "#[path = \"观览-查询-殿/模块.rs\"]\npub mod 观览_查询_殿;\n",
        )
        .unwrap();
        std::fs::write(
            根.join("观览-查询-殿/模块.rs"),
            "#[path = \"世界-观览-阁/模块.rs\"]\npub mod 世界_观览_阁;\npub use 世界_观览_阁::*;\n",
        )
        .unwrap();
        std::fs::write(
            根.join("观览-查询-殿/世界-观览-阁/模块.rs"),
            "#[path = \"流式-读取-园/模块.rs\"]\npub mod 流式_读取_园;\npub use 流式_读取_园::*;\n",
        )
        .unwrap();
        std::fs::write(
            园.join("模块.rs"),
            "#[path = \"流式读取.rs\"]\npub mod 流式读取;\npub use 流式读取::*;\n",
        )
        .unwrap();
        std::fs::write(
            园.join("流式读取.rs"),
            "pub fn 呈现世界时间() -> String {\"时间\".to_string()}\n",
        )
        .unwrap();

        let 锁 = 测试环境锁.lock().unwrap_or_else(|e| e.into_inner());
        let 旧根 = std::env::var("WORLD_WORKSPACE_ROOT").ok();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 产物们 = vec![产物条目 {
            路径: "观览-查询-殿/世界-观览-阁/流式-读取-园/流式读取.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 1,
            变化类型: "新增".to_string(),
        }];
        let 终裁 = 终裁裁决_无名("r1", None, &产物们, 0.0, &[], None, None);
        match 旧根 {
            Some(值) => std::env::set_var("WORLD_WORKSPACE_ROOT", 值),
            None => std::env::remove_var("WORLD_WORKSPACE_ROOT"),
        }
        drop(锁);
        let _ = std::fs::remove_dir_all(&根);

        assert_eq!(
            终裁.验收.结论,
            验收结论::通过,
            "降级路径应走规则兜底，模块树已接入应通过：{:?}",
            终裁.验收.验收意见
        );
        assert!(终裁.终裁依据.contains("规则兜底"));
        assert!(终裁.准圣意见们.is_empty(), "降级路径不应触发 LLM 准圣");
    }
}
