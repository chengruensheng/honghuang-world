//! 机制 - 提炼 - 园：复杂变更（≥3 文件）→ 汇变更与波及文件的函数切片 → 唤 LLM 归纳机制 → 理解·记忆格位。
//!
//! 定位：地道在执行期做增量收尾——小变更只记痕迹、零 LLM 成本；
//! 复杂变更白嫖任务生成等待期唤 LLM，把耦合文件共同支撑的机制沉淀下来。

use crate::推演波及;
use crate::{依赖图, 变更报告, 模型存储, 记录};
use jiance_fu::{观测角色, 进入观测};
use moxing_fu::{对话消息, 模型配置, 精简上限, 调用模型};
use rizhi_fu::info;
use std::path::Path;

/// 复杂阈值：变更文件数达到此值才唤起 LLM 机制归纳（对齐地道原型·复杂阈值）。
pub const 复杂阈值: usize = 3;

/// 机制归纳：变更 ≥ 复杂阈值 → 汇变更+波及文件切片 → 唤 LLM 提炼机制 → 写「理解·记忆」格位。
/// 返回 Some(归纳文本) 表示已归纳；None 表示未触发（变更数不足 / 素材为空）。
pub fn 机制归纳(
    存储: &模型存储,
    配置: &模型配置,
    图: &依赖图,
    根: &Path,
    报告: &变更报告,
) -> Result<Option<String>, String> {
    // 白箱观测：地道归纳进入归因角色（世界级记忆维护，无要求关联）。
    let _观测守卫 = 进入观测(观测角色::归因, None, None, None);
    if 报告.总处数() < 复杂阈值 {
        return Ok(None);
    }
    let 波及 = 推演波及(图, 报告);
    let 素材 = 汇切片(图, 根, &波及.变更们, &波及.波及们);
    if 素材.trim().is_empty() {
        return Ok(None);
    }
    let 提示 = format!(
        "你是一名项目记忆采集员。以下是一批有耦合关系的文件（变更文件与受波及文件），共同支撑一个机制。\
         请提炼这个机制：它们协同完成什么、边界在哪、改动时要注意什么。精炼成一条 200 字以内的记录，直接给结论。\n\n【素材】\n{素材}"
    );
    let (回复, 用量) = 调用模型(配置, &[对话消息::用户(提示)], 精简上限)?;
    存储.写记录(&记录::新(
        "理解·记忆",
        &回复,
        &format!("地道机制归纳：{} 处变更", 报告.总处数()),
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
}
