//! 依赖 - 图 - 类型：符号档案、依赖图、结构树（扫描配套）。

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 测试覆盖判定（接单门维度5「可验证性」缓存，2026-08-25 §19.4.1/§8.5 入稿）。
/// - 未判定：扫描默认（依赖图刚扫完或人工未确认）
/// - 是测试：代码本身就是测试（含 #[test]/#[cfg(test)] 或路径命中测试域）
/// - 不是测试：代码被误判（路径前缀/符号扫描命中但实际不是测试，记录待修正名单）
/// - 边缘：测试 helper/示例/fixture 等 LLM 看不出，需人工审
/// 设计：依赖图已读全文 → 零额外 IO 算字段值；LLM 现场确认写回；下次扫描保留（不覆盖）。
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum 测试覆盖判定 {
    #[default]
    未判定,
    是测试,
    不是测试,
    边缘,
}

/// 符号档案：单个符号（函数 / 类型）的微观语义。
/// 五层标识（项目 / 模块 / 文件 / 符号 / 代码）+ 解释 + 波及 + 测试覆盖判定（2026-08-25）。
/// 代码 = 完整定义体（M4 实测：执行层只喂命中函数体，质量最高且省 token）；
/// 签名 = 单行签名（补解释等摘要场景用，避免把整个函数体塞进提示）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 符号档案 {
    pub 项目: String,
    pub 模块: String,
    pub 文件: String,
    pub 符号: String,
    pub 代码: String,
    pub 签名: String,
    pub 解释: String,
    pub 波及: Vec<String>,
    /// 测试覆盖判定（接单门维度5 缓存，2026-08-25 §19.4.1/§8.5 入稿）。
    /// 默认 未判定（依赖图刚扫完）；扫描算字段值；LLM 现场确认写回。
    /// `#[serde(default)]` 兼容旧档案（缺字段时反序列化为 未判定）。
    #[serde(default)]
    pub 测试覆盖判定: 测试覆盖判定,
}

impl 符号档案 {
    /// 构造符号档案（波及初始为空，扫引用后回填）。
    /// 路径归一：文件、模块写入时统一正斜杠，查询时直接比较无需 replace（热路径优化）。
    pub fn 新(
        项目: &str,
        模块: &str,
        文件: &str,
        符号: &str,
        代码: &str,
        签名: &str,
        解释: &str,
    ) -> 符号档案 {
        符号档案 {
            项目: 项目.to_string(),
            模块: 模块.replace('\\', "/"),
            文件: 文件.replace('\\', "/"),
            符号: 符号.to_string(),
            代码: 代码.to_string(),
            签名: 签名.to_string(),
            解释: 解释.to_string(),
            波及: Vec::new(),
            // 默认 未判定（接单门维度5 缓存默认值），扫描时多算这一项。
            测试覆盖判定: 测试覆盖判定::default(),
        }
    }

    /// 原地归一化路径字段（文件、模块、波及）为正斜杠。
    /// 用于加载旧 json 兼容：旧档案可能存反斜杠，加载后调一次归一，后续查询免 replace。
    pub fn 归一路径(&mut self) {
        if self.文件.contains('\\') {
            self.文件 = self.文件.replace('\\', "/");
        }
        if self.模块.contains('\\') {
            self.模块 = self.模块.replace('\\', "/");
        }
        for 波 in &mut self.波及 {
            if 波.contains('\\') {
                *波 = 波.replace('\\', "/");
            }
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

    /// 原地归一化全部档案路径字段为正斜杠。
    /// 加载旧 json 后调用一次：旧档案可能存反斜杠，归一后所有查询免 replace（热路径优化）。
    pub fn 归一(&mut self) {
        for 档案 in &mut self.档案们 {
            档案.归一路径();
        }
    }

    /// 按符号名查档案（512→64 追溯）。
    pub fn 查符号(&self, 符号名: &str) -> Vec<&符号档案> {
        self.档案们
            .iter()
            .filter(|档案| 档案.符号 == 符号名)
            .collect()
    }

    /// 按文件查档案（64→512 定位）。
    pub fn 查文件(&self, 文件: &str) -> Vec<&符号档案> {
        self.档案们
            .iter()
            .filter(|档案| 档案.文件 == 文件)
            .collect()
    }

    /// 按模块（府 / 域）查档案。
    pub fn 查模块(&self, 模块: &str) -> Vec<&符号档案> {
        self.档案们
            .iter()
            .filter(|档案| 档案.模块 == 模块)
            .collect()
    }

    /// 按涉及路径（符号名 / 文件路径片段）查相关文件集合（含波及文件）。
    /// 符号名支持子串匹配；供执行层精确读现状：涉及什么 → 波及哪些文件 → 只读这些。
    pub fn 查涉及文件(&self, 涉及路径们: &[String]) -> Vec<String> {
        let mut 文件集 = HashSet::new();
        for 涉及 in 涉及路径们 {
            let 涉及 = 涉及.trim().replace('\\', "/");
            if 涉及.is_empty() {
                continue;
            }
            for 档案 in &self.档案们 {
                if 档案.符号 == 涉及 || 档案.符号.contains(&涉及) || 档案.文件.contains(&涉及)
                {
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
            .filter(|文件| self.档案们.iter().any(|档案| 档案.文件 == **文件))
            .map(|文件| 文件.to_string())
            .collect()
    }

    /// 跨阁出边补全：对给定文件集，纳入它们 use 引用的目标文件（出边）。
    /// 波及字段记录的是入边（谁引用了我），出边 = 波及里含我的档案所在文件。
    /// 设计稿 §4.2：防模型需看跨阁调用但现状只有签名清单。
    pub fn 补全跨阁出边(&self, 文件们: &[String]) -> Vec<String> {
        let mut 文件集: HashSet<String> = 文件们.iter().cloned().collect();
        for 文件 in 文件们 {
            let 文件归一 = 文件.replace('\\', "/");
            for 档案 in &self.档案们 {
                if 档案.波及.contains(&文件归一) {
                    文件集.insert(档案.文件.clone());
                }
            }
        }
        let mut 结果: Vec<String> = 文件集.into_iter().collect();
        结果.sort();
        结果
    }

    /// 同分组目录补全：把给定文件各自所在父目录下的全部源文件一并纳入。
    /// 通用启发——「同目录 = 同分组/同模块」，任何 Rust 项目分层结构下都成立，
    /// 不依赖本项目「阁/园」目录专名（原 提取阁目录 按「-阁」后缀判别，已去架构耦合）。
    /// 保证目标文件所在分组对执行者完整可见（函数级切片照常应用，不读全文）。
    pub fn 补全同阁(&self, 文件们: &[String]) -> Vec<String> {
        let mut 文件集: HashSet<String> = 文件们.iter().cloned().collect();
        for 文件 in 文件们 {
            let Some(父) = 父目录(文件) else {
                continue;
            };
            for 档案 in &self.档案们 {
                if 档案.文件.starts_with(&父) {
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
        let 兜底 = !关键词们.is_empty() && !self.结构树.子节点.iter().any(命中);
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

    /// 图谱契约·库根导出（设计稿 §4.2 规则6）：按涉及文件追溯所属 crate（模块字段末段 =
    /// crate 名），投影该 crate 可导入符号的签名清单——让实现层知道 打开存储/状态目录/fn 入口
    /// 等库根符号在哪、签名什么样，不再凭记忆猜。通用：任何 Rust 项目 crate 名即模块末段，
    /// 不依赖本项目 殿/阁/园 目录层级名词。
    /// 返回「crate 名 → 签名清单」文本；无匹配返回空串（零开销）。
    pub fn 查库根导出(&self, 涉及路径们: &[String]) -> String {
        let mut crate名集 = std::collections::BTreeSet::new();
        for 涉及 in 涉及路径们 {
            let 涉及 = 涉及.trim().replace('\\', "/");
            if 涉及.is_empty() {
                continue;
            }
            for 档案 in &self.档案们 {
                if 档案.文件.contains(&涉及) {
                    if let Some(crate名) = 档案.模块.split('/').next_back() {
                        if !crate名.is_empty() {
                            crate名集.insert(crate名.to_string());
                        }
                    }
                }
            }
        }
        // 精确到文件却无归属 crate（涉及路径是目录）→ 退化按目录名匹配模块。
        if crate名集.is_empty() {
            for 涉及 in 涉及路径们 {
                let 涉及 = 涉及.trim();
                for 档案 in &self.档案们 {
                    if 档案.模块.ends_with(涉及) {
                        if let Some(crate名) = 档案.模块.split('/').next_back() {
                            crate名集.insert(crate名.to_string());
                        }
                    }
                }
            }
        }
        let mut 输出 = String::new();
        for crate名 in &crate名集 {
            let 符号们 = self.查模块(crate名);
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
            输出.push_str(&format!("【{crate名}·库根导出】\n{}\n", 签名们.join("\n")));
        }
        输出
    }

    /// 图谱契约·测试样例参照（设计稿 §4.2 规则6）：依赖图中找含 `#[cfg(test)]` /
    /// `mod 测试` 的符号档案，优先取与涉及路径同 crate 的文件，投影其中一个完整测试模块的
    /// 签名+文件路径——实现层照抄该项目既有测试惯例（模块接入方式、#[cfg(test)] 写法、
    /// env 互斥锁），不再凭空发明测试结构。
    /// 返回「样本清单 + 一个完整样例文件路径」文本；无样例返回空串。
    /// 落位判据用通用描述（不写死本项目目录层级名词），任何 Rust 项目均适用。
    pub fn 查测试样例(&self, 涉及路径们: &[String]) -> String {
        // 同 crate 优先：涉及路径文件所属 crate 的测试档案。
        let 涉及crate们: std::collections::HashSet<String> = {
            let mut 集合 = std::collections::HashSet::new();
            for 涉及 in 涉及路径们 {
                let 涉及 = 涉及.trim().replace('\\', "/");
                for 档案 in &self.档案们 {
                    if 档案.文件.contains(&涉及) {
                        if let Some(crate名) = 档案.模块.split('/').next_back() {
                            集合.insert(crate名.to_string());
                        }
                    }
                }
            }
            集合
        };
        let 测试档案们: Vec<&符号档案> = self
            .档案们
            .iter()
            .filter(|档案| {
                档案.代码.contains("#[cfg(test)]")
                    || 档案.代码.contains("mod 测试")
                    || 档案.代码.contains("mod tests")
            })
            .collect();
        if 测试档案们.is_empty() {
            return String::new();
        }
        let 同crate: Vec<&符号档案> = 测试档案们
            .iter()
            .copied()
            .filter(|档案| {
                档案
                    .模块
                    .split('/')
                    .next_back()
                    .is_some_and(|crate名| 涉及crate们.contains(crate名))
            })
            .collect();
        let 样例 = if let Some(首) = 同crate.first() {
            *首
        } else {
            *测试档案们.first().unwrap()
        };
        let 签名 = if 样例.签名.is_empty() {
            样例.符号.clone()
        } else {
            样例.签名.clone()
        };
        let 摘要: String = 样例.代码.lines().take(5).collect::<Vec<_>>().join("\n");
        // 落位判据 = 通用 Rust 测试落位准则（不依赖本项目 殿/阁/园 或 模块三件套 等专有名词）。
        format!(
            "【测试样例参照】项目既有测试写法样例：{}\n文件路径：{}\n签名：{}\n片段：\n{}\n照抄此处的 测试模块声明/#[cfg(test)]/mod 测试 写法与模块接入方式，勿自造结构。\n\n\
             【补测试落位判据】① 涉及路径内已有被测 .rs → 测试直接内联写在该文件尾 #[cfg(test)]，禁止新建独立测试文件；② 勿往空目录/非涉及路径写测试（会被护栏拦截且不落盘）；③ 仅当涉及路径内无 .rs 可内联时，才在同类目录新建测试文件、并在父层模块按该 crate 既有接入方式声明（照上方测试样例示范）。\n",
            样例.符号, 样例.文件, 签名, 摘要
        )
    }

    /// 清理文件：移除该文件的符号档案 + 从其他档案.波及 中移除该文件（入边清理）。
    /// 用于增量检测识别删除后同步清理陈旧边（设计稿 §14.20.6）。
    /// 路径归一：档案.文件/波及 已在写入时归一为正斜杠，此处仅归一入参一次，内循环免 replace。
    /// 返回移除档案条数（不含波及清理数）。
    pub fn 清理文件(&mut self, 路径: &str) -> usize {
        let 路径归一 = 路径.replace('\\', "/");
        let 原数 = self.档案们.len();
        self.档案们.retain(|档案| 档案.文件 != 路径归一);
        let 移除数 = 原数 - self.档案们.len();
        for 档案 in &mut self.档案们 {
            档案.波及.retain(|波| *波 != 路径归一);
        }
        移除数
    }

    /// 统计 .rs 文件中测试符号命中数（#[test] / #[cfg(test)] / mod 测试 / mod tests）。
    /// 依赖图扫描第一遍复用：避免重复 IO，扫描全文时多算这一项。
    /// 返回 (含测试标记行数, 含 mod 测试行数)；命中其一即视为测试文件。
    pub fn 扫测试符号(行们: &[&str]) -> (usize, usize) {
        let mut 测试标记 = 0usize;
        let mut 测试模块 = 0usize;
        for 行 in 行们 {
            if 行.contains("#[test]") || 行.contains("#[cfg(test)]") {
                测试标记 += 1;
            }
            if 行.contains("mod 测试") || 行.contains("mod tests") {
                测试模块 += 1;
            }
        }
        (测试标记, 测试模块)
    }
}

/// 路径前缀过筛（接单门维度5 第①层，2026-08-25 §8.5/§19.4.1 入稿）。
/// 命中测试域（证道/验证殿/验证阁/验证园）或文件名属测试文件 → 天然是测试。
/// 设计稿 §8.5「路径前缀过筛」零 IO 判别，§七 通用启发 + 本项目六层约定。
/// 后续落码步骤 4 的接单门闭包会调用此函数（撞车延后实现）。
pub fn 是_测试文件(路径: &str) -> bool {
    // 路径归一：Windows 反斜杠归一为正斜杠（依赖图字段已归一，传入参数兼容两种）。
    let 归一 = 路径.replace('\\', "/");
    // 测试域：证道/*（专用测试维度）+ 任意府下 验证殿/验证阁/验证园（测试镜像）。
    if 归一.starts_with("证道/") {
        return true;
    }
    if 归一.contains("/验证殿/")
        || 归一.contains("/验证阁/")
        || 归一.contains("/验证园/")
    {
        return true;
    }
    // 文件名 = 测试.rs / tests.rs / *测试*.rs / *Tests*.rs。
    let 文件名 = std::path::Path::new(&归一)
        .file_name()
        .and_then(|名| 名.to_str())
        .unwrap_or("");
    if 文件名 == "测试.rs" || 文件名 == "tests.rs" {
        return true;
    }
    if 文件名.starts_with("测试") || 文件名.starts_with("tests") {
        return true;
    }
    if 文件名.ends_with("测试.rs") || 文件名.ends_with("Test.rs") || 文件名.ends_with("Tests.rs") {
        return true;
    }
    false
}

/// 节点或其子孙名字是否包含关键词（树内任意层命中即算）。
fn 树含关键词(节点: &结构节点, 关键词: &str) -> bool {
    节点.名字.contains(关键词) || 节点.子节点.iter().any(|子| 树含关键词(子, 关键词))
}

/// 取文件路径的父目录前缀（含尾部 /）：去掉最后一段（文件名），返回其上级目录。
/// 通用，不依赖本项目「阁/园」目录专名；任何 Rust 项目路径都成立。
/// 如 .../任务-派遣-阁/工具-循环-园/工具循环.rs → .../任务-派遣-阁/工具-循环-园/。
fn 父目录(文件: &str) -> Option<String> {
    let 统一 = 文件.replace('\\', "/");
    let 段们: Vec<&str> = 统一.split('/').collect();
    if 段们.len() < 2 {
        return None;
    }
    Some(段们[..段们.len() - 1].join("/") + "/")
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
        结构节点 {
            名字: 名字.to_string(),
            子节点: Vec::new(),
        }
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
        依赖图 {
            档案们: vec![
                符号档案::新(
                    "p",
                    "甲府",
                    "乾坤/甲府/入口.rs",
                    "打开存储",
                    "pub fn 打开存储",
                    "fn 打开存储",
                    "打开识海存储",
                ),
                符号档案::新(
                    "p",
                    "甲府",
                    "乾坤/甲府/入口.rs",
                    "状态目录",
                    "pub fn 状态目录",
                    "fn 状态目录",
                    "状态目录",
                ),
                符号档案::新(
                    "p",
                    "乙府",
                    "乾坤/乙府/入口.rs",
                    "乙函数",
                    "pub fn 乙函数()",
                    "fn 乙函数",
                    "乙函数",
                ),
                符号档案::新(
                    "p",
                    "乙府",
                    "乾坤/乙府/测试园/乙测试.rs",
                    "乙测试",
                    "#[cfg(test)]\nmod 测试 {\n    #[test]\n    fn 用例() {}\n}",
                    "",
                    "测试文件",
                ),
            ],
            结构树: 结构节点::新("根结构"),
        }
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
        assert!(
            结果.contains("甲府·库根导出"),
            "目录涉及应退化命中府：{结果}"
        );
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
        assert!(结果.contains("勿自造结构"), "应含照抄提示：{结果}");
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

    #[test]
    fn 补全同阁_按父目录分组补全同组源码() {
        let mut 图 = 造图();
        // 加一个与 入口.rs 同父目录(乾坤/甲府/)的兄弟文件 → 应被补全。
        图.档案们.push(符号档案::新(
            "p",
            "甲府",
            "乾坤/甲府/兄弟.rs",
            "兄弟",
            "pub fn 兄弟",
            "fn 兄弟",
            "",
        ));
        // 加一个在另一父目录(乾坤/乙府/)的文件 → 不应被补全。
        图.档案们.push(符号档案::新(
            "p",
            "乙府",
            "乾坤/乙府/别的.rs",
            "别的",
            "pub fn 别的",
            "fn 别的",
            "",
        ));
        let 结果 = 图.补全同阁(&["乾坤/甲府/入口.rs".to_string()]);
        assert!(
            结果.iter().any(|路径| 路径 == "乾坤/甲府/入口.rs"),
            "应含自身：{结果:?}"
        );
        assert!(
            结果.iter().any(|路径| 路径 == "乾坤/甲府/兄弟.rs"),
            "应补全同父目录兄弟：{结果:?}"
        );
        assert!(
            !结果.iter().any(|路径| 路径.contains("别的.rs")),
            "他父目录文件不应被补全：{结果:?}"
        );
    }

    #[test]
    fn 补全同阁_无父目录返回自身() {
        let 图 = 依赖图::default();
        let 结果 = 图.补全同阁(&["入口.rs".to_string()]);
        assert_eq!(
            结果,
            vec!["入口.rs".to_string()],
            "单段路径无父目录应仅返回自身：{结果:?}"
        );
    }

    #[test]
    fn 补全跨阁出边_纳入涉及文件use引用的目标文件() {
        // 波及字段记录入边（谁引用了我）：文件A use 符号"被调" → "被调"的档案.波及 = [文件A]。
        // 补全跨阁出边：对文件A，找波及含文件A的档案 → "被调"的档案（在文件B）→ 纳入文件B。
        let mut 图 = 依赖图 {
            档案们: vec![
                符号档案::新(
                    "p",
                    "甲府",
                    "乾坤/甲府/调用方.rs",
                    "调用方",
                    "pub fn 调用方",
                    "fn 调用方",
                    "",
                ),
                符号档案::新(
                    "p",
                    "乙府",
                    "乾坤/乙府/被调.rs",
                    "被调",
                    "pub fn 被调",
                    "fn 被调",
                    "",
                ),
            ],
            结构树: 结构节点::新("根结构"),
        };
        // 回填波及：文件A 引用了 "被调" → "被调" 的波及含文件A。
        图.档案们[1].波及.push("乾坤/甲府/调用方.rs".to_string());
        let 结果 = 图.补全跨阁出边(&["乾坤/甲府/调用方.rs".to_string()]);
        assert!(
            结果.iter().any(|路径| 路径 == "乾坤/甲府/调用方.rs"),
            "应含自身：{结果:?}"
        );
        assert!(
            结果.iter().any(|路径| 路径 == "乾坤/乙府/被调.rs"),
            "应纳入跨阁出边目标文件：{结果:?}"
        );
    }

    #[test]
    fn 补全跨阁出边_无出边时返回自身() {
        let 图 = 依赖图 {
            档案们: vec![符号档案::新(
                "p",
                "甲府",
                "乾坤/甲府/孤立.rs",
                "孤立",
                "pub fn 孤立",
                "fn 孤立",
                "",
            )],
            结构树: 结构节点::新("根结构"),
        };
        let 结果 = 图.补全跨阁出边(&["乾坤/甲府/孤立.rs".to_string()]);
        assert_eq!(
            结果,
            vec!["乾坤/甲府/孤立.rs".to_string()],
            "无出边应仅返回自身：{结果:?}"
        );
    }

    #[test]
    fn 清理文件_移除该文件档案并清理入边() {
        let mut 图 = 依赖图 {
            档案们: vec![
                符号档案::新(
                    "p",
                    "甲府",
                    "乾坤/甲府/甲.rs",
                    "甲函数",
                    "pub fn 甲函数",
                    "fn 甲函数",
                    "",
                ),
                符号档案::新(
                    "p",
                    "甲府",
                    "乾坤/甲府/甲.rs",
                    "甲辅",
                    "fn 甲辅",
                    "fn 甲辅",
                    "",
                ),
                符号档案::新(
                    "p",
                    "乙府",
                    "乾坤/乙府/乙.rs",
                    "乙函数",
                    "pub fn 乙函数",
                    "fn 乙函数",
                    "",
                ),
            ],
            结构树: 结构节点::新("根结构"),
        };
        // 乙函数 被 甲.rs 引用 → 乙函数.波及 含 甲.rs（入边）。
        图.档案们[2].波及.push("乾坤/甲府/甲.rs".to_string());

        let 移除数 = 图.清理文件("乾坤/甲府/甲.rs");
        assert_eq!(移除数, 2, "甲.rs 有两个符号档案应移除2");
        assert!(
            图.查文件("乾坤/甲府/甲.rs").is_empty(),
            "甲.rs 档案应已清空"
        );
        assert_eq!(图.查文件("乾坤/乙府/乙.rs").len(), 1, "乙.rs 档案应保留");
        assert!(
            图.档案们
                .iter()
                .all(|档案| 档案.波及.iter().all(|波| 波 != "乾坤/甲府/甲.rs")),
            "甲.rs 的入边应已从所有波及中清理"
        );
    }

    #[test]
    fn 清理文件_路径分隔符归一() {
        let mut 图 = 依赖图 {
            档案们: vec![符号档案::新(
                "p",
                "甲府",
                "乾坤\\甲府\\甲.rs",
                "甲函数",
                "pub fn 甲函数",
                "fn 甲函数",
                "",
            )],
            结构树: 结构节点::新("根结构"),
        };
        // 构造时已归一为正斜杠，正斜杠调用应命中。
        let 移除数 = 图.清理文件("乾坤/甲府/甲.rs");
        assert_eq!(移除数, 1, "正斜杠调用应清理归一化后的档案");
        assert!(图.档案们.is_empty());
    }

    #[test]
    fn 清理文件_不存在路径移除零() {
        let mut 图 = 依赖图 {
            档案们: vec![符号档案::新(
                "p",
                "甲府",
                "乾坤/甲府/甲.rs",
                "甲函数",
                "pub fn 甲函数",
                "fn 甲函数",
                "",
            )],
            结构树: 结构节点::新("根结构"),
        };
        assert_eq!(图.清理文件("乾坤/不存在.rs"), 0, "不存在路径应移除0");
        assert_eq!(图.档案们.len(), 1, "原档案应保留");
    }

    // ─────────── §19.4.1 + §8.5 测试覆盖判定单测（2026-08-25 入稿）───────────

    #[test]
    fn 符号档案_默认测试覆盖判定_未判定() {
        // ::新 构造的档案字段应 = 未判定（接单门维度5 缓存默认值）。
        let 档案 = 符号档案::新("p", "甲府", "甲.rs", "甲", "code", "sig", "");
        assert_eq!(档案.测试覆盖判定, 测试覆盖判定::未判定);
    }

    #[test]
    fn 符号档案_默认派生_空档案可构造() {
        // #[derive(Default)] 支持，依赖图空档案构造不应崩。
        let 档案 = 符号档案::default();
        assert_eq!(档案.测试覆盖判定, 测试覆盖判定::未判定);
        assert_eq!(档案.项目, "");
        assert_eq!(档案.波及.len(), 0);
    }

    #[test]
    fn 符号档案_旧json反序列化_测试覆盖判定默认未判定() {
        // 旧档案无 测试覆盖判定 字段 → #[serde(default)] 兜底为 未判定（兼容旧数据）。
        let 旧json = r#"{
            "项目": "p",
            "模块": "甲府",
            "文件": "甲.rs",
            "符号": "甲",
            "代码": "code",
            "签名": "sig",
            "解释": "",
            "波及": []
        }"#;
        let 档案: 符号档案 = serde_json::from_str(旧json).expect("旧档案应兼容");
        assert_eq!(档案.测试覆盖判定, 测试覆盖判定::未判定);
    }

    #[test]
    fn 是_测试文件_命中测试域_证道_过() {
        assert!(是_测试文件("证道/鸿蒙-域/单元测试-府/验证殿/某阁/某验证园/某测试.rs"));
    }

    #[test]
    fn 是_测试文件_命中验证殿_过() {
        assert!(是_测试文件("鸿蒙/基础设施 - 域/某府/验证殿/某阁/某测试.rs"));
    }

    #[test]
    fn 是_测试文件_文件名测试rs_过() {
        assert!(是_测试文件("鸿蒙/某府/某阁/某园/测试.rs"));
        assert!(是_测试文件("鸿蒙/某府/某阁/某园/即时应答测试.rs"));
        assert!(是_测试文件("鸿蒙/某府/某阁/某园/即时应答Test.rs"));
    }

    #[test]
    fn 是_测试文件_生产文件_拒() {
        assert!(!是_测试文件("鸿蒙/某府/某阁/某园/业务.rs"));
        assert!(!是_测试文件("世界/入口.rs"));
        assert!(!是_测试文件("鸿蒙/基础设施 - 域/某府/入口执行.rs"));
    }

    #[test]
    fn 是_测试文件_反斜杠归一_过() {
        // Windows 路径反斜杠归一为正斜杠后判别。
        assert!(是_测试文件("鸿蒙\\某府\\某阁\\测试园\\测试.rs"));
    }

    #[test]
    fn 扫测试符号_命中test和mod测试_计数() {
        let 行们 = vec![
            "pub fn x() {}",                                  // 0
            "#[test]",                                         // +1 测试标记
            "fn 用例() {}",                                     // 0
            "#[cfg(test)]",                                    // +1 测试标记
            "mod tests {",                                     // +1 测试模块
            "mod 测试 { #[test] fn y() {} }",                  // +2 (test + mod 测试)
        ];
        let (测试标记, 测试模块) = 依赖图::扫测试符号(&行们);
        assert_eq!(测试标记, 3, "应命中 #[test] 3 次");
        assert_eq!(测试模块, 2, "应命中 mod tests/测试 2 次");
    }

    #[test]
    fn 扫测试符号_空行_计数0() {
        let 行们: Vec<&str> = vec![];
        let (测试标记, 测试模块) = 依赖图::扫测试符号(&行们);
        assert_eq!(测试标记, 0);
        assert_eq!(测试模块, 0);
    }
}
