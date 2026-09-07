use serde::{Deserialize, Serialize};
use uuid::Uuid;
use hm_contract::当前时间戳;
use crate::任务模型_殿::{五行层级, 层级记录, 产物记录};

/// 任务唯一标识：UUID v4 + 任务名 + 发布时间戳 + 父任务/依赖任务。
///
/// 唯一可追溯：每个任务从诞生起即持有标识，层级处理痕迹与产物按标识关联。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct 任务标识 {
    /// UUID v4（主键演进方向；过渡期与现有序号 id 并存）
    pub 任务id: Uuid,
    pub 任务名: String,
    /// 发布时间（秒级时间戳，与项目时间体系一致）
    pub 发布时间: u64,
    /// 父任务（子任务派生自父任务时记录）
    pub 父任务id: Option<Uuid>,
    /// 依赖任务（本任务完成需先完成的其它任务）
    pub 依赖任务: Vec<Uuid>,
}

impl 任务标识 {
    /// 生成：自动分配 UUID v4 + 当前时间戳
    pub fn 生成(任务名: impl Into<String>, 父任务id: Option<Uuid>, 依赖任务: Vec<Uuid>) -> Self {
        任务标识 {
            任务id: Uuid::new_v4(),
            任务名: 任务名.into(),
            发布时间: 当前时间戳(),
            父任务id,
            依赖任务,
        }
    }

    /// 标识是否已初始化（未初始化的标识任务id 为空 UUID）
    pub fn 未初始化(&self) -> bool {
        self.任务id.is_nil()
    }

    /// 追溯路径：返回任务经过的层级处理路径（按时间顺序，含已回退的层）
    pub fn 追溯路径(&self, 层级历史: &[层级记录]) -> Vec<五行层级> {
        层级历史.iter().map(|记录| 记录.层级).collect()
    }

    /// 查找产物：按文件路径查出产出该文件的层级记录（先到优先，通常取最新）
    pub fn 查找产物<'a>(&self, 层级历史: &'a [层级记录], 文件路径: &str) -> Option<&'a 产物记录> {
        层级历史
            .iter()
            .rev()
            .find_map(|记录| 记录.产物.iter().find(|产物| 产物.文件路径 == 文件路径))
    }
}

impl Default for 任务标识 {
    /// 空标识：任务ID 为空 UUID（加载旧数据或手动构造时的默认态，发布时补生成）
    fn default() -> Self {
        任务标识 {
            任务id: Uuid::nil(),
            任务名: String::new(),
            发布时间: 0,
            父任务id: None,
            依赖任务: Vec::new(),
        }
    }
}
