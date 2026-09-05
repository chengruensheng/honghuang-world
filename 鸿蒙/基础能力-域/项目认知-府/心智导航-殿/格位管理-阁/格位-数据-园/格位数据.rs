use serde::{Deserialize, Serialize};
use hm_error::{Error, Result};

/// 认知维度：心智地图的六个主轴，成三对正交（内↔外、规则↔执行、目标↔经历）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

    /// 是否「内」向维度（AI 自身）；外向维度（外在/规则）返回 false
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
}

/// 目标内容：仅「目标」维度格位持有的三时间态与偏移检测
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct 目标内容 {
    pub 初心: String,
    pub 现况: String,
    pub 愿景: String,
    pub 偏移: Option<String>,
}

/// 格位：项目心智导航地图的一个认知切片（提炼摘要，可错可纠）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct 格位 {
    pub 维度: 维度,
    pub 格位名: String,
    pub 摘要: String,
    pub 可信度: f32,
    pub 证据引用: Vec<String>,
    pub 最后校验时间: u64,
    pub 目标内容: Option<目标内容>,
}

/// 心智地图：按 6 维度组织的 36 格位（6×6）项目心智导航地图
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct 心智地图 {
    pub 格位集: Vec<格位>,
}

impl 心智地图 {
    pub fn 新() -> Self {
        心智地图::default()
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
}

/// 构造「维度·格位名」标识，用于错误信息
fn 格式键(维度: 维度, 格位名: &str) -> String {
    format!("{:?}·{}", 维度, 格位名)
}
