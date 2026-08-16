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
