#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use hd_event::EventBus;

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("hd_event_test_{名}.toml"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn 发布事件后历史记录() {
        let mut bus = EventBus::new();
        let id = bus.发布("内存变更".into(), vec![("键".into(), "值".into())]);
        assert_eq!(bus.查询(id).unwrap().类型, "内存变更");
        assert_eq!(bus.全部().len(), 1);
    }

    #[test]
    fn 按类型订阅分发() {
        let mut bus = EventBus::new();
        let 计数 = Arc::new(Mutex::new(0));
        let c = 计数.clone();
        bus.订阅("内存变更", Arc::new(move |_e| { *c.lock().unwrap() += 1; }));
        bus.发布("内存变更".into(), vec![]);
        bus.发布("内存变更".into(), vec![]);
        assert_eq!(*计数.lock().unwrap(), 2);
    }

    #[test]
    fn 不匹配类型的处理器不被调用() {
        let mut bus = EventBus::new();
        let 计数 = Arc::new(Mutex::new(0));
        let c = 计数.clone();
        bus.订阅("规则命中", Arc::new(move |_e| { *c.lock().unwrap() += 1; }));
        bus.发布("内存变更".into(), vec![]);
        assert_eq!(*计数.lock().unwrap(), 0);
    }

    #[test]
    fn 事件按发布顺序记录() {
        let mut bus = EventBus::new();
        bus.发布("甲".into(), vec![]);
        bus.发布("乙".into(), vec![]);
        let 历史 = bus.全部();
        assert_eq!(历史[0].类型, "甲");
        assert_eq!(历史[1].类型, "乙");
    }

    #[test]
    fn 持久化往返() {
        let path = 临时路径("往返");
        let mut bus = EventBus::new();
        bus.发布("内存变更".into(), vec![("键".into(), "值".into())]);
        bus.保存(&path).unwrap();
        let loaded = EventBus::加载(&path).unwrap();
        assert_eq!(loaded.全部().len(), 1);
        assert_eq!(loaded.全部()[0].类型, "内存变更");
        assert_eq!(loaded.全部()[0].载荷, vec![("键".to_string(), "值".to_string())]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 加载后id延续不冲突() {
        let path = 临时路径("延续");
        let mut bus = EventBus::new();
        bus.发布("一".into(), vec![]);
        bus.保存(&path).unwrap();
        let mut loaded = EventBus::加载(&path).unwrap();
        let new_id = loaded.发布("二".into(), vec![]);
        assert_eq!(new_id, 2);
        std::fs::remove_file(&path).ok();
    }
}