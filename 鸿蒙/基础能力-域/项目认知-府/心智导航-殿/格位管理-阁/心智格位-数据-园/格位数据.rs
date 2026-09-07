use serde::{Deserialize, Serialize};
use hm_error::{Error, Result};
use hm_contract::当前时间戳;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 三态认知 · 认知常量（对齐 Python 原型 类型定义.py）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 被纠错格位强制先查图谱的冷却轮数（1 轮 ≈ 1 分钟）
pub const 冷却默认轮数: u64 = 5;
/// 规则维度摘要字符上限
pub const 规则摘要上限: usize = 80;
/// 目标/经历维度摘要字符上限
pub const 长摘要上限: usize = 150;
/// 其余维度摘要字符上限
pub const 中等摘要上限: usize = 200;

/// 维度置信度阈值（按维度；规则类错答代价高，宁可拒）
pub fn 维度置信度阈值(维度值: 维度) -> f32 {
    match 维度值 {
        维度::规则 => 0.95,
        维度::外在 | 维度::执行 => 0.75,
        维度::目标 => 0.80,
        维度::内部 => 0.70,
        维度::经历 => 0.65,
    }
}

/// 摘要字符上限（按维度）
pub fn 摘要上限(维度值: 维度) -> usize {
    match 维度值 {
        维度::规则 => 规则摘要上限,
        维度::目标 | 维度::经历 => 长摘要上限,
        _ => 中等摘要上限,
    }
}

/// 时效新鲜阈值（秒）：规则变化慢（30 天）；外在/执行 7 天；其余 2 天
pub fn 维度新鲜阈值秒(维度值: 维度) -> u64 {
    match 维度值 {
        维度::规则 => 30 * 24 * 3600,
        维度::外在 | 维度::执行 => 7 * 24 * 3600,
        _ => 2 * 24 * 3600,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 6 维度（主轴，三对正交）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 认知维度：心智地图的六个主轴（内↔外、规则↔执行、目标↔经历）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum 维度 {
    内部,
    外在,
    规则,
    执行,
    目标,
    经历,
}

impl 维度 {
    /// 全部六个维度，用于遍历构建心智地图
    pub fn 全部维度() -> [维度; 6] {
        [
            维度::内部,
            维度::外在,
            维度::规则,
            维度::执行,
            维度::目标,
            维度::经历,
        ]
    }

    /// 是否「内」向维度（AI 自身）
    pub fn 是否内向(&self) -> bool {
        matches!(
            self,
            维度::内部 | 维度::执行 | 维度::目标 | 维度::经历
        )
    }

    /// 返回本维度的正交对（本方, 对方）
    pub fn 正交对(&self) -> (维度, 维度) {
        match self {
            维度::内部 => (维度::内部, 维度::外在),
            维度::外在 => (维度::外在, 维度::内部),
            维度::规则 => (维度::规则, 维度::执行),
            维度::执行 => (维度::执行, 维度::规则),
            维度::目标 => (维度::目标, 维度::经历),
            维度::经历 => (维度::经历, 维度::目标),
        }
    }

    /// 中文名（用于路由权重与错误信息）
    pub fn 中文名(&self) -> &'static str {
        match self {
            维度::内部 => "内部",
            维度::外在 => "外在",
            维度::规则 => "规则",
            维度::执行 => "执行",
            维度::目标 => "目标",
            维度::经历 => "经历",
        }
    }

    /// 路由权重：规则/目标略高
    pub fn 路由权重(&self) -> f32 {
        match self {
            维度::规则 => 1.2,
            维度::目标 => 1.15,
            维度::外在 | 维度::执行 => 1.1,
            维度::经历 => 1.05,
            维度::内部 => 1.0,
        }
    }
}

/// 规则类格位的严重度分级
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum 严重度 {
    提示,
    警告,
    禁止,
    红线,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 维度载荷变体（统一骨架 + 维度变体，对齐原型）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 目标载荷：仅「目标」维度格位持有（三时间态 + 度量 + 依赖）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 目标载荷 {
    pub 初心: String,
    pub 现况: String,
    pub 愿景: String,
    pub 偏移: Option<String>,
    pub 度量: String,
    pub 依赖: Vec<String>,
}

/// 执行载荷：「执行」维度格位持有（稳定规范入格位，单次实例入临时）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 执行载荷 {
    pub 标准命令: Vec<String>,
    pub 标准步骤: Vec<String>,
    pub 工具清单: Vec<String>,
    pub 产出契约: String,
    pub 最近实例指针: Option<u64>,
}

/// 规则载荷：「规则」维度格位持有
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 规则载荷 {
    pub 触发条件: String,
    pub 严重度: 严重度,
    #[serde(default)]
    pub 例外条款: Vec<String>,
    #[serde(default)]
    pub 历史违反次数: u64,
}

impl Default for 规则载荷 {
    fn default() -> Self {
        规则载荷 {
            触发条件: String::new(),
            严重度: 严重度::警告,
            例外条款: Vec::new(),
            历史违反次数: 0,
        }
    }
}

/// 外在载荷：「外在」维度格位持有（多数被图谱覆盖，多为占位）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 外在载荷 {
    pub 节点路径: String,
    pub 入口: String,
    pub 数据形状: String,
}

/// 内部载荷：「内部」维度格位持有（AI 自指，最小化）
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 内部载荷 {
    pub 自评语句: String,
}

/// 经历载荷：「经历」维度格位持有
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 经历载荷 {
    pub 时间: Option<u64>,
    pub 触发器: String,
    pub 结果: String,
    pub 教训要点: String,
}

/// 维度载荷：按维度选择对应变体（统一骨架 + 维度变体）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 维度载荷 {
    目标(目标载荷),
    执行(执行载荷),
    规则(规则载荷),
    外在(外在载荷),
    内部(内部载荷),
    经历(经历载荷),
}

/// 按维度返回对应空变体
pub fn 空载荷(维度值: 维度) -> 维度载荷 {
    match 维度值 {
        维度::目标 => 维度载荷::目标(目标载荷::default()),
        维度::执行 => 维度载荷::执行(执行载荷::default()),
        维度::规则 => 维度载荷::规则(规则载荷::default()),
        维度::外在 => 维度载荷::外在(外在载荷::default()),
        维度::内部 => 维度载荷::内部(内部载荷::default()),
        维度::经历 => 维度载荷::经历(经历载荷::default()),
    }
}

/// 校验维度与载荷变体匹配；不匹配返回错误
pub fn 校验载荷维度(维度值: 维度, 载荷: &维度载荷) -> Result<()> {
    let 匹配 = matches!(
        (维度值, 载荷),
        (维度::目标, 维度载荷::目标(_))
            | (维度::执行, 维度载荷::执行(_))
            | (维度::规则, 维度载荷::规则(_))
            | (维度::外在, 维度载荷::外在(_))
            | (维度::内部, 维度载荷::内部(_))
            | (维度::经历, 维度载荷::经历(_))
    );
    if 匹配 {
        Ok(())
    } else {
        Err(Error::Other(format!(
            "维度 {:?} 与载荷变体不匹配",
            维度值
        )))
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 格位（统一骨架 + 维度变体）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 格位：项目心智导航地图的一个认知切片（提炼摘要，可错可纠）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 格位 {
    pub 维度: 维度,
    pub 格位名: String,
    pub 摘要: String,
    pub 可信度: f32,
    pub 证据引用: Vec<String>,
    pub 最后校验时间: u64,
    /// 冷却期截止时间戳；Some(截止) 且当前时间早于截止时处于冷却期
    #[serde(default)]
    pub 冷却期至: Option<u64>,
    /// 维度载荷变体；None 表示未填载荷（仅骨架）
    #[serde(default)]
    pub 维度载荷: Option<维度载荷>,
}

impl 格位 {
    /// 构造格位：摘要按维度上限截断、可信度夹紧 0..=1
    pub fn 新(
        维度值: 维度,
        格位名: impl Into<String>,
        摘要: impl Into<String>,
        可信度: f32,
        证据引用: Vec<String>,
        维度载荷: Option<维度载荷>,
    ) -> Self {
        let 摘要 = 摘要.into();
        let 上限 = 摘要上限(维度值);
        let 摘要 = if 摘要.chars().count() > 上限 {
            摘要.chars().take(上限).collect()
        } else {
            摘要
        };
        格位 {
            维度: 维度值,
            格位名: 格位名.into(),
            摘要,
            可信度: 可信度.clamp(0.0, 1.0),
            证据引用,
            最后校验时间: 当前时间戳(),
            冷却期至: None,
            维度载荷,
        }
    }

    /// 是否处于冷却期（按给定当前时间判定，便于测试）
    pub fn 处于冷却期(&self, 当前时间: u64) -> bool {
        matches!(self.冷却期至, Some(截止) if 当前时间 < 截止)
    }
}

/// 多信号「够用」判定聚合结果（S1-S5）
#[derive(Clone, Debug, Default, PartialEq)]
pub struct 够用信号 {
    pub 结构完整: bool,
    pub 置信度达标: bool,
    pub 时效新鲜: bool,
    pub 证据存在: bool,
    pub 未在冷却期: bool,
    pub 不达标项: Vec<String>,
}

impl 够用信号 {
    /// 五项全部满足才算够用
    pub fn 够用(&self) -> bool {
        self.结构完整
            && self.置信度达标
            && self.时效新鲜
            && self.证据存在
            && self.未在冷却期
    }
}
