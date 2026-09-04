#[cfg(test)]
mod tests {
    use dy_rule::RuleSet;

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("dy_rule_test_{名}.toml"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn 添加规则后可按名称查询() {
        let mut set = RuleSet::new();
        let id = set.添加规则("金规", vec![("层".into(), "园".into())], "代码落园", 1).unwrap();
        let r = set.按名称("金规").unwrap();
        assert_eq!(r.id, id);
        assert_eq!(r.结论, "代码落园");
    }

    #[test]
    fn 同名规则被拒绝() {
        let mut set = RuleSet::new();
        set.添加规则("金规", vec![("层".into(), "园".into())], "一", 1).unwrap();
        assert!(set.添加规则("金规", vec![("层".into(), "园".into())], "二", 2).is_err());
        assert_eq!(set.全部().len(), 1);
    }

    #[test]
    fn 规则评估匹配满足条件的规则() {
        let mut set = RuleSet::new();
        set.添加规则("落园规", vec![("对象".into(), "代码".into())], "代码落园", 1).unwrap();
        set.添加规则("无痕规", vec![("对象".into(), "秘密".into())], "不落盘", 2).unwrap();
        let 事实 = vec![("对象".into(), "代码".into())];
        let 命中 = set.评估(&事实);
        assert_eq!(命中.len(), 1);
        assert_eq!(命中[0].名称, "落园规");
    }

    #[test]
    fn 评估按优先级降序() {
        let mut set = RuleSet::new();
        set.添加规则("低", vec![("天".into(), "雨".into())], "低", 1).unwrap();
        set.添加规则("高", vec![("天".into(), "雨".into())], "高", 9).unwrap();
        let 事实 = vec![("天".into(), "雨".into())];
        let 命中 = set.评估(&事实);
        assert_eq!(命中.len(), 2);
        assert_eq!(命中[0].名称, "高");
        assert_eq!(命中[1].名称, "低");
    }

    #[test]
    fn 持久化往返() {
        let path = 临时路径("往返");
        let mut set = RuleSet::new();
        set.添加规则("金规", vec![("层".into(), "园".into())], "代码落园", 1).unwrap();
        set.保存(&path).unwrap();
        let loaded = RuleSet::加载(&path).unwrap();
        assert_eq!(loaded.全部().len(), 1);
        assert_eq!(loaded.按名称("金规").unwrap().结论, "代码落园");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 加载后id延续不冲突() {
        let path = 临时路径("延续");
        let mut set = RuleSet::new();
        set.添加规则("一", vec![("层".into(), "园".into())], "一", 1).unwrap();
        set.保存(&path).unwrap();
        let mut loaded = RuleSet::加载(&path).unwrap();
        let new_id = loaded.添加规则("二", vec![("层".into(), "殿".into())], "二", 1).unwrap();
        assert_eq!(new_id, 2);
        std::fs::remove_file(&path).ok();
    }
}