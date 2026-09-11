#[cfg(test)]
mod tests {
    use hm_config::{从目录向上查找, 对外契约, 对外配置};

    // 内置默认：空配置项 = 关闭对应能力，sse_max 兜底 10
    #[test]
    fn 对外配置默认值正确() {
        let 配置 = 对外配置::default();
        assert_eq!(配置.sse_max, 10);
        assert!(配置.cors_origins.is_empty());
        assert_eq!(配置.static_dir, "");
    }

    // TOML 解析：显式值覆盖默认（sse_max=0 表示不限制，cors_origins 指定来源）
    #[test]
    fn 对外契约解析覆盖字段() {
        let 文本 = "[\"对外\"]\nsse_max = 0\ncors_origins = [\"http://localhost:1234\"]\nstatic_dir = \"页面\"\n";
        let 契约: 对外契约 = toml::from_str(文本).unwrap();
        assert_eq!(契约.对外.sse_max, 0);
        assert_eq!(契约.对外.cors_origins, vec!["http://localhost:1234".to_string()]);
        assert_eq!(契约.对外.static_dir, "页面");
    }

    // TOML 解析：缺字段回落到结构体默认（空来源、不托管、sse_max=10）
    #[test]
    fn 对外契约缺字段用默认() {
        let 契约: 对外契约 = toml::from_str("").unwrap();
        assert_eq!(契约.对外.sse_max, 10);
        assert!(契约.对外.cors_origins.is_empty());
        assert_eq!(契约.对外.static_dir, "");
    }

    // 从子目录向上命中祖先目录中的契约文件（纯函数，不依赖 cwd）
    #[test]
    fn 从目录向上查找命中祖先文件() {
        let 根 = std::env::temp_dir().join("洪荒对外契约-向上命中");
        std::fs::remove_dir_all(&根).ok();
        let 子 = 根.join("甲").join("乙");
        std::fs::create_dir_all(&子).unwrap();
        let 目标 = 根.join("对外契约.toml");
        std::fs::write(&目标, "[对外]\nsse_max = 3\n").unwrap();

        let 命中 = 从目录向上查找(&子, "对外契约.toml");
        assert_eq!(命中, Some(目标));

        std::fs::remove_dir_all(&根).ok();
    }

    // 未命中：整条祖先链都不存在该文件时返回 None
    #[test]
    fn 从目录向上查找未命中() {
        let 根 = std::env::temp_dir().join("洪荒对外契约-向上未命中");
        std::fs::remove_dir_all(&根).ok();
        let 子 = 根.join("甲");
        std::fs::create_dir_all(&子).unwrap();

        let 命中 = 从目录向上查找(&子, "洪荒对外契约-不存在-9f3a7c.toml");
        assert_eq!(命中, None);

        std::fs::remove_dir_all(&根).ok();
    }
}
