use hm_contract::当前时间戳;
use crate::{维度, 心智地图, 上下文库, 消息角色};

/// 晋升记录：从临时上下文摘要晋升到格位的不可变记录（可追溯、可撤销）
#[derive(Clone, Debug, PartialEq)]
pub struct 晋升记录 {
    pub 源消息ids: Vec<u64>,
    /// 目标格位 (维度, 格位名)
    pub 目标格位: (维度, String),
    pub 摘要: String,
    pub 新可信度: f32,
    pub 时间: u64,
    pub 来源描述: String,
}

/// 默认压缩触发阈值（消息条数）
pub const 上下文压缩阈值: usize = 100;
/// 滑动窗口大小（压缩后保留的最近条数）
pub const 压缩滑动窗口: usize = 20;
/// 同类模式晋升阈值（重复 ≥ N 次晋升到经历·教训）
pub const 晋升重复阈值: u64 = 2;

/// 压缩器：三阶段压缩（滑动窗口 → 分层摘要 → 跨会话蒸馏晋升）
pub struct 压缩器<'a> {
    pub 上下文: &'a mut 上下文库,
    pub 心智: &'a mut 心智地图,
    /// 同类模式计数
    pub 模式计数: std::collections::HashMap<String, u64>,
}

impl<'a> 压缩器<'a> {
    pub fn 新(上下文: &'a mut 上下文库, 心智: &'a mut 心智地图) -> Self {
        压缩器 {
            上下文,
            心智,
            模式计数: std::collections::HashMap::new(),
        }
    }

    /// 追加消息，超阈值时自动压缩+晋升；返回晋升记录列表（None=未触发）
    pub fn 追加并自动压缩(
        &mut self,
        角色: 消息角色,
        内容: impl Into<String>,
    ) -> Option<Vec<晋升记录>> {
        self.上下文.追加(角色, 内容);
        if self.上下文.长度() >= 上下文压缩阈值 {
            Some(self.手动压缩())
        } else {
            None
        }
    }

    /// 执行一次三阶段压缩，返回晋升记录列表
    pub fn 手动压缩(&mut self) -> Vec<晋升记录> {
        let 消息 = self.上下文.全部().to_vec();
        if 消息.is_empty() {
            return Vec::new();
        }
        let mut 晋升: Vec<晋升记录> = Vec::new();
        if 消息.len() > 压缩滑动窗口 {
            let 保留数 = 压缩滑动窗口;
            let (丢弃, 保留) = 消息.split_at(消息.len() - 保留数);
            let 摘要 = self.阶段2分层摘要(丢弃);
            self.上下文.清空();
            self.上下文.追加(消息角色::系统, format!("[压缩摘要] {}", 摘要));
            for 消息 in 保留 {
                self.上下文.追加(消息.角色.clone(), 消息.内容.clone());
            }
            晋升.extend(self.阶段3蒸馏晋升(丢弃));
        }
        晋升
    }

    /// 阶段 2：分层摘要——保留决策点/错误原因/最终态，丢弃 verbose
    fn 阶段2分层摘要(&self, 消息: &[crate::上下文消息]) -> String {
        let mut 决策: Vec<String> = Vec::new();
        let mut 错误: Vec<String> = Vec::new();
        let mut 结果: Vec<String> = Vec::new();
        for 消息 in 消息 {
            let 内容: String = 消息.内容.chars().take(80).collect();
            if 消息.内容.contains("错误")
                || 消息.内容.contains("失败")
                || 消息.内容.contains("Error")
                || 消息.内容.contains("Exception")
            {
                错误.push(内容);
            } else if 消息.内容.contains("决策")
                || 消息.内容.contains("选择")
                || 消息.内容.contains("完成")
            {
                决策.push(内容);
            } else if matches!(消息.角色, 消息角色::助手 | 消息角色::工具结果) {
                结果.push(内容);
            }
        }
        let mut parts: Vec<String> = Vec::new();
        if !决策.is_empty() {
            parts.push(format!(
                "决策({})：{}",
                决策.len(),
                决策.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")
            ));
        }
        if !错误.is_empty() {
            parts.push(format!(
                "错误({})：{}",
                错误.len(),
                错误.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")
            ));
        }
        if !结果.is_empty() {
            parts.push(format!(
                "结果({})：{}",
                结果.len(),
                结果.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")
            ));
        }
        if parts.is_empty() {
            format!("压缩了 {} 条消息", 消息.len())
        } else {
            parts.join(" || ")
        }
    }

    /// 阶段 3：跨会话蒸馏——反复出现的模式晋升到「经历·教训」
    fn 阶段3蒸馏晋升(&mut self, 消息: &[crate::上下文消息]) -> Vec<晋升记录> {
        let mut 晋升: Vec<晋升记录> = Vec::new();
        for 消息 in 消息 {
            let 模式: String = 消息.内容.chars().take(30).collect();
            if 模式.trim().is_empty() {
                continue;
            }
            *self.模式计数.entry(模式.clone()).or_insert(0) += 1;
        }
        let 待晋升: Vec<(String, u64)> = self
            .模式计数
            .iter()
            .filter(|(_, 计数)| **计数 >= 晋升重复阈值)
            .map(|(模式, 计数)| (模式.clone(), *计数))
            .collect();
        for (模式, 计数) in 待晋升 {
            if let Some(记录) = self.晋升到经历教训(&模式, 计数) {
                晋升.push(记录);
            }
        }
        晋升
    }

    /// 晋升到「经历·教训」格位（可信度 0.7，覆盖式写入）
    fn 晋升到经历教训(&mut self, 模式: &str, 计数: u64) -> Option<晋升记录> {
        let 摘要: String = {
            let 前缀: String = 模式.chars().take(60).collect();
            format!("反复出现的模式（×{}）：{}", 计数, 前缀)
        };
        let 载荷 = crate::维度载荷::经历(crate::经历载荷 {
            触发器: format!("压缩阶段 3 检测到 {} 次", 计数),
            结果: 模式.chars().take(80).collect(),
            教训要点: 摘要.clone(),
            ..crate::经历载荷::default()
        });
        self.心智
            .写载荷(维度::经历, "教训", 摘要.clone(), 0.7, 载荷, Vec::new())
            .ok()?;
        Some(晋升记录 {
            源消息ids: Vec::new(),
            目标格位: (维度::经历, "教训".into()),
            摘要,
            新可信度: 0.7,
            时间: 当前时间戳(),
            来源描述: format!("压缩阶段 3 · 模式 {:?} 出现 {} 次", 模式, 计数),
        })
    }
}
