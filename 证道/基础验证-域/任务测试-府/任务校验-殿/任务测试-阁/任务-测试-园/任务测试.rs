#[cfg(test)]
mod tests {
    use tc_task::TaskStatus;
    use tc_task::TaskStore;

    #[test]
    fn 任务诞生后状态为待受理() {
        let mut store = TaskStore::new();
        let id = store.创建("第一个任务".into(), "验证诞生".into()).unwrap();
        let task = store.查询(id).unwrap();
        assert_eq!(task.title, "第一个任务");
        assert_eq!(task.status, TaskStatus::待受理);
    }

    #[test]
    fn 合法状态推进成功() {
        let mut store = TaskStore::new();
        let id = store.创建("推进任务".into(), "".into()).unwrap();
        assert!(store.推进(id, TaskStatus::进行中).is_ok());
        assert!(store.推进(id, TaskStatus::已完成).is_ok());
        assert_eq!(store.查询(id).unwrap().status, TaskStatus::已完成);
    }

    #[test]
    fn 非法状态流转被拒绝() {
        let mut store = TaskStore::new();
        let id = store.创建("非法流转".into(), "".into()).unwrap();
        // 待受理 → 已完成 非法
        assert!(store.推进(id, TaskStatus::已完成).is_err());
        // 状态不变
        assert_eq!(store.查询(id).unwrap().status, TaskStatus::待受理);
    }

    #[test]
    fn 取消流转成功() {
        let mut store = TaskStore::new();
        let id = store.创建("取消任务".into(), "".into()).unwrap();
        assert!(store.推进(id, TaskStatus::进行中).is_ok());
        assert!(store.推进(id, TaskStatus::已取消).is_ok());
        assert_eq!(store.查询(id).unwrap().status, TaskStatus::已取消);
    }

    #[test]
    fn 查询不存在的任务返回空() {
        let store = TaskStore::new();
        assert!(store.查询(999).is_none());
    }

    #[test]
    fn 持久化往返() {
        let path = std::env::temp_dir()
            .join("tc_task_test_往返.toml")
            .to_string_lossy()
            .into_owned();
        let mut store = TaskStore::new();
        store.创建("待持久化".into(), "持久化验证".into()).unwrap();
        store.保存(&path).unwrap();
        let loaded = TaskStore::加载(&path).unwrap();
        assert_eq!(loaded.全部().len(), 1);
        assert_eq!(loaded.查询(1).unwrap().title, "待持久化");
        std::fs::remove_file(&path).ok();
    }
}
