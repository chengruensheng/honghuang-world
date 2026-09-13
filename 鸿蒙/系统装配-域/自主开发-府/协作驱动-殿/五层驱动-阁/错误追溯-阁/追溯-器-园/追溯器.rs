use uuid::Uuid;
use tc_task::{DesignDoc, ImplementationDoc, 五行层级};
use crate::协作驱动_殿::五层驱动_阁::错误追溯_阁::{错误信息包, 错误类型};

/// 追溯器：验收不通过时的错误根源判定（纯规则，不用 LLM）。
///
/// 判定顺序（决定性）：
/// 1. 设计依赖循环 / 契约空壳 → 设计缺陷（火）
/// 2. 实现产物越界设计文件清单 / 实现与设计不一致 → 实现 Bug（土）
/// 3. 需求关键词未进设计边界 → 需求偏差（木）
/// 4. 错误现象关键词（需求/澄清/范围）→ 需求偏差（木）
/// 5. 兜底：未归类 → 环境问题（土，执行者处理）
pub struct 追溯器 {
    /// 设计文件清单对比失败判定的产物关键字（命中即视为实现 Bug 证据）
    _标记: (),
}

impl Default for 追溯器 {
    fn default() -> Self {
        追溯器::新()
    }
}

impl 追溯器 {
    pub fn 新() -> Self {
        追溯器 { _标记: () }
    }

    /// 追溯：给定错误现象与阶段文档，判定根源层级与错误类型
    pub fn 追溯(
        &self,
        出错任务id: Uuid,
        错误现象: &str,
        设计: Option<&DesignDoc>,
        实现: Option<&ImplementationDoc>,
        需求描述: &str,
    ) -> 错误信息包 {
        // 规则1：设计依赖循环 / 契约空壳 → 设计缺陷
        if let Some(设计) = 设计 {
            if let Some(环) = 检测循环依赖(设计) {
                return 错误信息包::新(
                    出错任务id,
                    五行层级::火,
                    错误类型::设计缺陷,
                    format!("设计依赖存在循环：{}", 环.join(" → ")),
                    设计.修改文件.clone(),
                );
            }
            if 设计.契约.iter().any(|c| c.契约名.is_empty() || c.方法.iter().any(|m| m.签名.is_empty())) {
                return 错误信息包::新(
                    出错任务id,
                    五行层级::火,
                    错误类型::设计缺陷,
                    "设计契约存在空契约名或空签名（接口定义不完整）".to_string(),
                    设计.修改文件.clone(),
                );
            }
        }

        // 规则2：实现与设计不一致（文件越界/设计文件清单未被执行） → 实现 Bug
        let 实现一致 = 实现.and_then(|实现| 设计.map(|设计| 判断实现与设计一致性(实现, 设计)));
        if 实现一致 == Some(false) {
            let 产物 = 实现
                .map(|d| d.代码变更.iter().map(|c| c.文件路径.clone()).collect())
                .unwrap_or_default();
            return 错误信息包::新(
                出错任务id,
                五行层级::土,
                错误类型::实现Bug,
                "实现产物与设计方案不一致（变更文件越界或未按设计文件清单实现）".to_string(),
                产物,
            );
        }

        // 规则3：需求关键词未进入设计边界/文件清单 → 需求偏差。
        // 仅当错误现象本身指向需求偏差（需求/澄清/范围字样）时才做词面覆盖检查：
        // 编译/测试失败等机器核验现象必须落到规则5带真实证据回退，禁止用词面启发式抢判
        // （2026-09-13 实证：核验命令失败被乱码伪要点误判为需求偏差，定向回退木层后卡死澄清）。
        if let Some(设计) = 设计 {
            if 含需求字样(错误现象) {
                let 设计文本 = 设计.边界定义.values().cloned().collect::<Vec<_>>().join(" ");
                let 需求关键词 = 提取需求关键词(需求描述);
                let 缺失: Vec<String> = 需求关键词
                    .iter()
                    .filter(|词| !设计文本.contains(*词) && !设计.新建文件.iter().any(|f| f.contains(*词)))
                    .map(|s| s.to_string())
                    .collect();
                if !缺失.is_empty() && !设计.边界定义.is_empty() {
                    return 错误信息包::新(
                        出错任务id,
                        五行层级::木,
                        错误类型::需求偏差,
                        format!("设计未覆盖需求要点：{}", 缺失.join("、")),
                        Vec::new(),
                    );
                }
            }
        }

        // 规则4：错误现象关键词命中需求相关 → 需求偏差
        let 现象 = 错误现象.to_string();
        if 含需求字样(&现象) {
            return 错误信息包::新(
                出错任务id,
                五行层级::木,
                错误类型::需求偏差,
                format!("错误现象指向需求偏差：{现象}"),
                Vec::new(),
            );
        }

        // 规则5：兜底 → 未归类（环境问题，执行者处理）
        错误信息包::新(
            出错任务id,
            五行层级::土,
            错误类型::环境问题,
            format!("未能归类错误（{现象}），交由执行者排查环境/实现"),
            Vec::new(),
        )
    }

    /// 判断实现产物与设计方案是否一致：实现变更文件应落在设计文件清单内
    pub fn 判断实现与设计一致性(&self, 实现: &ImplementationDoc, 设计: &DesignDoc) -> bool {
        判断实现与设计一致性(实现, 设计)
    }
}

/// 实现与设计一致性：实现代码变更的每个文件路径都在设计「修改文件/新建文件」清单内
fn 判断实现与设计一致性(实现: &ImplementationDoc, 设计: &DesignDoc) -> bool {
    if 实现.代码变更.is_empty() {
        return false; // 实现层没产出任何变更
    }
    let 设计清单: Vec<String> = 设计.修改文件.iter().chain(设计.新建文件.iter()).cloned().collect();
    if 设计清单.is_empty() {
        return false; // 设计未给出文件清单 → 视为未按设计实现（防空洞实现）
    }
    实现.代码变更.iter().all(|c| 设计清单.contains(&c.文件路径))
}

/// 设计依赖 DFS 循环检测，返回任一循环路径
fn 检测循环依赖(设计: &DesignDoc) -> Option<Vec<String>> {
    let 节点: Vec<String> = 设计
        .依赖
        .iter()
        .flat_map(|d| vec![d.来源模块.clone(), d.目标模块.clone()])
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    if 节点.is_empty() {
        return None;
    }
    let mut 邻接: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for 依赖 in &设计.依赖 {
        邻接.entry(依赖.来源模块.as_str()).or_default().push(依赖.目标模块.as_str());
    }
    fn dfs<'a>(
        当前: &'a str,
        邻接: &std::collections::HashMap<&'a str, Vec<&'a str>>,
        路径: &mut Vec<String>,
        在栈: &mut std::collections::HashSet<String>,
    ) -> Option<Vec<String>> {
        if !在栈.insert(当前.to_string()) {
            let 起点 = 路径.iter().position(|n| n == 当前)?;
            let mut 环 = 路径[起点..].to_vec();
            环.push(当前.to_string());
            return Some(环);
        }
        路径.push(当前.to_string());
        if let Some(下一批) = 邻接.get(当前) {
            for 下一 in 下一批 {
                if let Some(环) = dfs(下一, 邻接, 路径, 在栈) {
                    return Some(环);
                }
            }
        }
        路径.pop();
        在栈.remove(当前);
        None
    }
    let mut 路径 = Vec::new();
    let mut 在栈 = std::collections::HashSet::new();
    for 起点 in &节点 {
        在栈.clear();
        路径.clear();
        if let Some(环) = dfs(起点, &邻接, &mut 路径, &mut 在栈) {
            return Some(环);
        }
    }
    None
}

/// 从需求描述提取关键词：连续拉丁段保留整词（≥2 字符），连续汉字段段内做 2 字滑窗。
/// 语种分段是为了不再跨中英边界切出「用R/Ru/t新」类碎片（2026-09-13 实证缺陷）；
/// 标点、空白与符号只作分段边界，不产出词条。
fn 提取需求关键词(需求: &str) -> Vec<String> {
    let mut 词: Vec<String> = Vec::new();
    let mut 拉丁 = String::new();
    let mut 汉字: Vec<char> = Vec::new();
    for 字 in 需求.chars() {
        if 字.is_ascii_alphanumeric() {
            if !汉字.is_empty() {
                收汉字窗(&汉字, &mut 词);
                汉字.clear();
            }
            拉丁.push(字);
        } else if 字.is_alphabetic() {
            // 汉字等非 ASCII 字母：进入汉字段
            if !拉丁.is_empty() {
                收拉丁词(&mut 拉丁, &mut 词);
            }
            汉字.push(字);
        } else {
            // 空白/标点/符号：分段边界
            if !拉丁.is_empty() {
                收拉丁词(&mut 拉丁, &mut 词);
            }
            if !汉字.is_empty() {
                收汉字窗(&汉字, &mut 词);
                汉字.clear();
            }
        }
    }
    if !拉丁.is_empty() {
        收拉丁词(&mut 拉丁, &mut 词);
    }
    if !汉字.is_empty() {
        收汉字窗(&汉字, &mut 词);
    }
    词.dedup();
    词.into_iter().take(8).collect()
}

/// 拉丁段冲刷：整词保留（≥2 字符），单字符丢弃
fn 收拉丁词(拉丁: &mut String, 词: &mut Vec<String>) {
    if 拉丁.chars().count() >= 2 {
        词.push(std::mem::take(拉丁));
    } else {
        拉丁.clear();
    }
}

/// 汉字段冲刷：段内 2 字滑窗
fn 收汉字窗(汉字: &[char], 词: &mut Vec<String>) {
    if 汉字.len() >= 2 {
        for w in 汉字.windows(2) {
            词.push(w.iter().collect());
        }
    }
}

/// 错误现象是否指向需求偏差（需求/澄清/范围/不是想要等字样）
fn 含需求字样(现象: &str) -> bool {
    const 字样: [&str; 6] = ["需求", "澄清", "范围外", "不是想要的", "不符合需求", "想要"];
    字样.iter().any(|w| 现象.contains(w))
}
