use crate::任务模型_殿::{漂移类型, 漂移项, 漂移报告};

/// 系统/构建目录前缀（快照中剔除，不算越界新增）
const 忽略前缀: [&str; 2] = [".git/", "target/"];

/// 归一化相对路径：统一 `/` 分隔、去掉首部 `./`、去空
fn 归一化(路径: &str) -> String {
    路径.replace('\\', "/").trim_start_matches("./").to_string()
}

/// 解析执行器 `按名找文件` 快照文本 → 相对路径列表。
/// 空/`（无匹配）` 视为空集。
pub fn 解析快照(快照: &str) -> Vec<String> {
    let mut 路径们: Vec<String> = 快照
        .lines()
        .map(|行| 行.trim())
        .filter(|行| !行.is_empty() && *行 != "（无匹配）")
        .map(归一化)
        .filter(|p| 非忽略(p))
        .collect();
    路径们.sort();
    路径们.dedup();
    路径们
}

fn 非忽略(路径: &str) -> bool {
    !忽略前缀.iter().any(|前缀| 路径.starts_with(前缀))
}

/// 漂移检测：声明文件集 × 实际文件集，产出三类核对结果。
/// 纯函数（无 IO、无锁），供扫尾检查器与执行者复用。
pub fn 漂移检测(声明: &[String], 实际: &[String]) -> 漂移报告 {
    let 声明集: Vec<String> = 归一化去重(声明);
    let 实际集: Vec<String> = 归一化去重(实际);

    let mut 漂移项 = Vec::new();
    let mut 兑现数 = 0;
    for 路径 in &声明集 {
        if 实际集.contains(路径) {
            兑现数 += 1;
        } else {
            漂移项.push(漂移项 {
                类型: 漂移类型::声明未兑现,
                路径: 路径.clone(),
                说明: "实现文档声明了该文件，但工作区未落地".into(),
            });
        }
    }
    for 路径 in &实际集 {
        if !声明集.contains(路径) {
            漂移项.push(漂移项 {
                类型: 漂移类型::未声明新增,
                路径: 路径.clone(),
                说明: "工作区存在该文件，但实现文档未声明".into(),
            });
        }
    }
    let 未兑现数 = 声明集.len() - 兑现数;
    // 实际集恰好含兑现的声明文件 + 多余新增；多余数 = 实际数 - 兑现数
    let 多余数 = 实际集.len().saturating_sub(兑现数);

    漂移报告 {
        声明数: 声明集.len(),
        实际数: 实际集.len(),
        兑现数,
        未兑现数,
        多余数,
        漂移项,
    }
}

fn 归一化去重(输入: &[String]) -> Vec<String> {
    let mut 结果: Vec<String> = 输入
        .iter()
        .map(|s| 归一化(s))
        .filter(|s| !s.is_empty() && 非忽略(s))
        .collect();
    结果.sort();
    结果.dedup();
    结果
}