//! 依赖 - 图 - 类型：符号档案、依赖图、结构树（扫描配套）。

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 符号档案：单个符号（函数 / 类型）的微观语义。
/// 五层标识（项目 / 模块 / 文件 / 符号 / 代码）+ 解释 + 波及。
/// 代码 = 完整定义体（M4 实测：执行层只喂命中函数体，质量最高且省 token）；
/// 签名 = 单行签名（补解释等摘要场景用，避免把整个函数体塞进提示）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 符号档案 {
    pub 项目: String,
    pub 模块: String,
    pub 文件: String,
    pub 符号: String,
    pub 代码: String,
    pub 签名: String,
    pub 解释: String,
    pub 波及: Vec<String>,
}

impl 符号档案 {
    /// 构造符号档案（波及初始为空，扫引用后回填）。
    pub fn 新(项目: &str, 模块: &str, 文件: &str, 符号: &str, 代码: &str, 签名: &str, 解释: &str) -> 符号档案 {
        符号档案 {
            项目: 项目.to_string(),
            模块: 模块.to_string(),
            文件: 文件.to_string(),
            符号: 符号.to_string(),
            代码: 代码.to_string(),
            签名: 签名.to_string(),
            解释: 解释.to_string(),
            波及: Vec::new(),
        }
    }
}

/// 依赖图：全项目符号档案集合，落盘 json。
/// 支撑双向导航：64→512 按模块/文件定位，512→64 读波及追溯。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 依赖图 {
    pub 档案们: Vec<符号档案>,
    /// 结构树：完整目录层级（根为 crate 完整相对路径，向下为 crate 内 殿/阁/园），支撑按需下探。
    /// 与符号档案同源落盘；执行层按涉及路径下探，把殿/阁/园喂给执行者。
    #[serde(default)]
    pub 结构树: 结构节点,
}

impl 依赖图 {
    /// 命令接线文件清单：新增 CLI 命令类任务（方向含「命令」）的联动文件，
    /// 提审补联动路径与 查相关文件 兜底共用，避免两处清单漂移。
    pub const 命令接线文件: [&str; 3] = [
        "乾坤/呈现-域/命令操作-府/命令-解析-殿/命令-分发-阁/兜底-分发-园/兜底分发.rs",
        "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/缓存-读取-园/缓存读取.rs",
        "乾坤/呈现-域/命令操作-府/命令-解析-殿/命令-入口-阁/兜底-入口-园/入口执行.rs",
    ];

    /// 按符号名查档案（512→64 追溯）。
    pub fn 查符号(&self, 符号名: &str) -> Vec<&符号档案> {
        self.档案们.iter().filter(|档案| 档案.符号 == 符号名).collect()
    }

    /// 按文件查档案（64→512 定位）。
    pub fn 查文件(&self, 文件: &str) -> Vec<&符号档案> {
        self.档案们.iter().filter(|档案| 档案.文件 == 文件).collect()
    }

    /// 按模块（府 / 域）查档案。
    pub fn 查模块(&self, 模块: &str) -> Vec<&符号档案> {
        self.档案们.iter().filter(|档案| 档案.模块 == 模块).collect()
    }

    /// 按涉及路径（符号名 / 文件路径片段）查相关文件集合（含波及文件）。
    /// 符号名支持子串匹配；供执行层精确读现状：涉及什么 → 波及哪些文件 → 只读这些。
    pub fn 查涉及文件(&self, 涉及路径们: &[String]) -> Vec<String> {
        let mut 文件集 = HashSet::new();
        for 涉及 in 涉及路径们 {
            // 统一分隔符：依赖图路径存反斜杠，LLM 填的涉及路径常用正斜杠，不一致会漏匹配。
            let 涉及 = 涉及.trim().replace('\\', "/");
            if 涉及.is_empty() {
                continue;
            }
            for 档案 in &self.档案们 {
                let 文件 = 档案.文件.replace('\\', "/");
                if 档案.符号 == 涉及 || 档案.符号.contains(&涉及) || 文件.contains(&涉及) {
                    文件集.insert(档案.文件.clone());
                    for 波及 in &档案.波及 {
                        文件集.insert(波及.clone());
                    }
                }
            }
        }
        let mut 结果: Vec<String> = 文件集.into_iter().collect();
        结果.sort();
        结果
    }

    /// 执行层读现状：涉及路径 → 精确相关文件；匹配为空时兜底命令分发接线文件，
    /// 保证「新增 CLI 命令」类任务至少能看到命令表与分发现状（防假成功形态二）。
    pub fn 查相关文件(&self, 涉及路径们: &[String]) -> Vec<String> {
        let 精确 = self.查涉及文件(涉及路径们);
        if !精确.is_empty() {
            return 精确;
        }
        // 只在档案确有此文件时兜底，硬编码路径失效则自然退回空。
        Self::命令接线文件
            .iter()
            .filter(|文件| self.档案们.iter().any(|档案| 档案.文件.replace('\\', "/") == **文件))
            .map(|文件| 文件.to_string())
            .collect()
    }

    /// 按阁补全：把给定文件各自所在「阁」目录下的全部源文件一并纳入（含兄弟园），
    /// 保证目标文件所在阁对执行者完整可见（函数级切片照常应用，不读全文）。
    pub fn 补全同阁(&self, 文件们: &[String]) -> Vec<String> {
        let mut 文件集: HashSet<String> = 文件们.iter().cloned().collect();
        for 文件 in 文件们 {
            let Some(阁) = 提取阁目录(文件) else { continue };
            for 档案 in &self.档案们 {
                if 档案.文件.replace('\\', "/").starts_with(&阁) {
                    文件集.insert(档案.文件.clone());
                }
            }
        }
        let mut 结果: Vec<String> = 文件集.into_iter().collect();
        结果.sort();
        结果
    }

    /// 结构下探：按关键词匹配结构树（根为 crate 完整相对路径，向下为 crate 内目录）。
    /// 关键词命中树内任意节点名即输出该 crate 子树；无命中兜底渲染全部 crate 树。
    pub fn 下探(&self, 关键词们: &[String]) -> String {
        let 命中 = |根: &结构节点| {
            关键词们.is_empty() || 关键词们.iter().any(|关键词| 树含关键词(根, 关键词))
        };
        let 兜底 = !关键词们.is_empty() && !self.结构树.子节点.iter().any(|根| 命中(根));
        let mut 输出 = String::new();
        for 根 in &self.结构树.子节点 {
            if 兜底 || 命中(根) {
                输出.push_str(&根.名字);
                输出.push('\n');
                for 子 in &根.子节点 {
                    渲染子树(子, 1, &mut 输出);
                }
            }
        }
        输出.trim_end().to_string()
    }

    /// 图谱契约·库根导出（设计稿 §4.2 规则6）：按涉及文件追溯所属府（模块名 = 府名，
    /// 入口.rs 的 pub use 全量重导出该府顶层符号），投影该府可导入符号签清单——
    /// 让实现层知道 打开存储/状态目录/工作区根 等库根函数在哪、签名什么样，不再凭记忆猜。
    /// 返回「府名 → 签名清单」文本；无匹配返回空串（零开销）。
    pub fn 查库根导出(&self, 涉及路径们: &[String]) -> String {
        let mut 府名集合 = std::collections::BTreeSet::new();
        for 涉及 in 涉及路径们 {
            let 涉及 = 涉及.trim().replace('\\', "/");
            if 涉及.is_empty() {
                continue;
            }
            for 档案 in &self.档案们 {
                let 文件 = 档案.文件.replace('\\', "/");
                if 文件.contains(&涉及) {
                    if let Some(府名) = 档案.模块.split('/').last() {
                        if !府名.is_empty() {
                            府名集合.insert(府名.to_string());
                        }
                    }
                }
            }
        }
        // 精确到文件却无归属府（涉及路径是目录）→ 退化按目录名匹配模块。
        if 府名集合.is_empty() {
            for 涉及 in 涉及路径们 {
                let 涉及 = 涉及.trim();
                for 档案 in &self.档案们 {
                    if 档案.模块.replace('\\', "/").ends_with(涉及) {
                        if let Some(府名) = 档案.模块.split('/').last() {
                            府名集合.insert(府名.to_string());
                        }
                    }
                }
            }
        }
        let mut 输出 = String::new();
        for 府名 in &府名集合 {
            let 符号们 = self.查模块(府名);
            if 符号们.is_empty() {
                continue;
            }
            let 签名们: Vec<&str> = 符号们
                .iter()
                .filter(|档案| !档案.签名.is_empty())
                .map(|档案| 档案.签名.as_str())
                .collect();
            if 签名们.is_empty() {
                continue;
            }
            输出.push_str(&format!("【{府名}·库根导出】\n{}\n", 签名们.join("\n")));
        }
        输出
    }

    /// 图谱契约·测试样例参照（设计稿 §4.2 规则6）：依赖图中找含 `#[cfg(test)]` /
    /// `mod 测试` 的符号档案，优先取与涉及路径同府的文件，投影其中一个完整测试模块的
    /// 签名+文件路径——实现层照抄该项目既有测试惯例（模块三件套、#[cfg(test)] 写法、
    /// env 互斥锁），不再凭空发明测试结构。
    /// 返回「样本清单 + 一个完整样例文件路径」文本；无样例返回空串。
    pub fn 查测试样例(&self, 涉及路径们: &[String]) -> String {
        // 同府优先：涉及路径文件所属府的测试档案。
        let 涉及府们: std::collections::HashSet<String> = {
            let mut 集合 = std::collections::HashSet::new();
            for 涉及 in 涉及路径们 {
                let 涉及 = 涉及.trim().replace('\\', "/");
                for 档案 in &self.档案们 {
                    if 档案.文件.replace('\\', "/").contains(&涉及) {
                        if let Some(府名) = 档案.模块.split('/').last() {
                            集合.insert(府名.to_string());
                        }
                    }
                }
            }
            集合
        };
        let 测试档案们: Vec<&符号档案> = self
            .档案们
            .iter()
            .filter(|档案| 档案.代码.contains("#[cfg(test)]") || 档案.代码.contains("mod 测试") || 档案.代码.contains("mod tests"))
            .collect();
        if 测试档案们.is_empty() {
            return String::new();
        }
        let 同府: Vec<&符号档案> = 测试档案们
            .iter()
            .copied()
            .filter(|档案| {
                档案.模块.split('/').last().map(|府| 涉及府们.contains(府)).unwrap_or(false)
            })
            .collect();
        let 样例 = if let Some(首) = 同府.first() {
            *首
        } else {
            *测试档案们.first().unwrap()
        };
        let 签名 = if 样例.签名.is_empty() { 样例.符号.clone() } else { 样例.签名.clone() };
        let 摘要: String = 样例.代码.lines().take(5).collect::<Vec<_>>().join("\n");
        format!(
            "【测试样例参照】项目既有测试写法样例：{}\n文件路径：{}\n签名：{}\n片段：\n{}\n照抄此处的模块三件套 与 #[cfg(test)]/mod 测试 惯例，勿自造测试结构。\n\n\
             【补测试落位判据】① 涉及路径内已有被测 .rs → 测试直接内联写在该文件尾 #[cfg(test)]，禁止新建独立测试文件；② 勿往空目录/非涉及路径写测试（会被护栏拦截且不落盘）；③ 仅当涉及路径内无 .rs 可内联时，才同阁新建测试文件并照 模块三件套 接线。\n",
            样例.符号, 样例.文件, 签名, 摘要
        )
    }
}

/// 节点或其子孙名字是否包含关键词（树内任意层命中即算）。
fn 树含关键词(节点: &结构节点, 关键词: &str) -> bool {
    节点.名字.contains(关键词) || 节点.子节点.iter().any(|子| 树含关键词(子, 关键词))
}

/// 提取文件路径中的「阁」目录前缀（含阁段）：从后往前找以 -阁 结尾的目录段，
/// 如 .../任务-派遣-阁/工具-循环-园/工具循环.rs → .../任务-派遣-阁/。
fn 提取阁目录(文件: &str) -> Option<String> {
    let 统一 = 文件.replace('\\', "/");
    let 段们: Vec<&str> = 统一.split('/').collect();
    let 位置 = 段们.iter().rposition(|段| 段.ends_with("-阁"))?;
    Some(段们[..=位置].join("/") + "/")
}

/// 结构节点：目录层级树（根为 crate 完整相对路径，向下为 crate 内目录）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 结构节点 {
    pub 名字: String,
    pub 子节点: Vec<结构节点>,
}

impl 结构节点 {
    /// 构造结构节点。
    pub fn 新(名字: &str) -> 结构节点 {
        结构节点 { 名字: 名字.to_string(), 子节点: Vec::new() }
    }

    /// 按层级逐段插入一段目录，复用同名节点。
    pub fn 插入(&mut self, 段们: &[String]) {
        if 段们.is_empty() {
            return;
        }
        let 首 = &段们[0];
        let 位置 = self.子节点.iter().position(|节点| 节点.名字 == *首);
        let 节点 = match 位置 {
            Some(位置) => &mut self.子节点[位置],
            None => {
                self.子节点.push(结构节点::新(首));
                self.子节点.last_mut().unwrap()
            }
        };
        节点.插入(&段们[1..]);
    }
}

/// 递归渲染目录子树（缩进 2 空格每层）。
fn 渲染子树(节点: &结构节点, 深度: usize, 输出: &mut String) {
    for _ in 0..深度 {
        输出.push_str("  ");
    }
    输出.push_str(&节点.名字);
    输出.push('\n');
    for 子 in &节点.子节点 {
        渲染子树(子, 深度 + 1, 输出);
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    /// 造带两府符号的依赖图（甲府有库根导出符号，乙府有测试符号）。
    fn 造图() -> 依赖图 {
        let mut 图 = 依赖图::default();
        图.档案们 = vec![
            符号档案::新("p", "甲府", "乾坤/甲府/入口.rs", "打开存储", "pub fn 打开存储", "fn 打开存储", "打开识海存储"),
            符号档案::新("p", "甲府", "乾坤/甲府/入口.rs", "状态目录", "pub fn 状态目录", "fn 状态目录", "状态目录"),
            符号档案::新("p", "乙府", "乾坤/乙府/入口.rs", "乙函数", "pub fn 乙函数()", "fn 乙函数", "乙函数"),
            符号档案::新("p", "乙府", "乾坤/乙府/测试园/乙测试.rs", "乙测试", "#[cfg(test)]\nmod 测试 {\n    #[test]\n    fn 用例() {}\n}", "", "测试文件"),
        ];
        图
    }

    #[test]
    fn 查库根导出_按涉及文件回溯府并给签名() {
        let 图 = 造图();
        let 结果 = 图.查库根导出(&["乾坤/甲府/入口.rs".to_string()]);
        assert!(结果.contains("甲府·库根导出"), "应含府名：{结果}");
        assert!(结果.contains("fn 打开存储"), "应含库根符号签名：{结果}");
        assert!(结果.contains("fn 状态目录"), "应含状态目录：{结果}");
        assert!(!结果.contains("乙函数"), "不应含他府符号：{结果}");
    }

    #[test]
    fn 查库根导出_目录涉及路径退化匹配() {
        let 图 = 造图();
        let 结果 = 图.查库根导出(&["甲府".to_string()]);
        assert!(结果.contains("甲府·库根导出"), "目录涉及应退化命中府：{结果}");
    }

    #[test]
    fn 查库根导出_无匹配返回空() {
        let 图 = 造图();
        let 结果 = 图.查库根导出(&["不存在的路径.rs".to_string()]);
        assert!(结果.is_empty(), "无匹配应为空：{结果}");
    }

    #[test]
    fn 查测试样例_取含cfg测试的档案() {
        let 图 = 造图();
        let 结果 = 图.查测试样例(&["乾坤/乙府/入口.rs".to_string()]);
        assert!(结果.contains("测试样例参照"), "应含标题：{结果}");
        assert!(结果.contains("乙测试"), "应选中测试符号：{结果}");
        assert!(结果.contains("cfg(test)"), "应含测试写法片段：{结果}");
        assert!(结果.contains("勿自造测试结构"), "应含照抄提示：{结果}");
        assert!(结果.contains("补测试落位判据"), "应含落位判据：{结果}");
        assert!(结果.contains("内联写在该文件尾"), "应含内联指令：{结果}");
    }

    #[test]
    fn 查测试样例_无测试档案返回空() {
        let mut 图 = 造图();
        图.档案们.retain(|档案| !档案.代码.contains("#[cfg(test)]"));
        let 结果 = 图.查测试样例(&["乾坤/甲府/入口.rs".to_string()]);
        assert!(结果.is_empty(), "无测试档案应为空：{结果}");
    }
}
