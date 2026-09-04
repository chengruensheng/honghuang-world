#[cfg(test)]
mod tests {
    use hm_config::Config;

    #[test]
    fn 解析合法配置() {
        let s = "[app]\nname = \"测试\"\nversion = \"0.0.1\"\n[log]\nlevel = \"debug\"\n";
        let cfg: Config = toml::from_str(s).unwrap();
        assert_eq!(cfg.app.name, "测试");
        assert_eq!(cfg.log.level, "debug");
    }

    #[test]
    fn 缺字段用默认值() {
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg.app.name, "洪荒·世界");
        assert_eq!(cfg.log.level, "info");
    }

    #[test]
    fn 默认配置可解析() {
        let cfg = hm_config::default_config();
        assert_eq!(cfg.app.name, "洪荒·世界");
        assert_eq!(cfg.log.level, "info");
    }
}
