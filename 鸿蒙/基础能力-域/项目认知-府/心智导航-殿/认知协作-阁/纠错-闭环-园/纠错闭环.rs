use hm_contract::当前时间戳;
use crate::{维度, 图谱, 心智地图, 上下文库, 维度载荷, 经历载荷, 消息角色, 冷却默认轮数, 晋升重复阈值};

/// 纠错事件：一次纠错的不可变记录（可追溯回放）
#[derive(Clone, Debug, PartialEq)]
pub struct 纠错事件 {
    pub 时间: u64,
    pub 检测到的差异: String,
    /// 受影响格位（维度·格位名）
    pub 受影响格位: Vec<String>,
    pub 新摘要: String,
    pub 新可信度: f32,
    /// 教训触发描述（同类纠错 ≥ 阈值时生成）
    pub 教训触发: Option<String>,
}

/// 纠错引擎：七步纠错闭环执行器
///
/// 步骤：1 检测 → 2 定位 → 3 暂存旧值（经历·事件）→ 4 写入新摘要（置信度降至 0.7）
///       → 5 同类 ≥ 阈值生成教训 → 6 通知上游（引用方下调，暂留接口）→ 7 进入冷却期
pub struct 纠错引擎<'a> {
    pub 图谱: &'a 图谱,
    pub 心智: &'a mut 心智地图,
    pub 上下文: &'a mut 上下文库,
    /// 事件序列（可回放审计）
    pub 事件序列: Vec<纠错事件>,
    /// 同类纠错计数：(维度, 格位名, 关键词)
    pub 同类计数: std::collections::HashMap<(维度, String, String), u64>,
}

impl<'a> 纠错引擎<'a> {
    pub fn 新(图谱: &'a 图谱, 心智: &'a mut 心智地图, 上下文: &'a mut 上下文库) -> Self {
        纠错引擎 {
            图谱,
            心智,
            上下文,
            事件序列: Vec::new(),
            同类计数: std::collections::HashMap::new(),
        }
    }

    /// 执行七步纠错闭环，返回纠错事件记录
    pub fn 纠正(
        &mut self,
        维度值: 维度,
        格位名: &str,
        新摘要: impl Into<String>,
        新证据: Vec<String>,
        差异描述: impl Into<String>,
    ) -> 纠错事件 {
        let 新摘要 = 新摘要.into();
        let 旧摘要 = self
            .心智
            .查询格位(维度值, 格位名)
            .map(|格位| 格位.摘要.clone())
            .unwrap_or_default();
        let 旧可信度 = self
            .心智
            .查询格位(维度值, 格位名)
            .map(|格位| 格位.可信度)
            .unwrap_or(0.0);
        let 描述 = 差异描述.into();
        let 差异 = if 描述.is_empty() {
            let 旧: String = 旧摘要.chars().take(40).collect();
            let 新: String = 新摘要.chars().take(40).collect();
            format!("摘要差异：旧={:?} 新={:?}", 旧, 新)
        } else {
            描述
        };

        // 步骤 3：暂存旧值到「经历·事件」（信号消息）
        self.上下文.追加(
            消息角色::信号,
            format!("[纠错·{}·{}] {}", 维度值.中文名(), 格位名, 差异),
        );

        // 步骤 4：写入新摘要，置信度降至 0.7（不高于旧值）
        let 新可信度 = 旧可信度.min(0.7);
        let _ = self
            .心智
            .写(维度值, 格位名, 新摘要.clone(), 新可信度, 新证据);

        // 步骤 5：同类纠错 ≥ 阈值 → 生成「经历·教训」
        let 关键词: String = 新摘要.chars().take(8).collect();
        let 键 = (维度值, 格位名.to_string(), 关键词.clone());
        let 计数 = self.同类计数.entry(键).or_insert(0);
        *计数 += 1;
        let 教训触发 = if *计数 >= 晋升重复阈值 {
            let 教训 = format!("同类纠错（{}·{}）重复 {} 次：{}", 维度值.中文名(), 格位名, 计数, 关键词);
            let 载荷 = 维度载荷::经历(经历载荷 {
                触发器: format!("纠错步骤 5 · 同类 {} 次", 计数),
                结果: 新摘要.clone(),
                教训要点: 教训.clone(),
                ..经历载荷::default()
            });
            let _ = self
                .心智
                .写载荷(维度::经历, "教训", 教训.clone(), 0.7, 载荷, Vec::new());
            Some(教训)
        } else {
            None
        };

        // 步骤 7：进入冷却期（强制先查图谱）
        let _ = self.心智.进入冷却期(维度值, 格位名, 冷却默认轮数);

        let 事件 = 纠错事件 {
            时间: 当前时间戳(),
            检测到的差异: 差异,
            受影响格位: vec![format!("{}·{}", 维度值.中文名(), 格位名)],
            新摘要,
            新可信度,
            教训触发,
        };
        self.事件序列.push(事件.clone());
        事件
    }
}
