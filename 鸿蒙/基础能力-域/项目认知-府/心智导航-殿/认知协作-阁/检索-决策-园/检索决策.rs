use serde::{Deserialize, Serialize};
use hm_contract::当前时间戳;
use crate::{维度, 图谱, 心智地图, 字符嵌入器, 嵌入器, 语义检索, 上下文库};

/// 检索源：一次检索的最终答案来源（对齐原型 检索决策.py）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum 检索源 {
    格位,
    临时,
    图谱,
    空,
}

/// 检索决策：一次检索的完整决策轨迹（用于可解释性 / 终端可见性）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 检索决策记录 {
    pub 问题: String,
    /// 候选格位 (维度, 格位名)
    pub 候选格位: Vec<(维度, String)>,
    /// 各格位判定 (维度, 格位名, 够用, 不达标项)
    pub 各格位判定: Vec<(维度, String, bool, Vec<String>)>,
    /// 下沉路径（访问过的检索源，按序）
    pub 下沉路径: Vec<检索源>,
    pub 最终来源: 检索源,
    pub 答复: String,
    /// 命中级别：图谱命中时标注「关键词命中」/「语义命中」；格位/临时/空为 None
    #[serde(default)]
    pub 命中级别: Option<String>,
    pub 时间: u64,
}

impl 检索决策记录 {
    /// 构造一次决策记录
    pub fn 新(
        问题: impl Into<String>,
        候选格位: Vec<(维度, String)>,
        各格位判定: Vec<(维度, String, bool, Vec<String>)>,
        下沉路径: Vec<检索源>,
        最终来源: 检索源,
        答复: impl Into<String>,
    ) -> Self {
        检索决策记录 {
            问题: 问题.into(),
            候选格位,
            各格位判定,
            下沉路径,
            最终来源,
            答复: 答复.into(),
            命中级别: None,
            时间: 当前时间戳(),
        }
    }
}

/// 检索器：三态检索决策器（格位 → 临时 → 图谱 → 诚实兜底）
///
/// 核心思想（设计文档第五节）：
/// - 「优先级」决定「先问谁」（省成本）：格位 > 临时 > 图谱
/// - 「置信度」决定「信谁」（保准确）：图谱 > 临时 > 格位
/// - 格位够用 → 直接用；不够 → 临时；临时也不够 → 图谱真源，并轻量校准候选格位
pub struct 检索器<'a> {
    pub 图谱: &'a 图谱,
    pub 心智: &'a mut 心智地图,
    pub 上下文: &'a 上下文库,
    /// 最大候选格位数
    pub 最大候选: usize,
    /// 临时上下文返回条数
    pub 临时返回数: usize,
    /// 语义检索命中阈值（0~1，默认 0.55；可配置，测试可调）
    pub 语义阈值: f32,
    /// 语义嵌入器（默认本地 `字符嵌入器`，可替换为 ONNX/供应商 API，可插拔）
    pub 嵌入器: Box<dyn 嵌入器>,
}

impl<'a> 检索器<'a> {
    pub fn 新(图谱: &'a 图谱, 心智: &'a mut 心智地图, 上下文: &'a 上下文库) -> Self {
        检索器 {
            图谱,
            心智,
            上下文,
            最大候选: 6,
            临时返回数: 5,
            语义阈值: 0.55,
            嵌入器: Box::new(字符嵌入器::新()),
        }
    }

    /// 指定自定义嵌入器与阈值的构造（可插拔：接入 ONNX bge-small-zh 或供应商 embeddings API）
    pub fn 带语义(
        图谱: &'a 图谱,
        心智: &'a mut 心智地图,
        上下文: &'a 上下文库,
        嵌入器: Box<dyn 嵌入器>,
        阈值: f32,
    ) -> Self {
        检索器 {
            图谱,
            心智,
            上下文,
            最大候选: 6,
            临时返回数: 5,
            语义阈值: 阈值,
            嵌入器,
        }
    }

    /// 执行一次检索，返回完整决策轨迹
    pub fn 检索(&mut self, 问题: &str) -> 检索决策记录 {
        if 问题.trim().is_empty() {
            return 检索决策记录::新(
                问题,
                Vec::new(),
                Vec::new(),
                vec![检索源::空],
                检索源::空,
                "(问题为空)",
            );
        }

        let mut 各格位判定: Vec<(维度, String, bool, Vec<String>)> = Vec::new();
        let mut 下沉路径 = vec![检索源::格位];

        // 第一站：格位
        let 候选 = self.心智.路由(问题, self.最大候选);
        if let Some(答复) = self.查格位(&候选, &mut 各格位判定) {
            return 检索决策记录::新(问题, 候选, 各格位判定, 下沉路径, 检索源::格位, 答复);
        }

        // 第二站：临时上下文
        下沉路径.push(检索源::临时);
        if let Some(答复) = self.查临时(问题) {
            return 检索决策记录::新(问题, 候选, 各格位判定, 下沉路径, 检索源::临时, 答复);
        }

        // 第三站：图谱（最可信），命中后轻量校准候选格位
        下沉路径.push(检索源::图谱);
        if let Some((答复, 级别)) = self.查图谱(问题) {
            self.校准候选格位(&候选);
            let mut 记录 = 检索决策记录::新(问题, 候选, 各格位判定, 下沉路径, 检索源::图谱, 答复);
            记录.命中级别 = Some(级别);
            return 记录;
        }

        // 三态皆空：诚实兜底
        下沉路径.push(检索源::空);
        检索决策记录::新(
            问题,
            候选,
            各格位判定,
            下沉路径,
            检索源::空,
            "(无答案：格位/临时/图谱三态均未命中)",
        )
    }

    /// 按候选顺序查格位，返回第一个够用的拼接摘要；否则 None
    fn 查格位(
        &self,
        候选: &[(维度, String)],
        判定记录: &mut Vec<(维度, String, bool, Vec<String>)>,
    ) -> Option<String> {
        let mut 命中摘要: Vec<String> = Vec::new();
        for (维度值, 名) in 候选 {
            let 信号 = self.心智.够用(*维度值, 名);
            判定记录.push((*维度值, 名.clone(), 信号.够用(), 信号.不达标项.clone()));
            if 信号.够用() {
                if let Some(格位) = self.心智.查询格位(*维度值, 名) {
                    if !格位.摘要.is_empty() {
                        命中摘要.push(format!("[{}·{}] {}", 维度值.中文名(), 名, 格位.摘要));
                    }
                }
                if 命中摘要.len() >= 3 {
                    break;
                }
            }
        }
        if 命中摘要.is_empty() {
            None
        } else {
            Some(命中摘要.join("；"))
        }
    }

    /// 查临时上下文最近相关消息；无命中返回 None
    fn 查临时(&self, 问题: &str) -> Option<String> {
        let 消息 = self.上下文.最近相关(问题, self.临时返回数);
        if 消息.is_empty() {
            return None;
        }
        let 行: Vec<String> = 消息
            .iter()
            .take(3)
            .map(|消息| {
                let 内容: String = 消息.内容.chars().take(120).collect();
                format!("[{:?}] {}", 消息.角色, 内容)
            })
            .collect();
        Some(format!("临时上下文：{}", 行.join(" | ")))
    }

    /// 查图谱：按问题关键词找节点聚合为答复；关键词无命中时降级语义相似检索。
    /// 返回 (答复, 命中级别)，命中级别为「关键词命中」或「语义命中」。
    fn 查图谱(&self, 问题: &str) -> Option<(String, String)> {
        let 关键词 = 提取关键词(问题);
        let mut 模块命中: Vec<&crate::模块> = Vec::new();
        let mut 符号命中: Vec<&crate::符号> = Vec::new();
        for 词 in &关键词 {
            模块命中.extend(self.图谱.按模块名(词));
            模块命中.extend(self.图谱.按路径(词));
            符号命中.extend(self.图谱.按符号名(词));
        }
        if 模块命中.is_empty() && 符号命中.is_empty() {
            // 关键词无命中 → 语义相似检索（同义不同词，如「任务看板」vs「看板任务」）
            return self
                .查图谱语义(问题)
                .map(|答复| (答复, "语义命中".to_string()));
        }
        let 模块名: Vec<String> = {
            let mut 集合: Vec<&str> = 模块命中.iter().map(|模块| 模块.名称.as_str()).collect();
            集合.sort_unstable();
            集合.dedup();
            集合.into_iter().take(5).map(String::from).collect()
        };
        let 符号名: Vec<String> = {
            let mut 集合: Vec<&str> = 符号命中.iter().map(|符号| 符号.名称.as_str()).collect();
            集合.sort_unstable();
            集合.dedup();
            集合.into_iter().take(5).map(String::from).collect()
        };
        let mut parts: Vec<String> = Vec::new();
        if !模块名.is_empty() {
            parts.push(format!("涉及模块：{}", 模块名.join("、")));
        }
        if !符号名.is_empty() {
            parts.push(format!("符号：{}", 符号名.join("、")));
        }
        if parts.is_empty() {
            None
        } else {
            Some((format!("图谱查询：{}", parts.join("；")), "关键词命中".to_string()))
        }
    }

    /// 查图谱语义：对模块名 + 符号名做语义相似检索，命中返回答复；无命中返回 None
    fn 查图谱语义(&self, 问题: &str) -> Option<String> {
        let mut 候选名: Vec<String> = self
            .图谱
            .全部模块()
            .iter()
            .map(|模块| 模块.名称.clone())
            .collect();
        候选名.extend(self.图谱.全部符号().iter().map(|符号| 符号.名称.clone()));
        let (命中名, _相似度) = 语义检索(问题, &候选名, self.嵌入器.as_ref(), self.语义阈值)?;
        Some(format!("图谱查询（语义命中）：{}", 命中名))
    }

    /// 图谱下沉后轻量校准候选格位：只下调置信度 0.1，不重写摘要（重写属纠错闭环职责）
    fn 校准候选格位(&mut self, 候选: &[(维度, String)]) {
        for (维度值, 名) in 候选 {
            if let Some(格位) = self.心智.查询格位(*维度值, 名) {
                if 格位.摘要.is_empty() {
                    continue;
                }
                let 新置信度 = (格位.可信度 - 0.1).max(0.0);
                if let Err(失败) = self
                    .心智
                    .写(*维度值, 名, 格位.摘要.clone(), 新置信度, 格位.证据引用.clone())
                {
                    tracing::warn!("检索校准格位失败: {失败}");
                }
            }
        }
    }
}

/// 从问题提取查询关键词：英文词（≥3 字符）+ 中文滑动窗（2-4 字）
pub fn 提取关键词(问题: &str) -> Vec<String> {
    let mut 词: Vec<String> = Vec::new();
    for 段 in 问题.split(|c: char| !c.is_ascii_alphanumeric()) {
        if 段.len() >= 3 && 段.chars().all(|c| c.is_ascii_alphanumeric()) {
            if !词.iter().any(|w| w == 段) {
                词.push(段.to_string());
            }
        }
    }
    for 中文段 in 问题
        .split(|c: char| !('\u{4e00}'..='\u{9fff}').contains(&c))
        .filter(|段| !段.is_empty())
    {
        for 长度 in 2..=4 {
            let 字: Vec<char> = 中文段.chars().collect();
            if 字.len() < 长度 {
                continue;
            }
            for i in 0..=字.len() - 长度 {
                let 切片: String = 字[i..i + 长度].iter().collect();
                if !词.iter().any(|w| w == &切片) {
                    词.push(切片);
                }
            }
        }
    }
    词.into_iter().take(30).collect()
}
