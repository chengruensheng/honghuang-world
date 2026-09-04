use std::sync::Arc;
use hm_contract::Component;
use hm_error::Result;

/// 任务仓库契约：木之诞生与成长
///
/// 泛型参数解耦领域模型（任务/状态），使契约府不依赖具体引擎，
/// 生产路径注入真实实现，测试路径注入 mock，核心无特权。
pub trait 任务仓库契约<任务, 状态>: Component {
    /// 创建并登记任务，返回任务 id；待受理任务过盛（超容量上限）时拒绝创建
    fn 创建(&mut self, 标题: String, 描述: String) -> Result<u64>;
    /// 按 id 查询任务
    fn 查询(&self, id: u64) -> Option<&任务>;
    /// 列出全部任务
    fn 全部(&self) -> Vec<&任务>;
    /// 状态推进（非法流转返回错误）
    fn 推进(&mut self, id: u64, 状态: 状态) -> Result<()>;

}

/// 迭代日志契约：火之变革与演进
pub trait 迭代日志契约<迭代, 版本>: Component {
    /// 开启迭代，返回迭代 id；进行中迭代过盛（超容量上限）时拒绝开启
    fn 开启(&mut self, 版本: 版本, 变更说明: String) -> Result<u64>;
    /// 完成迭代
    fn 完成(&mut self, id: u64) -> Result<()>;
    /// 放弃迭代
    fn 放弃(&mut self, id: u64) -> Result<()>;
    /// 按 id 查询迭代
    fn 查询(&self, id: u64) -> Option<&迭代>;
    /// 演进历史（全部迭代）
    fn 全部(&self) -> Vec<&迭代>;
    /// 当前版本（最新已完成迭代的版本）
    fn 当前版本(&self) -> 版本;

}

/// 记忆库契约：土之承载
pub trait 记忆库契约<记忆>: Component {
    /// 写入记忆，返回记忆 id；失败返回错误
    fn 写入(&mut self, 内容: String, 标签: String) -> Result<u64>;
    /// 按 id 查询
    fn 查询(&self, id: u64) -> Option<&记忆>;
    /// 按标签查询
    fn 按标签(&self, 标签: &str) -> Vec<&记忆>;
    /// 全部记忆
    fn 全部(&self) -> Vec<&记忆>;

}

/// 规则库契约：金之收敛
pub trait 规则库契约<规则>: Component {
    /// 添加规则（同名拒绝），返回规则 id
    fn 添加规则(&mut self, 名称: &str, 条件: Vec<(String, String)>, 结论: &str, 优先级: u32) -> Result<u64>;
    /// 按名称查询
    fn 按名称(&self, 名称: &str) -> Option<&规则>;
    /// 全部规则
    fn 全部(&self) -> Vec<&规则>;
    /// 评估事实，按优先级降序返回命中的规则
    fn 评估(&self, 事实: &[(String, String)]) -> Vec<&规则>;

}

/// 事件总线契约：水之流动
pub trait 事件总线契约<事件>: Component {
    /// 订阅某类型事件
    fn 订阅(&mut self, 类型: &str, 处理器: Arc<dyn Fn(&事件) + Send + Sync>);
    /// 发布事件，返回事件 id
    fn 发布(&mut self, 类型: String, 载荷: Vec<(String, String)>) -> u64;
    /// 按 id 查询
    fn 查询(&self, id: u64) -> Option<&事件>;
    /// 事件历史（按发布顺序）
    fn 全部(&self) -> Vec<&事件>;
    /// 去重事件（土克水：记忆固化事件流），按类型与载荷合并重复事件，返回移除数量
    fn 去重(&mut self) -> usize;
}