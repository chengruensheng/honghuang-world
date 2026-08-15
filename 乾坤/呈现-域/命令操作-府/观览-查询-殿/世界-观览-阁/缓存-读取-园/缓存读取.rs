//! 世界观览：真实世界状态（巡世扫描快照）

use crate::工作区根;

pub fn 呈现世界状态() -> String {
    let 根 = 工作区根();
    let 报告 = tianting_fu::扫描世界(&根);
    format!(
        "世界状态\n工作区：{}\n候选改进：{} 条\n违逆：{} 条",
        根.display(),
        报告.候选.len(),
        报告.违逆.len()
    )
}

pub fn 呈现队列水位() -> String {
    "队列水位\n在途要求：0（队列调度殿已建，端到端全流程由「想法 投递」直接驱动）".to_string()
}

pub fn 呈现版本历史() -> String {
    let 根 = 工作区根();
    let 版本库 = 根.join(".上下文").join("版本库");
    if !版本库.exists() {
        return "版本历史\n（暂无，用「版本 存档」创建）".to_string();
    }
    let 内容 = 版本库.display().to_string();
    format!("版本历史\n版本库：{内容}")
}

pub fn 版本详情(版本号: &str) -> String {
    let 根 = 工作区根();
    let 快照 = 根.join(".上下文").join("版本库").join(format!("版本-{版本号}")).join("源码-快照");
    if 快照.exists() {
        format!("版本 {版本号}\n快照路径：{}", 快照.display())
    } else {
        format!("版本 {版本号} 快照不存在")
    }
}
