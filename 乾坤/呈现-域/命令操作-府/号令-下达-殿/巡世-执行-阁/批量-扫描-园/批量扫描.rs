//! 巡世执行：天道巡世扫描，产出候选与违逆报告

use crate::工作区根;

pub fn 巡世扫描() -> String {
    let 报告 = tianting_fu::扫描世界(&工作区根());
    format!(
        "巡世扫描完成\nid：{}\n候选：{} 条\n违逆：{} 条",
        报告.id, 报告.候选.len(), 报告.违逆.len()
    )
}

pub fn 巡世报告() -> String {
    let 报告 = tianting_fu::扫描世界(&工作区根());
    let mut 行 = format!("巡世报告\nid：{}\n", 报告.id);
    for 候选 in &报告.候选 {
        行.push_str(&format!("- {}（{}）· {:?}\n", 候选.目标, 候选.依据, 候选.优先级));
    }
    for 违逆 in &报告.违逆 {
        行.push_str(&format!("- 违逆：{} → {}\n", 违逆.路径, 违逆.违逆内容));
    }
    if 报告.候选.is_empty() && 报告.违逆.is_empty() {
        行.push_str("（无候选、无违逆）");
    }
    行
}
