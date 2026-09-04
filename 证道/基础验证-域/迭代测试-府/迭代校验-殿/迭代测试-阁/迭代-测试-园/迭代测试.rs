#[cfg(test)]
mod tests {
    use lj_iteration::Version;
    use lj_iteration::IterationLog;
    use lj_iteration::迭代状态;

    #[test]
    fn 版本递增正确() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.递增主(), Version::new(2, 0, 0));
        assert_eq!(v.递增次(), Version::new(1, 3, 0));
        assert_eq!(v.递增修订(), Version::new(1, 2, 4));
    }

    #[test]
    fn 版本显示格式() {
        assert_eq!(Version::new(1, 2, 3).to_string(), "1.2.3");
    }

    #[test]
    fn 开启迭代后状态为进行中() {
        let mut log = IterationLog::new();
        let id = log.开启(Version::new(1, 0, 0), "首个版本".into()).unwrap();
        assert_eq!(log.查询(id).unwrap().status, 迭代状态::进行中);
    }

    #[test]
    fn 完成迭代后当前版本更新() {
        let mut log = IterationLog::new();
        let id = log.开启(Version::new(1, 0, 0), "首个版本".into()).unwrap();
        assert!(log.完成(id).is_ok());
        assert_eq!(log.当前版本(), Version::new(1, 0, 0));
    }

    #[test]
    fn 放弃迭代不影响当前版本() {
        let mut log = IterationLog::new();
        let id = log.开启(Version::new(1, 0, 0), "废弃版本".into()).unwrap();
        assert!(log.放弃(id).is_ok());
        assert_eq!(log.当前版本(), Version::new(0, 0, 0));
    }

    #[test]
    fn 重复完成已完成的迭代被拒绝() {
        let mut log = IterationLog::new();
        let id = log.开启(Version::new(1, 0, 0), "".into()).unwrap();
        assert!(log.完成(id).is_ok());
        assert!(log.完成(id).is_err());
        assert_eq!(log.查询(id).unwrap().status, 迭代状态::已完成);
    }

    #[test]
    fn 演进历史与当前版本取最新() {
        let mut log = IterationLog::new();
        let a = log.开启(Version::new(1, 0, 0), "一".into()).unwrap();
        let b = log.开启(Version::new(2, 0, 0), "二".into()).unwrap();
        log.完成(a).unwrap();
        log.完成(b).unwrap();
        assert_eq!(log.全部().len(), 2);
        assert_eq!(log.当前版本(), Version::new(2, 0, 0));
    }

    #[test]
    fn 持久化往返() {
        let path = std::env::temp_dir()
            .join("lj_iteration_test_往返.toml")
            .to_string_lossy()
            .into_owned();
        let mut log = IterationLog::new();
        log.开启(Version::new(1, 0, 0), "待持久化".into()).unwrap();
        log.保存(&path).unwrap();
        let loaded = IterationLog::加载(&path).unwrap();
        assert_eq!(loaded.全部().len(), 1);
        assert_eq!(loaded.查询(1).unwrap().变更说明, "待持久化");
        std::fs::remove_file(&path).ok();
    }
}
