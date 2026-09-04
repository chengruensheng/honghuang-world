use std::sync::Arc;
use hm_contract::Component;
use crate::信号定义_殿::信号;

/// 信号总线：跨府信号的中立发布/订阅契约（府可插拔，装配层注入实现）
pub trait 信号总线: Component {
    /// 发布信号：分发给所有匹配类型的订阅者，处理器在发布线程同步执行
    fn 发布(&self, 信号: &信号);
    /// 订阅某类型信号
    fn 订阅(&self, 类型: &str, 处理器: Arc<dyn Fn(&信号) + Send + Sync>);
}