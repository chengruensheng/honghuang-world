use crate::{图谱, 格位, 心智地图, 上下文库, 消息角色, 提取关键词, 维度, 维度载荷, 规则层级};

/// 注入片段：一个待注入 LLM 的文本片段（含来源与角色）
#[derive(Clone, Debug, PartialEq)]
pub struct 注入片段 {
    pub 来源: String,
    pub 角色: 消息角色,
    pub 内容: String,
}

/// 注入器：把三态内容拼装成最终注入上下文（推=格位常驻 / 流=临时最近 / 拉=图谱按需）
pub struct 注入器<'a> {
    pub 图谱: &'a 图谱,
    pub 心智: &'a 心智地图,
    pub 上下文: &'a 上下文库,
    /// 推模式下最多注入几个格位
    pub 常驻上限: usize,
    /// 流模式下最多注入几条消息
    pub 流式窗口: usize,
}

impl<'a> 注入器<'a> {
    pub fn 新(图谱: &'a 图谱, 心智: &'a 心智地图, 上下文: &'a 上下文库) -> Self {
        注入器 {
            图谱,
            心智,
            上下文,
            常驻上限: 10,
            流式窗口: 20,
        }
    }

    /// 推模式：常驻注入「已填充」格位的最新摘要（按可信度降序，≤ 常驻上限）。
    /// 规则维度只推「大道」层级（天道/临时不参与常驻注入，分别走拉态和流态）。
    pub fn 推_格位摘要(&self) -> Vec<注入片段> {
        let mut 已填: Vec<&格位> = self
            .心智
            .全部()
            .iter()
            .filter(|格位| {
                if 格位.摘要.is_empty() || 格位.可信度 <= 0.0 {
                    return false;
                }
                // 规则维度只推大道层级
                if 格位.维度 == 维度::规则 {
                    return matches!(
                        &格位.维度载荷,
                        Some(维度载荷::规则(载荷)) if 载荷.层级 == 规则层级::大道
                    );
                }
                true
            })
            .collect();
        已填.sort_by(|a, b| b.可信度.total_cmp(&a.可信度));
        已填
            .into_iter()
            .take(self.常驻上限)
            .map(|格位| 注入片段 {
                来源: "格位".into(),
                角色: 消息角色::系统,
                内容: format!(
                    "[{}·{}] (可信度={:.2}) {}",
                    格位.维度.中文名(),
                    格位.格位名,
                    格位.可信度,
                    格位.摘要
                ),
            })
            .collect()
    }

    /// 流模式：注入最近 N 条消息
    pub fn 流_最近消息(&self) -> Vec<注入片段> {
        self.上下文
            .最近(self.流式窗口)
            .into_iter()
            .map(|消息| 注入片段 {
                来源: "临时".into(),
                角色: 消息.角色.clone(),
                内容: 消息.内容.clone(),
            })
            .collect()
    }

    /// 拉模式：按问题从图谱拉相关节点（≤ 最大）
    pub fn 拉_图谱片段(&self, 问题: &str, 最大: usize) -> Vec<注入片段> {
        let 关键词 = 提取关键词(问题);
        let mut 命中: Vec<String> = Vec::new();
        for 词 in &关键词 {
            for 模块 in self.图谱.按模块名(词) {
                if !命中.iter().any(|id| id == &模块.名称) {
                    命中.push(模块.名称.clone());
                }
            }
            for 符号 in self.图谱.按符号名(词) {
                let id = format!("{}/{}", 符号.所属模块, 符号.名称);
                if !命中.iter().any(|已有| 已有 == &id) {
                    命中.push(id);
                }
            }
        }
        命中.into_iter()
            .take(最大)
            .map(|id| 注入片段 {
                来源: "图谱".into(),
                角色: 消息角色::系统,
                内容: format!("[节点] {}", id),
            })
            .collect()
    }
}
