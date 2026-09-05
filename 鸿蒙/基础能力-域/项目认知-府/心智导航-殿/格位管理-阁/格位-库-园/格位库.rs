use serde::{Deserialize, Serialize};
use hm_contract::当前时间戳;
use hm_error::{Error, Result};
use crate::{维度, 格位, 够用信号, 维度载荷, 空载荷, 校验载荷维度, 维度置信度阈值, 维度新鲜阈值秒};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 36 格位清单（6 × 6，对齐设计文档第四节）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 每维度的 6 个固定格位名
pub fn 维度格位名(维度值: 维度) -> &'static [&'static str; 6] {
    match 维度值 {
        维度::内部 => &["角色", "能力", "职责", "状态", "记忆", "边界"],
        维度::外在 => &["定位", "结构", "依赖", "接口", "环境", "数据"],
        维度::规则 => &["禁止", "门禁", "规范", "流程", "例外", "红线"],
        维度::执行 => &["命令", "技术栈", "步骤", "工具", "验证", "产出"],
        维度::目标 => &["初心", "现况", "愿景", "偏移", "度量", "依赖"],
        维度::经历 => &["事件", "教训", "决策", "来源", "时间", "归档"],
    }
}

/// 构造 36 个全空格位的骨架（摘要空、可信度 0、无证据、无载荷）
pub fn 默认三十六格位() -> Vec<格位> {
    let mut 全部 = Vec::with_capacity(36);
    for 维度值 in 维度::全部维度() {
        for 名 in 维度格位名(维度值) {
            全部.push(格位::新(维度值, *名, "", 0.0, Vec::new(), Some(空载荷(维度值))));
        }
    }
    全部
}

/// 简易分词：英文词 + 中文 1-4 字滑动窗（对齐原型 格位库.py）
fn 分词(文本: &str) -> Vec<String> {
    let mut 词 = Vec::new();
    // 英文/数字词
    for 段 in 文本.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-') {
        let 段 = 段.trim_matches('_').trim_matches('-');
        if 段.len() >= 3 {
            词.push(段.to_lowercase());
            for 子 in 段.split(['_', '-']) {
                if 子.len() >= 3 && !词.iter().any(|w| w == 子) {
                    词.push(子.to_lowercase());
                }
            }
        }
    }
    // 中文滑动窗（1-4 字）
    let mut 中文段 = String::new();
    for ch in 文本.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            中文段.push(ch);
        } else if !中文段.is_empty() {
            for 长度 in 1..=4 {
                for i in 0..=中文段.chars().count().saturating_sub(长度) {
                    let 切片: String = 中文段.chars().skip(i).take(长度).collect();
                    if !词.iter().any(|w| w == &切片) {
                        词.push(切片);
                    }
                }
            }
            中文段.clear();
        }
    }
    if !中文段.is_empty() {
        for 长度 in 1..=4 {
            for i in 0..=中文段.chars().count().saturating_sub(长度) {
                let 切片: String = 中文段.chars().skip(i).take(长度).collect();
                if !词.iter().any(|w| w == &切片) {
                    词.push(切片);
                }
            }
        }
    }
    词
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 心智地图：36 格位的内存存储 + 够用判定 + 路由
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 心智地图：按 6 维度组织的 36 格位（6×6）项目心智导航地图
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct 心智地图 {
    pub 格位集: Vec<格位>,
}

impl 心智地图 {
    /// 心智地图默认即 36 格位骨架（格位常驻：三态设计心智态）
    pub fn 新() -> Self {
        心智地图 {
            格位集: 默认三十六格位(),
        }
    }

    /// 空地图（不预置格位），用于测试与渐进装配
    pub fn 空() -> Self {
        心智地图 { 格位集: vec![] }
    }

    /// 构造 36 格位骨架（与 新 等价，语义更明确）
    pub fn 默认三十六格位() -> Self {
        心智地图::新()
    }

    /// 添加格位；同「维度+格位名」拒绝，避免语义重复
    pub fn 添加格位(&mut self, 格位: 格位) -> Result<()> {
        if self.查询格位(格位.维度, &格位.格位名).is_some() {
            return Err(Error::格位已存在(格式键(格位.维度, &格位.格位名)));
        }
        self.格位集.push(格位);
        Ok(())
    }

    /// 按「维度+格位名」查询格位；不存在返回 None
    pub fn 查询格位(&self, 维度: 维度, 格位名: &str) -> Option<&格位> {
        self.格位集
            .iter()
            .find(|格位| 格位.维度 == 维度 && 格位.格位名 == 格位名)
    }

    /// 单点修正：替换同「维度+格位名」格位（保留最新）；不存在则报错
    pub fn 更新格位(&mut self, 格位: 格位) -> Result<()> {
        match self
            .格位集
            .iter_mut()
            .find(|旧| 旧.维度 == 格位.维度 && 旧.格位名 == 格位.格位名)
        {
            Some(旧格位) => {
                *旧格位 = 格位;
                Ok(())
            }
            None => Err(Error::格位不存在(格式键(格位.维度, &格位.格位名))),
        }
    }

    /// 按维度列出格位
    pub fn 按维度查询(&self, 维度: 维度) -> Vec<&格位> {
        self.格位集
            .iter()
            .filter(|格位| 格位.维度 == 维度)
            .collect()
    }

    /// 列出全部格位
    pub fn 全部(&self) -> &[格位] {
        &self.格位集
    }

    // ━━━━━━━━━━━━━━ 写（对齐原型 格位库.py） ━━━━━━━━━━━━━━

    /// 写入或更新格位：摘要截断、可信度夹紧、刷新校验时间
    pub fn 写(
        &mut self,
        维度值: 维度,
        格位名: &str,
        摘要: impl Into<String>,
        可信度: f32,
        证据引用: Vec<String>,
    ) -> Result<格位> {
        self.写载荷(维度值, 格位名, 摘要, 可信度, 空载荷(维度值), 证据引用)
    }

    /// 写入带维度载荷变体的格位；载荷维度不匹配返回错误
    pub fn 写载荷(
        &mut self,
        维度值: 维度,
        格位名: &str,
        摘要: impl Into<String>,
        可信度: f32,
        载荷: 维度载荷,
        证据引用: Vec<String>,
    ) -> Result<格位> {
        校验载荷维度(维度值, &载荷)?;
        if self.查询格位(维度值, 格位名).is_none() {
            return Err(Error::格位不存在(格式键(维度值, 格位名)));
        }
        let 新格位 = 格位::新(维度值, 格位名, 摘要, 可信度, 证据引用, Some(载荷));
        self.更新格位(新格位.clone())?;
        Ok(新格位)
    }

    /// 让指定格位进入冷却期（强制先查图谱的轮数；1 轮 ≈ 1 分钟）
    pub fn 进入冷却期(&mut self, 维度值: 维度, 格位名: &str, 轮数: u64) -> Result<格位> {
        let 键 = 格式键(维度值, 格位名);
        let 格位 = self
            .查询格位(维度值, 格位名)
            .cloned()
            .ok_or_else(|| Error::格位不存在(键.clone()))?;
        let 截止 = 当前时间戳() + 轮数 * 60;
        let 新格位 = 格位 { 冷却期至: Some(截止), ..格位 };
        self.更新格位(新格位.clone())?;
        Ok(新格位)
    }

    // ━━━━━━━━━━━━━━ 多信号够用判定（S1-S5） ━━━━━━━━━━━━━━

    /// 多信号够用判定：结构 / 置信度 / 时效 / 证据 / 冷却期
    pub fn 够用(&self, 维度值: 维度, 格位名: &str) -> 够用信号 {
        let 当前 = 当前时间戳();
        let Some(格位) = self.查询格位(维度值, 格位名) else {
            return 够用信号 {
                不达标项: vec!["格位不存在".into()],
                ..够用信号::default()
            };
        };
        let s1 = !格位.摘要.is_empty() && 格位.可信度 > 0.0;
        let s2 = 格位.可信度 >= 维度置信度阈值(维度值);
        let s3 = 当前.saturating_sub(格位.最后校验时间) <= 维度新鲜阈值秒(维度值);
        let s4 = !格位.证据引用.is_empty();
        let s5 = !格位.处于冷却期(当前);

        let mut 不达标项 = Vec::new();
        if !s1 {
            不达标项.push("S1结构不完整".into());
        }
        if !s2 {
            不达标项.push(format!(
                "S2置信度{:.2}<{:.2}",
                格位.可信度,
                维度置信度阈值(维度值)
            ));
        }
        if !s3 {
            不达标项.push(format!(
                "S3超过新鲜阈值{}天",
                维度新鲜阈值秒(维度值) / 86400
            ));
        }
        if !s4 {
            不达标项.push("S4无证据引用".into());
        }
        if !s5 {
            不达标项.push("S5处于冷却期".into());
        }

        够用信号 {
            结构完整: s1,
            置信度达标: s2,
            时效新鲜: s3,
            证据存在: s4,
            未在冷却期: s5,
            不达标项,
        }
    }

    // ━━━━━━━━━━━━━━ 路由（对齐原型 格位库.py） ━━━━━━━━━━━━━━

    /// 基于关键词权重把问题路由到候选格位（分数 = 摘要命中 + 名命中×1.5 + 维度命中×1.2，×维度权重）
    pub fn 路由(&self, 问题: &str, 最大: usize) -> Vec<(维度, String)> {
        let 词 = 分词(问题);
        let mut 评分: Vec<(f32, 维度, String)> = Vec::new();
        for 格位 in &self.格位集 {
            let 名命中 = 词.iter().filter(|w| 格位.格位名.contains(&**w)).count();
            let 摘要命中 = 词.iter().filter(|w| 格位.摘要.contains(&**w)).count();
            let 维命中 = if 词.iter().any(|w| 格位.维度.中文名().contains(w)) {
                1
            } else {
                0
            };
            if 名命中 + 摘要命中 + 维命中 == 0 {
                continue;
            }
            let 分 = (摘要命中 as f32 + 名命中 as f32 * 1.5 + 维命中 as f32 * 1.2)
                * 格位.维度.路由权重();
            评分.push((分, 格位.维度, 格位.格位名.clone()));
        }
        评分.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        评分
            .into_iter()
            .take(最大)
            .map(|(_, 维度, 名)| (维度, 名))
            .collect()
    }

    /// 格位填充状态统计
    pub fn 统计(&self) -> (usize, usize, usize, usize) {
        let 当前 = 当前时间戳();
        let 总 = self.格位集.len();
        let 已填 = self
            .格位集
            .iter()
            .filter(|格位| !格位.摘要.is_empty())
            .count();
        let 够用 = self
            .格位集
            .iter()
            .filter(|格位| self.够用(格位.维度, &格位.格位名).够用())
            .count();
        let 冷却中 = self
            .格位集
            .iter()
            .filter(|格位| 格位.处于冷却期(当前))
            .count();
        (总, 已填, 够用, 冷却中)
    }
}

/// 构造「维度·格位名」标识，用于错误信息
fn 格式键(维度: 维度, 格位名: &str) -> String {
    format!("{:?}·{}", 维度, 格位名)
}
