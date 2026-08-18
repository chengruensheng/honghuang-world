//! 波及 - 追溯 - 园：变更文件 → 依赖图反查受影响文件（出边波及 + 入边引用），供机制归纳喂上下文。
//!
//! 出边 = 变更文件内符号声明的波及文件（改动往外流）；入边 = 其它文件里波及列表含变更文件的（改动被谁引用）。

use crate::{依赖图, 变更报告};
use rizhi_fu::info;
use std::collections::HashSet;

/// 波及报告：变更文件（新增+修改）+ 受影响文件（波及们，不含变更自身）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct 波及报告 {
    pub 变更们: Vec<String>,
    pub 波及们: Vec<String>,
}

/// 推演指令（阴·事前推演，§14.18.2）：给执行层的三类边界。
/// - 可以：涉及路径内目标文件 → 可写；
/// - 不能漏：波及/引用/接线 → 改 A 必须检查同步 B；
/// - 不能碰：红线（根级敏感等）→ 只读参考，禁止写。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct 推演指令 {
    pub 可以: Vec<String>,
    pub 不能漏: Vec<String>,
    pub 不能碰: Vec<String>,
}

/// 事前推演：计划改动（涉及路径）→ 三类指令。
/// 复用 推演波及 的出边/入边反查逻辑（图.档案们 的 波及 字段），输入从「已变更文件」改为「计划涉及路径」。
/// - 可以：涉及路径（含目录落点）；
/// - 不能漏：出边波及（涉及文件内符号声明的波及文件）+ 入边引用（波及列表含涉及文件的符号所在文件）− 可以；
/// - 不能碰：红线文件（由调用方按根级敏感规则传入，如 根 Cargo.toml/AGENTS.md/.上下文）。
pub fn 事前推演(
    图: &依赖图, 涉及路径: &[String], 红线文件: &[String]
) -> 推演指令 {
    let 涉及集: HashSet<String> = 涉及路径.iter().map(|p| p.replace('\\', "/")).collect();
    let 红线集: HashSet<String> = 红线文件.iter().map(|p| p.replace('\\', "/")).collect();
    let mut 波及集 = HashSet::new();
    for 档案 in &图.档案们 {
        let 文件 = 档案.文件.replace('\\', "/");
        // 出边：涉及文件内符号声明的波及文件
        if 涉及集.contains(&文件) {
            for 波及 in &档案.波及 {
                波及集.insert(波及.replace('\\', "/"));
            }
        }
        // 入边：其它符号的波及列表含涉及文件 → 该符号所在文件受影响
        if 档案
            .波及
            .iter()
            .any(|波| 涉及集.contains(&波.replace('\\', "/")))
        {
            波及集.insert(文件);
        }
    }
    // 可以 = 涉及路径本身（目标落点，含目录）；剔除红线（红线绝不进可以）。
    let 可以集: HashSet<String> = 涉及集
        .iter()
        .filter(|路径| !红线集.contains(*路径))
        .cloned()
        .collect();
    // 不能漏 = 波及 − 可以 − 红线（改 A 必须检查同步 B；波及中含涉及文件自身的剔除）。
    let 不能漏集: HashSet<String> = 波及集
        .iter()
        .filter(|波及| !可以集.contains(*波及))
        .filter(|波及| !红线集.contains(*波及))
        .cloned()
        .collect();
    let mut 可以: Vec<String> = 可以集.into_iter().collect();
    let mut 不能漏: Vec<String> = 不能漏集.into_iter().collect();
    let mut 不能碰: Vec<String> = 红线集.into_iter().collect();
    可以.sort();
    不能漏.sort();
    不能碰.sort();
    if !不能漏.is_empty() || !不能碰.is_empty() {
        info!(
            可以数 = 可以.len(),
            不能漏数 = 不能漏.len(),
            不能碰数 = 不能碰.len(),
            "事前推演完成"
        );
    }
    推演指令 {
        可以,
        不能漏,
        不能碰,
    }
}

/// 推演波及：给定变更报告与依赖图，回推受影响文件。
/// 删除的文件已不在盘面，波及无从推，故变更们只取 新增 + 修改。
pub fn 推演波及(图: &依赖图, 报告: &变更报告) -> 波及报告 {
    let 变更集: HashSet<String> = 报告
        .新增
        .iter()
        .chain(报告.修改.iter())
        .map(|路径| 路径.replace('\\', "/"))
        .collect();
    let mut 波及集 = HashSet::new();
    for 档案 in &图.档案们 {
        let 文件 = 档案.文件.replace('\\', "/");
        // 出边：变更文件内符号声明的波及文件
        if 变更集.contains(&文件) {
            for 波及 in &档案.波及 {
                波及集.insert(波及.clone());
            }
        }
        // 入边：其它符号的波及列表含此变更文件 → 该符号所在文件受影响
        if 档案
            .波及
            .iter()
            .any(|波| 变更集.contains(&波.replace('\\', "/")))
        {
            波及集.insert(档案.文件.clone());
        }
    }
    // 剔除变更自身，避免自我波及噪声
    波及集.retain(|波| !变更集.contains(&波.replace('\\', "/")));
    let mut 变更们: Vec<String> = 变更集.into_iter().collect();
    let mut 波及们: Vec<String> = 波及集.into_iter().collect();
    变更们.sort();
    波及们.sort();
    if !波及们.is_empty() {
        info!(变更数 = 变更们.len(), 波及数 = 波及们.len(), "波及推演完成");
    }
    波及报告 {
        变更们, 波及们
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::符号档案;

    fn 档案(文件: &str, 波及们: &[&str]) -> 符号档案 {
        let mut 档案 = 符号档案::新("项目", "模块", 文件, "符号", "代码", "签名", "解释");
        档案.波及 = 波及们.iter().map(|波| 波.to_string()).collect();
        档案
    }

    #[test]
    fn 推演波及_出边与入边都收集且剔除自身() {
        let 图 = 依赖图 {
            档案们: vec![
                档案("甲.rs", &["乙.rs"]), // 甲 波及 乙（出边）
                档案("丙.rs", &["甲.rs"]), // 丙 的波及含甲 → 丙受影响（入边）
                档案("丁.rs", &[]),        // 无关
            ],
            ..Default::default()
        };
        let 报告 = 变更报告 {
            新增: vec!["甲.rs".to_string()],
            修改: Vec::new(),
            删除: Vec::new(),
        };
        let 结果 = 推演波及(&图, &报告);
        assert_eq!(结果.变更们, vec!["甲.rs"]);
        assert_eq!(结果.波及们, vec!["丙.rs", "乙.rs"]);
    }

    #[test]
    fn 推演波及_无波及文件时为空() {
        let 图 = 依赖图 {
            档案们: vec![档案("甲.rs", &[])],
            ..Default::default()
        };
        let 报告 = 变更报告 {
            新增: vec!["甲.rs".to_string()],
            修改: Vec::new(),
            删除: Vec::new(),
        };
        let 结果 = 推演波及(&图, &报告);
        assert!(结果.波及们.is_empty());
    }

    /// §14.18.2：事前推演——计划改甲（波及乙、被丙引用）→ 可以=[甲] 不能漏=[乙,丙] 不能碰=[红线]。
    #[test]
    fn 事前推演_三指令齐全() {
        let 图 = 依赖图 {
            档案们: vec![
                档案("甲.rs", &["乙.rs"]), // 甲 出边波及 乙
                档案("丙.rs", &["甲.rs"]), // 丙 入边引用 甲
                档案("丁.rs", &[]),        // 无关
            ],
            ..Default::default()
        };
        let 指令 = 事前推演(&图, &["甲.rs".to_string()], &["Cargo.toml".to_string()]);
        assert_eq!(指令.可以, vec!["甲.rs"]);
        assert!(
            指令.不能漏.iter().any(|p| p == "乙.rs"),
            "应含出边波及乙：{:?}",
            指令.不能漏
        );
        assert!(
            指令.不能漏.iter().any(|p| p == "丙.rs"),
            "应含入边引用丙：{:?}",
            指令.不能漏
        );
        assert!(
            !指令.不能漏.iter().any(|p| p == "甲.rs"),
            "涉及自身不入不能漏"
        );
        assert_eq!(指令.不能碰, vec!["Cargo.toml"]);
    }

    /// §14.18.2：涉及路径为空（审验类）→ 可以/不能漏空，不能碰=红线（不误推）。
    #[test]
    fn 事前推演_涉及路径空不误推() {
        let 图 = 依赖图 {
            档案们: vec![],
            ..Default::default()
        };
        let 指令 = 事前推演(&图, &[], &["Cargo.toml".to_string()]);
        assert!(指令.可以.is_empty());
        assert!(
            指令.不能漏.is_empty(),
            "无涉及路径不应有波及：{:?}",
            指令.不能漏
        );
        assert_eq!(指令.不能碰, vec!["Cargo.toml"]);
    }

    /// §14.18.2：涉及路径含红线 → 红线归不能碰，绝不进可以。
    #[test]
    fn 事前推演_红线不进可以() {
        let 图 = 依赖图 {
            档案们: vec![],
            ..Default::default()
        };
        let 指令 = 事前推演(
            &图,
            &["Cargo.toml".to_string()],
            &["Cargo.toml".to_string()],
        );
        assert!(指令.可以.is_empty(), "红线不得进可以：{:?}", 指令.可以);
        assert_eq!(指令.不能碰, vec!["Cargo.toml"]);
    }
}
