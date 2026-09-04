use serde::{Deserialize, Serialize};

/// 事件：水之流动，一次状态变化的记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub 类型: String,
    /// 载荷：键值对列表，事件携带的数据（与规则引擎的事实表示一致）
    pub 载荷: Vec<(String, String)>,
    pub created_at: u64,
}

impl Event {
    pub fn 新建(id: u64, 类型: String, 载荷: Vec<(String, String)>, created_at: u64) -> Self {
        Event { id, 类型, 载荷, created_at }
    }
}
