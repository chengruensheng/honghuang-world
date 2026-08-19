//! 对话-循环-园：界主发言 → 落对话记录 → 意图判别 → 分流（闲聊/发布任务/追问/干预/点名）。
//! 设计稿 §1.5.5：鸿钧 = 界主的对话伙伴 + 任务总控；界主只跟鸿钧说话。
//! 消息可见性：界主发言非@仅鸿钧可见、@点名才带上该角色；鸿钧答复在界主-鸿钧之间。

use crate::类型_定义_殿::{想法, 想法状态};
use crate::{判别, 判别结果, 对话意图};
use daoshu_fu::任务调度;
use jiance_fu::{观测角色, 进入观测};
use moxing_fu::{对话消息, 模型配置, 精简上限, 调用模型};
use rizhi_fu::{error, info, warn};

/// 状态目录：工作区根下的 .上下文/状态（与 世界运行.rs 同款，本园复制以保持跨府引用只走 lib 根符号的边界）。
fn 状态目录() -> std::path::PathBuf {
    let 根 = std::env::var("WORLD_WORKSPACE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    根.join(".上下文").join("状态")
}

/// 界主发言入口：判别意图 → 分流 → 返回鸿钧答复（答复同时落对话记录）。
/// 存储/调度 参数保留（公共 API 形状稳定）：阶段 1 发布任务改异步登记后暂不直用，
/// 阶段 3 任务线机制接管执行；后续对话内直跑（如 追问详情 调 LLM）会复用。
pub fn 界主发言(
    消息: &str,
    配置: &模型配置,
    _存储: &shihai_fu::模型存储,
    _调度: &mut 任务调度,
) -> String {
    // 白箱观测：界主对话整链进入鸿钧角色（无要求关联——对话不绑定某要求书）。
    let _观测守卫 = 进入观测(观测角色::鸿钧, None, None, None);
    let 消息 = 消息.trim();
    if 消息.is_empty() {
        return "请说点什么".to_string();
    }
    let 判别结果 = 判别(消息, 配置);
    // 界主消息可见性：默认仅鸿钧；@点名才带上该角色。
    let mut 可见 = vec!["鸿钧".to_string()];
    if let Some(角色) = &判别结果.点名角色 {
        if !可见.contains(角色) {
            可见.push(角色.clone());
        }
    }
    crate::落对话记录("界主", 消息, &可见);

    let 答复 = match 判别结果.意图 {
        对话意图::闲聊 => 闲聊回复(消息, 配置),
        对话意图::发布任务 => {
            // 发布前澄清（生产化 1.4）：缺方向或验收标准 → 鸿钧反问补齐，不直接开跑。
            // 防「帮我修一下」这种模糊指令直接进全链烧预算（2026-08-17 实测教训）。
            let 缺方向 = 判别结果.方向.trim().is_empty();
            let 缺验收 = 判别结果.验收标准.trim().is_empty();
            if 缺方向 || 缺验收 {
                let 缺 = if 缺方向 { "要做什么（方向）" } else { "怎样算完成（验收标准）" };
                let 补全示例 = if 缺方向 {
                    "方向=新增「世界 昼夜」只读命令"
                } else {
                    "验收标准=cargo build --workspace 与 cargo test 全部通过"
                };
                format!(
                    "收到。开工前先对齐一下：你还没说「{缺}」。请补全后我再召集圣人商讨，例如：{补全示例}。"
                )
            } else {
                发布任务(消息, &判别结果)
            }
        }
        对话意图::追问进度 => 追问进度回复(消息),
        对话意图::中途干预 => {
            "中途干预（停止/打回/加急）在阶段 3 开放：当前可通过「号令 世界 驱动」或守护模式管理任务线。".to_string()
        }
        对话意图::点名角色 => {
            let 角色 = 判别结果.点名角色.as_deref().unwrap_or("该角色");
            format!("@{角色} 已收到你的点名。消息直达在阶段 3 开放，当前已记录本条。")
        }
    };
    crate::落对话记录("鸿钧", &答复, &["界主".to_string(), "鸿钧".to_string()]);
    答复
}

/// 追问进度分流（生产化 1.2）：消息含「要求-N」→ 展开该要求最新回执详情（终裁依据/验收意见/准圣意见）；
/// 否则给统计视图（要求/任务线/最近验收）。全部机械汇总真实回执，不调 LLM、不编造。
fn 追问进度回复(消息: &str) -> String {
    let 目录 = 状态目录();
    // 命中「要求-N」→ 详情模式。
    let 目标 = 提取要求id(消息);
    if let Some(要求id) = 目标 {
        return 要求详情(&目录, &要求id);
    }
    // 要求现状：读要求.jsonl 全部，按状态统计。
    let 要求队列 = crate::落盘队列::<crate::要求书>::打开(目录.join("要求.jsonl"));
    let 要求们 = 要求队列.读全部().unwrap_or_default();
    let 状态数 = 要求们
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut 表, 要求| {
            *表.entry(format!("{:?}", 要求.状态)).or_insert(0usize) += 1;
            表
        });
    let 状态段 = if 状态数.is_empty() {
        "（无要求记录）".to_string()
    } else {
        状态数
            .iter()
            .map(|(状态, 数)| format!("{状态} {数} 条"))
            .collect::<Vec<_>>()
            .join("；")
    };
    // 最近验收：验收.jsonl 尾部 5 条。
    let 验收队列 = crate::落盘队列::<crate::终裁回执>::打开(目录.join("验收.jsonl"));
    let 验收们 = 验收队列.读全部().unwrap_or_default();
    let 尾部 = 验收们.iter().rev().take(5).rev();
    let 验收段 = if 验收们.is_empty() {
        "（无验收记录）".to_string()
    } else {
        尾部
            .map(|回执| format!("{} {:?}", 回执.验收.要求id, 回执.验收.结论))
            .collect::<Vec<_>>()
            .join("；")
    };
    // 任务线：按状态统计（阶段 3）。
    let 任务线们 = crate::读任务线们().unwrap_or_default();
    let 线状态数 = 任务线们
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut 表, 线| {
            *表.entry(format!("{:?}", 线.状态)).or_insert(0usize) += 1;
            表
        });
    let 线段 = if 线状态数.is_empty() {
        "（无任务线）".to_string()
    } else {
        线状态数
            .iter()
            .map(|(状态, 数)| format!("{状态} {数} 条"))
            .collect::<Vec<_>>()
            .join("；")
    };
    format!("当前世界状态\n要求：{状态段}\n任务线：{线段}\n最近验收：{验收段}")
}

/// 从追问文本提取「要求-N」（数字序号）。
fn 提取要求id(消息: &str) -> Option<String> {
    let 字符们: Vec<char> = 消息.chars().collect();
    for 起点 in 0..字符们.len() {
        if 字符们[起点..].iter().take(3).collect::<String>() == "要求" {
            // 要求 后须跟 -数字
            let 剩余: String = 字符们[起点 + 2..].iter().collect();
            if let Some(尾) = 剩余.strip_prefix('-') {
                let 数字: String = 尾.chars().take_while(|字| 字.is_ascii_digit()).collect();
                if !数字.is_empty() {
                    return Some(format!("要求-{数字}"));
                }
            }
        }
    }
    None
}

/// 要求详情：该要求在验收.jsonl 的最新回执（终裁依据/验收意见/准圣意见摘要）。
fn 要求详情(目录: &std::path::Path, 要求id: &str) -> String {
    let 验收队列 = crate::落盘队列::<crate::终裁回执>::打开(目录.join("验收.jsonl"));
    let 验收们 = 验收队列.读全部().unwrap_or_default();
    let 最新 = 验收们.iter().rev().find(|回执| 回执.验收.要求id == 要求id);
    let Some(回执) = 最新 else {
        return format!("{要求id} 暂无验收记录（可能尚未执行或未定档）");
    };
    let mut 行们 = vec![format!(
        "{要求id} 验收详情\n结论：{:?}\n验收意见：{}",
        回执.验收.结论,
        回执.验收.验收意见.as_deref().unwrap_or("（无）")
    )];
    if !回执.终裁依据.is_empty() {
        行们.push(format!("终裁依据：{}", 回执.终裁依据));
    }
    if !回执.准圣意见们.is_empty() {
        行们.push("六准圣意见：".to_string());
        for 意见 in &回执.准圣意见们 {
            行们.push(format!(
                "  {} {:?}（{} 分）：{}",
                意见.维度.名称(),
                意见.结论,
                意见.评分,
                意见.关键问题
            ));
        }
    }
    if !回执.验收.产物.is_empty() {
        行们.push(format!(
            "产物：{}",
            回执
                .验收
                .产物
                .iter()
                .map(|产物| format!("{}（{} {}）", 产物.路径, 产物.变化类型, 产物.字节数))
                .collect::<Vec<_>>()
                .join("、")
        ));
    }
    行们.join("\n")
}

/// 闲聊分流：鸿钧人格直接回应（轻量调用，失败兜底不阻塞对话）。
fn 闲聊回复(消息: &str, 配置: &模型配置) -> String {
    let 提示 = format!(
        "你是鸿钧，天层主政之神，界主的对话伙伴。界主对你说：{消息}\n\
请自然回应，1-3 句即可。项目相关的话题可以简要说说你的看法；与项目无关的轻松回应。"
    );
    match 调用模型(配置, &[对话消息::用户(&提示)], 精简上限) {
        Ok((回复, _)) => {
            let 回复 = 回复.trim().to_string();
            if 回复.is_empty() {
                "在听，请继续说。".to_string()
            } else {
                回复
            }
        }
        Err(错误) => {
            warn!(错误 = %错误, "闲聊回复调用失败，兜底回应");
            "在听，请继续说。".to_string()
        }
    }
}

/// 发布任务分流（阶段 3）：构造想法 → 登记任务线（待执行）→ 返回受理文本。
/// 任务线由「世界 守护」常驻消费或「世界 驱动」手动消费；完成后鸿钧汇报进对话记录。
fn 发布任务(消息: &str, 判别: &判别结果) -> String {
    let 内容 = 拼装任务文本(消息, 判别);
    let 想法 = 想法 {
        id: crate::唯一id("想法"),
        内容,
        时间: shihai_fu::当前毫秒(),
        状态: 想法状态::未处理,
    };
    match crate::登记任务线(&想法) {
        Ok(任务线) => {
            info!(想法id = %想法.id, 任务线id = %任务线.id, "对话发布任务，任务线已登记");
            format!(
                "任务已受理（任务线：{}，待执行）\n启动「号令 世界 守护」会自动执行；或「号令 世界 驱动」立即执行一条。",
                任务线.id
            )
        }
        Err(错误) => {
            error!(想法id = %想法.id, "任务线登记失败：{错误}");
            format!("任务受理失败：{错误}")
        }
    }
}

/// 拼装任务文本：界主原文优先 + 验收标准/涉及路径补充。
/// 原文比方向更详细时用原文（防意图判别摘要丢失指令细节，
/// 2026-08-19 实测：界主发475字符详细指令被摘要成130字符方向，世界收不到完整任务）；
/// 原文简短时（如「帮我修一下」）用模型提取的方向。
fn 拼装任务文本(消息: &str, 判别: &判别结果) -> String {
    let mut 文本 = if 消息.len() > 判别.方向.len() || 判别.方向.is_empty() {
        消息.to_string()
    } else {
        判别.方向.clone()
    };
    if !判别.验收标准.is_empty() && !文本.contains(&判别.验收标准) {
        文本.push_str(&format!("。验收标准：{}", 判别.验收标准));
    }
    if !判别.涉及路径.is_empty() {
        let 路径文 = 判别.涉及路径.join("、");
        if !文本.contains(&路径文) {
            文本.push_str(&format!("。涉及路径：{}", 路径文));
        }
    }
    文本
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 拼装任务文本_方向为空用原话() {
        let 判别 = 判别结果 {
            意图: 对话意图::发布任务,
            方向: String::new(),
            验收标准: String::new(),
            涉及路径: vec![],
            点名角色: None,
        };
        assert_eq!(拼装任务文本("给我写个测试", &判别), "给我写个测试");
    }

    #[test]
    fn 拼装任务文本_带验收与路径() {
        let 判别 = 判别结果 {
            意图: 对话意图::发布任务,
            方向: "新增世界 昼夜 命令".to_string(),
            验收标准: "cargo test 通过".to_string(),
            涉及路径: vec!["乾坤/呈现-域".to_string()],
            点名角色: None,
        };
        let 文本 = 拼装任务文本("忽略", &判别);
        assert!(文本.contains("新增世界 昼夜 命令"));
        assert!(文本.contains("cargo test 通过"));
        assert!(文本.contains("乾坤/呈现-域"));
    }

    #[test]
    fn 拼装任务文本_原文比方向长时保留原文() {
        // 界主发详细指令（475字符）不应被意图判别摘要（130字符）覆盖
        let 判别 = 判别结果 {
            意图: 对话意图::发布任务,
            方向: "修复3个问题".to_string(),
            验收标准: String::new(),
            涉及路径: vec![],
            点名角色: None,
        };
        let 原文 = "修复原子校验.rs剩余3个质量问题，问题1改use导入，问题2补白名单，问题3改日志宏。验收标准：cargo test通过。涉及路径：乾坤/原子校验.rs";
        let 文本 = 拼装任务文本(原文, &判别);
        assert!(
            文本.contains("修复原子校验.rs剩余3个质量问题"),
            "原文详细时应保留原文而非短摘要：{文本}"
        );
    }

    #[test]
    fn 拼装任务文本_验收标准去重不重复追加() {
        // 原文已含验收标准时不再重复追加
        let 判别 = 判别结果 {
            意图: 对话意图::发布任务,
            方向: String::new(),
            验收标准: "cargo test 通过".to_string(),
            涉及路径: vec![],
            点名角色: None,
        };
        let 原文 = "补测试。验收标准：cargo test 通过";
        let 文本 = 拼装任务文本(原文, &判别);
        let 次数 = 文本.matches("cargo test 通过").count();
        assert_eq!(次数, 1, "验收标准不应重复追加：{文本}");
    }

    /// 兼容回归：真实验收.jsonl（含旧六维历史记录）可被 终裁回执 全量反序列化（追问进度/流水观览依赖）。
    #[test]
    #[ignore = "需真实工作区"]
    fn 兼容_真实验收jsonl全量可解析() {
        std::env::set_var("WORLD_WORKSPACE_ROOT", "D:\\洪荒 - 世界");
        let 目录 = 状态目录();
        let 队列 = crate::落盘队列::<crate::终裁回执>::打开(目录.join("验收.jsonl"));
        let 回执们 = 队列.读全部().expect("历史验收记录（含旧六维）应全量可解析");
        assert!(!回执们.is_empty(), "真实验收.jsonl 不应为空");
    }
}
