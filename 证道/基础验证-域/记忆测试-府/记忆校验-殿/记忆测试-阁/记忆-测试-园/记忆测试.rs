#[cfg(test)]
mod tests {
    use qk_memory::MemoryStore;

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("qk_memory_test_{名}.toml"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn 写入记忆后可查询() {
        let mut store = MemoryStore::new();
        let id = store.写入("第一条经验".into(), "经验".into()).unwrap();
        let m = store.查询(id).unwrap();
        assert_eq!(m.内容, "第一条经验");
        assert_eq!(m.标签, "经验");
    }

    #[test]
    fn 按标签查询() {
        let mut store = MemoryStore::new();
        store.写入("经验A".into(), "经验".into()).unwrap();
        store.写入("经验B".into(), "经验".into()).unwrap();
        store.写入("规则X".into(), "规则".into()).unwrap();
        assert_eq!(store.按标签("经验").len(), 2);
        assert_eq!(store.按标签("规则").len(), 1);
    }

    #[test]
    fn 查询不存在的记忆返回空() {
        let store = MemoryStore::new();
        assert!(store.查询(999).is_none());
    }

    #[test]
    fn 持久化往返() {
        let path = 临时路径("往返");
        let mut store = MemoryStore::new();
        store.写入("待持久化".into(), "经验".into()).unwrap();
        store.保存(&path).unwrap();
        let loaded = MemoryStore::加载(&path).unwrap();
        assert_eq!(loaded.全部().len(), 1);
        assert_eq!(loaded.查询(1).unwrap().内容, "待持久化");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 加载后id延续不冲突() {
        let path = 临时路径("延续");
        let mut store = MemoryStore::new();
        store.写入("一".into(), "经验".into()).unwrap();
        store.保存(&path).unwrap();
        let mut loaded = MemoryStore::加载(&path).unwrap();
        let new_id = loaded.写入("二".into(), "经验".into()).unwrap();
        assert_eq!(new_id, 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 设置持久化路径后写入自动落盘() {
        let path = 临时路径("自动");
        let _ = std::fs::remove_file(&path);
        let mut store = MemoryStore::new();
        store.设置持久化路径(path.clone());
        store.写入("自动记忆".into(), "经验".into()).unwrap();
        // 无需手动调用 保存，变更已自动落盘
        let loaded = MemoryStore::加载(&path).unwrap();
        assert_eq!(loaded.全部().len(), 1);
        assert_eq!(loaded.查询(1).unwrap().内容, "自动记忆");
        std::fs::remove_file(&path).ok();
    }
}
