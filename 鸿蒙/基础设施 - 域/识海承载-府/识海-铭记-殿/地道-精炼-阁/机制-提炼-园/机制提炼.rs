//! 机制 - 提炼 - 园：多维度触发判定 → 汇变更与波及文件的函数切片 → 唤 LLM 归纳机制 → 理解·记忆格位。
//!
//! 定位：地道在执行期做增量收尾——小变更只记痕迹、零 LLM 成本；
//! 复杂变更白嫖任务生成等待期唤 LLM，把耦合文件共同支撑的机制沉淀下来。
//! 触发判定（§14.10）综合三维度：变更类型 / 影响范围 / 教训关联。

use crate::推演波及;
use crate::{依赖图, 变更报告, 教训格位, 模型存储, 记录};
use jiance_fu::{观测角色, 进入观测};
use moxing_fu::{对话消息, 模型配置, 精简上限, 调用模型};
use rizhi_fu::info;
use std::collections::HashSet;
use std::path::Path;

/// 复杂阈值：变更文件数达到此值才唤起 LLM 机制归纳（对齐地道原型·复杂阈值）。
pub const 复杂阈值: usize = 3;

/// 教训密集阈值：近期有效教训达到此值视为"反复踩坑"，降低机制归纳门槛。
const 教训密集阈值: usize = 5;

/// 机制归纳：判定触发 → 汇变更+波及文件切片 → 唤 LLM 提炼机制 → 写「理解·记忆」格位。
/// 返回 Some(归纳文本) 表示已归纳；None 表示未触发（判定未过 / 素材为空）。
pub fn 机制归纳(
    存储: &模型存储,
    配置: &模型配置,
    图: &依赖图,
    根: &Path,
    报告: &变更报告,
) -> Result<Option<String>, String> {
    // 白箱观测：地道归纳进入归因角色（世界级记忆维护，无要求关联）。
    let _观测守卫 = 进入观测(观测角色::归因, None, None, None);
    let (触发, 缘由) = 判定触发(存储, 报告);
    if !触发 {
        info!("机制归纳未触发：{}", 缘由);
        return Ok(None);
    }
    info!("机制归纳触发：{}", 缘由);
    let 波及 = 推演波及(图, 报告);
    let 素材 = 汇切片(图, 根, &波及.变更们, &波及.波及们);
    if 素材.trim().is_empty() {
        return Ok(None);
    }
    let 提示 = format!(
        "你是一名项目记忆采集员。以下是一批有耦合关系的文件（变更文件与受波及文件），共同支撑一个机制。\
         请用简体中文提炼这个机制，直接给结论，不要输出思考过程。\
         结构化输出：【机制名】一句话命名【协同完成什么】它们协同完成什么【边界在哪】边界在哪【改动注意】改动时要注意什么。\
         精炼成一条 200 字以内的记录。\n\n【素材】\n{素材}"
    );
    let (回复, 用量) = 调用模型(配置, &[对话消息::用户(提示)], 精简上限)?;
    存储.写记录(&记录::新(
        "理解·记忆",
        &回复,
        &format!("地道机制归纳：{缘由}"),
        "LLM",
    ))?;
    info!(
        变更数 = 波及.变更们.len(),
        波及数 = 波及.波及们.len(),
        内容长度 = 回复.len(),
        提示词 = 用量.提示词,
        "机制归纳完成"
    );
    Ok(Some(回复))
}

/// 触发判定：综合变更类型、影响范围、教训关联三维度，决定是否唤 LLM 归纳机制。
/// 返回 (是否触发, 触发缘由)——缘由落日志供界主诊断。
/// 优先级：复杂变更（处数≥复杂阈值）> 新增多（≥2）> 跨府 > 跨维 > 教训密集。
fn 判定触发(存储: &模型存储, 报告: &变更报告) -> (bool, String) {
    let 总处数 = 报告.总处数();
    // 1 处变更不构成"机制"
    if 总处数 < 2 {
        return (false, format!("变更仅 {总处数} 处，不构成机制"));
    }
    // 原逻辑保留：复杂变更（处数 ≥ 复杂阈值）
    if 总处数 >= 复杂阈值 {
        return (true, format!("复杂变更 {总处数} 处"));
    }
    // 维度一·变更类型：新增 ≥2 处 → 新机制落地
    if 报告.新增.len() >= 2 {
        return (true, format!("新增 {} 处（新机制落地）", 报告.新增.len()));
    }
    // 维度二·影响范围：跨府 → 跨边界变更
    let 府集: HashSet<&str> = 报告
        .新增
        .iter()
        .chain(报告.修改.iter())
        .filter_map(|p| 府名(p))
        .collect();
    if 府集.len() >= 2 {
        return (true, format!("跨 {} 府变更", 府集.len()));
    }
    let 维集: HashSet<&str> = 报告
        .新增
        .iter()
        .chain(报告.修改.iter())
        .filter_map(|p| 维名(p))
        .collect();
    if 维集.len() >= 2 {
        return (true, format!("跨 {} 维变更", 维集.len()));
    }
    // 维度三·教训关联：近期教训密集 → 反复踩坑
    let 近期教训数 = 存储
        .读格位(教训格位)
        .map(|记录们| 记录们.iter().rev().take(10).filter(|r| !r.失效).count())
        .unwrap_or(0);
    if 近期教训数 >= 教训密集阈值 {
        return (true, format!("近期教训 {近期教训数} 条（反复踩坑）"));
    }
    (false, format!("变更 {总处数} 处，未达任一触发条件"))
}

/// 从路径提取维度名（第 1 段，如「鸿蒙」「乾坤」「证道」）。
/// 路径为正斜杠相对路径（对齐 变更报告 约定）。
fn 维名(路径: &str) -> Option<&str> {
    路径.split('/').next().filter(|s| !s.is_empty())
}

/// 从路径提取府名（第 3 段，如「识海承载-府」「天庭治理-府」）。
/// 路径形如 `鸿蒙/基础设施 - 域/识海承载-府/...`。
fn 府名(路径: &str) -> Option<&str> {
    路径.split('/').nth(2).filter(|s| !s.is_empty())
}

/// 汇函数切片：变更与波及文件 → 符号定义体（符号未入库时兜底读全文前 60 行）。
fn 汇切片(图: &依赖图, 根: &Path, 变更们: &[String], 波及们: &[String]) -> String {
    let mut 素材 = String::new();
    for 文件 in 变更们.iter().chain(波及们.iter()) {
        let 档案们 = 图.查文件(文件);
        let 定义们: Vec<&str> = 档案们
            .iter()
            .filter(|档案| !档案.代码.is_empty())
            .map(|档案| 档案.代码.as_str())
            .collect();
        if 定义们.is_empty() {
            if let Ok(内容) = std::fs::read_to_string(根.join(文件)) {
                素材.push_str(&format!(
                    "【文件：{文件}】\n{}\n\n",
                    内容.lines().take(60).collect::<Vec<_>>().join("\n")
                ));
            }
            continue;
        }
        素材.push_str(&format!("【文件：{文件}】\n{}\n\n", 定义们.join("\n")));
    }
    素材
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::符号档案;

    /// 造一个不触网的假配置（机制归纳只在超阈值 + 有素材时才调 LLM，假配置足矣）。
    fn 假配置() -> 模型配置 {
        模型配置 {
            密钥: String::new(),
            地址: String::new(),
            模型: String::new(),
        }
    }

    #[test]
    fn 机制归纳_低于阈值不唤_llm() {
        let 存储 = crate::模型存储::打开(
            std::env::temp_dir().join(format!("机制阈值测试-{}", crate::当前毫秒())),
        );
        let 报告 = 变更报告 {
            新增: vec!["甲.rs".to_string()],
            修改: Vec::new(),
            删除: Vec::new(),
        };
        let 图 = 依赖图::default();
        // 若误唤 LLM，空密钥必报错；返回 None 即证明未触发。
        let 结果 = 机制归纳(&存储, &假配置(), &图, std::path::Path::new("."), &报告).unwrap();
        assert_eq!(结果, None);
    }

    #[test]
    fn 机制归纳_超阈值但素材为空不唤_llm() {
        let 存储 = crate::模型存储::打开(
            std::env::temp_dir().join(format!("机制空素材测试-{}", crate::当前毫秒())),
        );
        let 报告 = 变更报告 {
            新增: vec!["不存在的甲.rs".to_string(), "不存在的乙.rs".to_string()],
            修改: vec!["不存在的丙.rs".to_string()],
            删除: Vec::new(),
        };
        // 依赖图空 + 根下无文件 → 素材空 → 返回 None，不唤 LLM。
        let 图 = 依赖图::default();
        let 根 = std::env::temp_dir().join(format!("机制空素材根-{}", crate::当前毫秒()));
        let 结果 = 机制归纳(&存储, &假配置(), &图, &根, &报告).unwrap();
        assert_eq!(结果, None);
    }

    #[test]
    fn 汇切片_有符号走定义体() {
        let 图 = 依赖图 {
            档案们: vec![符号档案::新(
                "项目",
                "模块",
                "甲.rs",
                "甲符号",
                "pub fn 甲() {}",
                "签名",
                "解释",
            )],
            ..Default::default()
        };
        let 素材 = 汇切片(&图, std::path::Path::new("."), &["甲.rs".to_string()], &[]);
        assert!(素材.contains("甲.rs"));
        assert!(素材.contains("pub fn 甲() {}"));
    }

    #[test]
    fn 判定触发_单处变更不触发() {
        let 存储 = crate::模型存储::打开(
            std::env::temp_dir().join(format!("触发单处-{}", crate::当前毫秒())),
        );
        let 报告 = 变更报告 {
            新增: vec!["鸿蒙/基础设施 - 域/识海承载-府/甲.rs".to_string()],
            修改: Vec::new(),
            删除: Vec::new(),
        };
        let (触发, _) = 判定触发(&存储, &报告);
        assert!(!触发);
    }

    #[test]
    fn 判定触发_新增两处触发() {
        let 存储 = crate::模型存储::打开(
            std::env::temp_dir().join(format!("触发新增-{}", crate::当前毫秒())),
        );
        let 报告 = 变更报告 {
            新增: vec![
                "鸿蒙/基础设施 - 域/识海承载-府/甲.rs".to_string(),
                "鸿蒙/基础设施 - 域/识海承载-府/乙.rs".to_string(),
            ],
            修改: Vec::new(),
            删除: Vec::new(),
        };
        let (触发, 缘由) = 判定触发(&存储, &报告);
        assert!(触发);
        assert!(缘由.contains("新机制落地"), "应因新增触发：{缘由}");
    }

    #[test]
    fn 判定触发_跨府触发() {
        let 存储 = crate::模型存储::打开(
            std::env::temp_dir().join(format!("触发跨府-{}", crate::当前毫秒())),
        );
        // 1 新增 + 1 修改 = 2 处（< 复杂阈值=3），但跨两个府 → 触发
        let 报告 = 变更报告 {
            新增: vec!["鸿蒙/基础设施 - 域/识海承载-府/甲.rs".to_string()],
            修改: vec!["鸿蒙/基础设施 - 域/天庭治理-府/乙.rs".to_string()],
            删除: Vec::new(),
        };
        let (触发, 缘由) = 判定触发(&存储, &报告);
        assert!(触发);
        assert!(缘由.contains("跨"), "应因跨府触发：{缘由}");
    }

    #[test]
    fn 判定触发_跨维触发() {
        let 存储 = crate::模型存储::打开(
            std::env::temp_dir().join(format!("触发跨维-{}", crate::当前毫秒())),
        );
        // 1 新增 + 1 修改 = 2 处，同府但跨维度（鸿蒙 vs 乾坤）→ 触发
        let 报告 = 变更报告 {
            新增: vec!["鸿蒙/基础设施 - 域/识海承载-府/甲.rs".to_string()],
            修改: vec!["乾坤/呈现-域/命令操作-府/乙.rs".to_string()],
            删除: Vec::new(),
        };
        let (触发, 缘由) = 判定触发(&存储, &报告);
        assert!(触发);
        assert!(缘由.contains("跨"), "应因跨维触发：{缘由}");
    }

    #[test]
    fn 判定触发_教训密集触发() {
        let 存储 = crate::模型存储::打开(
            std::env::temp_dir().join(format!("触发教训-{}", crate::当前毫秒())),
        );
        // 写 5 条教训
        for i in 0..5 {
            存储
                .写记录(&记录::新(教训格位, &format!("教训{i}"), "测试", "LLM"))
                .unwrap();
        }
        // 2 处同府修改（不跨府不跨维不新增），但教训密集 → 触发
        let 报告 = 变更报告 {
            新增: Vec::new(),
            修改: vec![
                "鸿蒙/基础设施 - 域/识海承载-府/甲.rs".to_string(),
                "鸿蒙/基础设施 - 域/识海承载-府/乙.rs".to_string(),
            ],
            删除: Vec::new(),
        };
        let (触发, 缘由) = 判定触发(&存储, &报告);
        assert!(触发);
        assert!(缘由.contains("教训"), "应因教训密集触发：{缘由}");
    }

    #[test]
    fn 判定触发_两处同府无教训不触发() {
        let 存储 = crate::模型存储::打开(
            std::env::temp_dir().join(format!("触发不触发-{}", crate::当前毫秒())),
        );
        // 2 处同府修改，无教训 → 不触发
        let 报告 = 变更报告 {
            新增: Vec::new(),
            修改: vec![
                "鸿蒙/基础设施 - 域/识海承载-府/甲.rs".to_string(),
                "鸿蒙/基础设施 - 域/识海承载-府/乙.rs".to_string(),
            ],
            删除: Vec::new(),
        };
        let (触发, _) = 判定触发(&存储, &报告);
        assert!(!触发);
    }

    #[test]
    fn 府名_提取第三段() {
        assert_eq!(
            府名("鸿蒙/基础设施 - 域/识海承载-府/甲.rs"),
            Some("识海承载-府")
        );
        assert_eq!(府名("甲.rs"), None);
        assert_eq!(府名("鸿蒙/基础设施 - 域"), None);
    }

    #[test]
    fn 维名_提取首段() {
        assert_eq!(维名("鸿蒙/基础设施 - 域/识海承载-府/甲.rs"), Some("鸿蒙"));
        assert_eq!(维名("乾坤/呈现-域/命令操作-府/乙.rs"), Some("乾坤"));
        assert_eq!(维名("甲.rs"), Some("甲.rs"));
        assert_eq!(维名(""), None);
    }
}
