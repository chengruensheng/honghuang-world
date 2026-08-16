//! 波及 - 追溯 - 园：变更文件 → 依赖图反查受影响文件（出边波及 + 入边引用），供机制归纳喂上下文。
//!
//! 出边 = 变更文件内符号声明的波及文件（改动往外流）；入边 = 其它文件里波及列表含变更文件的（改动被谁引用）。

use crate::{变更报告, 依赖图};
use rizhi_fu::info;
use std::collections::HashSet;

/// 波及报告：变更文件（新增+修改）+ 受影响文件（波及们，不含变更自身）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct 波及报告 {
    pub 变更们: Vec<String>,
    pub 波及们: Vec<String>,
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
        if 档案.波及.iter().any(|波| 变更集.contains(&波.replace('\\', "/"))) {
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
    波及报告 { 变更们, 波及们 }
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
                档案("甲.rs", &["乙.rs"]),   // 甲 波及 乙（出边）
                档案("丙.rs", &["甲.rs"]),   // 丙 的波及含甲 → 丙受影响（入边）
                档案("丁.rs", &[]),           // 无关
            ],
            ..Default::default()
        };
        let 报告 = 变更报告 { 新增: vec!["甲.rs".to_string()], 修改: Vec::new(), 删除: Vec::new() };
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
        let 报告 = 变更报告 { 新增: vec!["甲.rs".to_string()], 修改: Vec::new(), 删除: Vec::new() };
        let 结果 = 推演波及(&图, &报告);
        assert!(结果.波及们.is_empty());
    }
}
