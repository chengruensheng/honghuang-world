use crate::{维度, 图谱};

/// 候选摘要：提炼器的一次产出（对齐原型 提炼规则.py）
#[derive(Clone, Debug, PartialEq)]
pub struct 候选摘要 {
    pub 维度: 维度,
    pub 格位名: String,
    pub 摘要: String,
    pub 证据引用: Vec<String>,
    pub 可信度: f32,
    /// 模式：规则 / LLM / 评审（当前仅规则）
    pub 模式: String,
}

/// 规则提炼器：纯机械、零网络、可重现；把图谱映射为候选摘要
pub struct 规则提炼器;

impl 规则提炼器 {
    /// 常用工具依赖名 → 用途（对齐原型）
    const 常用工具: &'static [(&'static str, &'static str)] = &[
        ("serde", "序列化"),
        ("tokio", "异步运行时"),
        ("tracing", "日志追踪"),
        ("anyhow", "错误处理"),
        ("thiserror", "错误派生"),
        ("ureq", "同步HTTP"),
        ("reqwest", "异步HTTP"),
    ];

    /// 从图谱提炼候选摘要（外在·结构 / 外在·依赖 / 执行·技术栈 / 执行·工具）
    pub fn 提炼(&self, 图谱: &图谱) -> Vec<候选摘要> {
        let mut 全部: Vec<候选摘要> = Vec::new();
        全部.push(self.外在结构(图谱));
        全部.push(self.外在依赖(图谱));
        全部.push(self.执行技术栈(图谱));
        全部.push(self.执行工具(图谱));
        全部.retain(|候选| !候选.摘要.is_empty());
        全部
    }

    /// 外在·结构：模块总数与模块名清单
    fn 外在结构(&self, 图谱: &图谱) -> 候选摘要 {
        let 模块数 = 图谱.模块集.len();
        let 名称: Vec<&str> = 图谱.模块集.iter().map(|模块| 模块.名称.as_str()).collect();
        let 摘要 = if 模块数 == 0 {
            String::new()
        } else {
            format!("项目共 {} 个模块：{}", 模块数, 名称.join("、"))
        };
        候选摘要 {
            维度: 维度::外在,
            格位名: "结构".into(),
            摘要,
            证据引用: 图谱.模块集.iter().map(|模块| 模块.名称.clone()).collect(),
            可信度: 0.8,
            模式: "规则".into(),
        }
    }

    /// 外在·依赖：依赖边摘要（源 → 目标）
    fn 外在依赖(&self, 图谱: &图谱) -> 候选摘要 {
        if 图谱.依赖集.is_empty() {
            return 候选摘要 {
                维度: 维度::外在,
                格位名: "依赖".into(),
                摘要: String::new(),
                证据引用: Vec::new(),
                可信度: 0.8,
                模式: "规则".into(),
            };
        }
        let 行: Vec<String> = 图谱
            .依赖集
            .iter()
            .take(10)
            .map(|边| format!("{}→{}", 边.源, 边.目标))
            .collect();
        候选摘要 {
            维度: 维度::外在,
            格位名: "依赖".into(),
            摘要: format!("依赖方向（{} 条）：{}", 图谱.依赖集.len(), 行.join("、")),
            证据引用: 图谱
                .依赖集
                .iter()
                .map(|边| format!("{}→{}", 边.源, 边.目标))
                .collect(),
            可信度: 0.8,
            模式: "规则".into(),
        }
    }

    /// 执行·技术栈：基于图谱技术栈线索
    fn 执行技术栈(&self, 图谱: &图谱) -> 候选摘要 {
        if 图谱.技术栈.is_empty() {
            return 候选摘要 {
                维度: 维度::执行,
                格位名: "技术栈".into(),
                摘要: String::new(),
                证据引用: Vec::new(),
                可信度: 0.7,
                模式: "规则".into(),
            };
        }
        候选摘要 {
            维度: 维度::执行,
            格位名: "技术栈".into(),
            摘要: format!("技术栈：{}", 图谱.技术栈.join("、")),
            证据引用: 图谱.技术栈.clone(),
            可信度: 0.7,
            模式: "规则".into(),
        }
    }

    /// 执行·工具：技术栈中命中的常用工具（serde/tokio/...）
    fn 执行工具(&self, 图谱: &图谱) -> 候选摘要 {
        let 命中: Vec<String> = Self::常用工具
            .iter()
            .filter(|(名, _)| 图谱.技术栈.iter().any(|已有| 已有 == 名))
            .map(|(名, 用途)| format!("{}（{}）", 名, 用途))
            .collect();
        if 命中.is_empty() {
            return 候选摘要 {
                维度: 维度::执行,
                格位名: "工具".into(),
                摘要: String::new(),
                证据引用: Vec::new(),
                可信度: 0.7,
                模式: "规则".into(),
            };
        }
        候选摘要 {
            维度: 维度::执行,
            格位名: "工具".into(),
            摘要: format!("常用工具：{}", 命中.join("、")),
            证据引用: 图谱.技术栈.clone(),
            可信度: 0.7,
            模式: "规则".into(),
        }
    }
}
