use std::sync::{Arc, Mutex};
use hm_contract::Component;
use hm_signal::{信号, 信号总线};

/// 内存信号总线：订阅关系存内存，发布时分发给匹配类型的订阅者
///
/// 处理器在订阅者锁释放后执行，因此处理器内部再发布信号是重入安全的，
/// 不会因嵌套锁而死锁。
pub struct 内存信号总线 {
    subscribers: Mutex<Vec<(String, Arc<dyn Fn(&信号) + Send + Sync>)>>,
}

impl 内存信号总线 {
    pub fn new() -> Self {
        内存信号总线 {
            subscribers: Mutex::new(Vec::new()),
        }
    }
}

impl Component for 内存信号总线 {
    fn name(&self) -> &'static str { "信号总线" }
}

impl 信号总线 for 内存信号总线 {
    fn 发布(&self, 信号: &信号) {
        let handlers: Vec<Arc<dyn Fn(&信号) + Send + Sync>> = {
            let subs = self.subscribers.lock().expect("订阅表锁中毒");
            subs.iter()
                .filter(|(t, _)| t == &信号.类型)
                .map(|(_, h)| h.clone())
                .collect()
        };
        for h in handlers {
            h(信号);
        }
    }

    fn 订阅(&self, 类型: &str, 处理器: Arc<dyn Fn(&信号) + Send + Sync>) {
        self.subscribers
            .lock()
            .expect("总线订阅锁中毒")
            .push((类型.to_string(), 处理器));
    }
}