//! 契约校验：插件清单与产出的 schema、协议版本校验
//!
//! 设计依据：`.传承/设计/落地设计/语言解析插件-落地设计.md` §五（降级矩阵）。
//! 原则：任何单点失败都不得使整张图谱失败。

use crate::{产出, 悬空, 符号种类, 边种类, 插件清单, 当前协议版本};
use std::collections::BTreeSet;

/// 校验插件清单是否可加载；`Err` 表示应拒绝该插件
pub fn 校验清单(清单: &插件清单) -> Result<(), String> {
    if 清单.协议版本 != 当前协议版本 {
        return Err(format!(
            "协议版本不匹配：清单为 {}，引擎为 {}",
            清单.协议版本, 当前协议版本
        ));
    }
    if 清单.语言.trim().is_empty() {
        return Err("语言字段为空".to_string());
    }
    if 清单.文件后缀.is_empty() {
        return Err("文件后缀为空".to_string());
    }
    if 清单.入口.is_empty() {
        return Err("入口为空".to_string());
    }
    Ok(())
}

/// 校验插件产出的硬性约束；`Err` 表示应整包丢弃该产出
pub fn 校验产出(包: &产出, 期望语言: &str) -> Result<(), String> {
    if 包.协议版本 != 当前协议版本 {
        return Err(format!(
            "产出协议版本不匹配：{} / {}",
            包.协议版本, 当前协议版本
        ));
    }
    if 包.语言 != 期望语言 {
        return Err(format!("产出语言不符：期望 {}，实为 {}", 期望语言, 包.语言));
    }
    for 符 in &包.符号 {
        if 符.id.trim().is_empty() {
            return Err(format!("符号「{}」的 id 为空", 符.名));
        }
        if 符.文件.trim().is_empty() {
            return Err(format!("符号「{}」的文件为空", 符.名));
        }
    }
    Ok(())
}

/// 归一化去重的实绩，供引擎写进插件状态
pub struct 去重实绩 {
    /// 同一 (ID, 种类) 重复出现、被丢弃的符号数
    pub 符号重复: usize,
    /// 同一 (从, 到, 类型) 重复出现、被丢弃的边数
    pub 边重复: usize,
    /// 同 ID 但不同种类的符号对数（合法，保留）
    pub 同名异种类: usize,
    /// 同 ID 但不同种类的样例子，便于人工核对
    pub 同名样例: Vec<String>,
}

/// 归一化：同一插件内部可能把同一个逻辑节点产两遍，同一对端点也可能连两条同类边
///
/// 已知来源：`模块.rs` 里的 `mod X;` 声明与 `X.rs` 的文件级模块符号会算出同一个 ID
/// ——二者本就是同一个逻辑模块被声明了两遍。不去重会让入度统计把同一节点算两次，
/// 直接失真"死代码"判定。
///
/// **唯一键取 (ID, 种类) 而非单独的 ID**：语言的类型命名空间与值命名空间允许同名
/// （`struct 任务快照` 与 `fn 任务快照`），那是两个符号，不能合并，只记录不丢弃。
pub fn 去重产出(包: &mut 产出) -> 去重实绩 {
    let 原符号 = std::mem::take(&mut 包.符号);
    let mut 已见: BTreeSet<(String, 符号种类)> = BTreeSet::new();
    let mut 单独id: BTreeSet<String> = BTreeSet::new();
    let mut 保留 = Vec::with_capacity(原符号.len());
    let mut 符号重复 = 0usize;
    let mut 同名异种类 = 0usize;
    let mut 同名样例 = Vec::new();

    for 符 in 原符号 {
        if !已见.insert((符.id.clone(), 符.种类)) {
            符号重复 += 1;
            continue;
        }
        if !单独id.insert(符.id.clone()) {
            同名异种类 += 1;
            if 同名样例.len() < 5 {
                同名样例.push(符.id.clone());
            }
        }
        保留.push(符);
    }

    let 原边 = std::mem::take(&mut 包.边);
    let mut 已见边: BTreeSet<(String, String, 边种类)> = BTreeSet::new();
    let mut 留边 = Vec::with_capacity(原边.len());
    let mut 边重复 = 0usize;

    for 边 in 原边 {
        if !已见边.insert((边.从.clone(), 边.到.clone(), 边.类型)) {
            边重复 += 1;
            continue;
        }
        留边.push(边);
    }

    包.符号 = 保留;
    包.边 = 留边;
    去重实绩 {
        符号重复,
        边重复,
        同名异种类,
        同名样例,
    }
}

/// 把引用了不存在符号的边降级为悬空（软性问题，不整包丢弃）
///
/// 返回被降级的边数。降级而非丢弃，是因为「指向不存在目标的引用」
/// 恰恰是幽灵引用诊断的原始素材。
pub fn 降级悬空边(包: &mut 产出) -> usize {
    let 已知: std::collections::BTreeSet<String> =
        包.符号.iter().map(|s| s.id.clone()).collect();

    let 原边 = std::mem::take(&mut 包.边);
    let mut 保留 = Vec::with_capacity(原边.len());
    let mut 降级 = Vec::new();

    for 边 in 原边 {
        if 已知.contains(&边.从) && 已知.contains(&边.到) {
            保留.push(边);
        }
        else {
            降级.push(悬空 {
                从: 边.从,
                目标文本: 边.到,
                类型: 边.类型,
                置信度: 边.置信度,
            });
        }
    }

    let 数 = 降级.len();
    包.边 = 保留;
    包.悬空.extend(降级);
    数
}
