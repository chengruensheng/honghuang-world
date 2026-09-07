use std::collections::HashSet;
use std::sync::Arc;
use hm_contract::Component;
use hm_error::{Error, Result};
use hm_signal::{信号总线, 信号类型, 信号载荷};
use hm_signal::引擎支撑;
use hm_domain_contract::规则库契约;
use serde::{Deserialize, Serialize};
use crate::规则定义_殿::Rule;

/// 容量上限默认值：不限（约束由装配层注入）。
/// 可移植写法：64 位平台可序列化 i64::MAX，32 位平台回退 usize::MAX 避免溢出。
fn 默认容量上限() -> usize {
    match usize::try_from(i64::MAX) {
        Ok(上限) => 上限,
        Err(_) => usize::MAX,
    }
}

/// 规则库：收敛规则，支持事实评估、容量约束与落盘持久化
#[derive(Clone, Serialize, Deserialize)]
pub struct RuleSet {
    rules: Vec<Rule>,
    next_id: u64,
    #[serde(default = "默认容量上限")]
    容量上限: usize,
    #[serde(skip)]
    信号总线: Option<Arc<dyn 信号总线>>,
    #[serde(skip)]
    持久化路径: Option<String>,
}

impl RuleSet {
    pub fn new() -> Self {
        RuleSet {
            rules: Vec::new(),
            next_id: 1,
            容量上限: 默认容量上限(),
            信号总线: None,
            持久化路径: None,
        }
    }

    引擎支撑!();

    /// 设置规则容量上限（火克金：过盛时拒绝新增，严格数量上限）
    pub fn 设置容量上限(&mut self, 上限: usize) {
        self.容量上限 = 上限;
    }

    /// 添加规则（金之收敛：同名拒绝；达容量上限一律拒绝），返回规则 id
    pub fn 添加规则(&mut self, 名称: &str, 条件: Vec<(String, String)>, 结论: &str, 优先级: u32) -> Result<u64> {
        if self.rules.iter().any(|r| r.名称 == 名称) {
            return Err(Error::规则已存在(名称.to_string()));
        }
        if 条件.is_empty() {
            return Err(Error::Other("规则条件不能为空".to_string()));
        }
        if self.rules.len() >= self.容量上限 {
            return Err(Error::容量超限(format!("规则已达上限 {}", self.容量上限)));
        }
        let id = self.next_id;
        self.next_id += 1;
        self.rules.push(Rule::新建(id, 名称.to_string(), 条件, 结论.to_string(), 优先级));
        self.自动保存();
        Ok(id)
    }

    /// 按名称查询
    pub fn 按名称(&self, 名称: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.名称 == 名称)
    }

    /// 全部规则
    pub fn 全部(&self) -> Vec<&Rule> {
        self.rules.iter().collect()
    }

    /// 评估：匹配所有条件都满足的规则，按优先级降序返回。
    /// 事实转为 HashSet，条件匹配由 O(n) 线性扫描降为 O(1) 查找。
    pub fn 评估(&self, 事实: &[(String, String)]) -> Vec<&Rule> {
        let 事实集: HashSet<&(String, String)> = 事实.iter().collect();
        let mut matched: Vec<&Rule> = self
            .rules
            .iter()
            .filter(|r| r.条件.iter().all(|条件| 事实集.contains(条件)))
            .collect();
        matched.sort_by(|a, b| b.优先级.cmp(&a.优先级));
        if let Some(命中的) = matched.first() {
            self.发布信号(
                信号类型::规则命中,
                信号载荷 {
                    规则名: Some(命中的.名称.clone()),
                    结论: Some(命中的.结论.clone()),
                    ..信号载荷::default()
                },
            );
        }
        matched
    }

    /// 保存到文件（落盘）
    pub fn 保存(&self, path: &str) -> Result<()> {
        let content = toml::to_string(self)
            .map_err(|e| Error::序列化(format!("序列化规则失败: {e}")))?;
        hm_contract::原子写入文件(path, &content)?;
        Ok(())
    }

    /// 从文件加载（还原）
    pub fn 加载(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
        toml::from_str(&content)
            .map_err(|e| Error::反序列化(format!("解析规则文件失败: {e}")))
    }

}

impl Component for RuleSet {
    fn name(&self) -> &'static str { "规则库" }
}

impl 规则库契约<Rule> for RuleSet {
    fn 添加规则(&mut self, 名称: &str, 条件: Vec<(String, String)>, 结论: &str, 优先级: u32) -> Result<u64> {
        RuleSet::添加规则(self, 名称, 条件, 结论, 优先级)
    }

    fn 按名称(&self, 名称: &str) -> Option<&Rule> {
        RuleSet::按名称(self, 名称)
    }

    fn 全部(&self) -> Vec<&Rule> {
        RuleSet::全部(self)
    }

    fn 评估(&self, 事实: &[(String, String)]) -> Vec<&Rule> {
        RuleSet::评估(self, 事实)
    }

}
