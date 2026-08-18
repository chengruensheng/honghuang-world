//! 播种-归纳-园 · 测试：按种子降级链归纳语义记录（含界主交互）。

#[cfg(test)]
mod 测试 {
    use moxing_fu::模型配置;
    use shihai_fu::{
        共享度, 固化度, 播种结果, 播种降级, 格位, 模型存储, 模型播种, 界主交互, 范畴, 顺序档位,
    };

    fn 造格位(推荐位置: &str) -> 格位 {
        let mut 格位 = 格位::新(
            "架构",
            范畴::规则,
            "技术架构原则",
            "人类",
            固化度::经,
            共享度::共享,
            顺序档位::最前,
        );
        格位.推荐位置 = 推荐位置.to_string();
        格位
    }

    #[test]
    fn 无推荐位置且无源文件则回落人类() {
        let 根 = std::env::temp_dir().join("识海测试-播种-空目录");
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();

        let 格位 = 造格位("");
        let 配置 = 模型配置 {
            密钥: "k".into(),
            地址: "未用".into(),
            模型: "m".into(),
        };
        let 存储 = 模型存储::打开(&根);
        match 播种降级(&存储, &格位, &配置, &根).unwrap() {
            播种结果::需人类(_) => {}
            播种结果::已归纳(_) => unreachable!("无文档无代码不应归纳出语义"),
        }

        std::fs::remove_dir_all(&根).ok();
    }

    struct 假界主;
    impl 界主交互 for 假界主 {
        fn 审阅(&self, _格位名: &str, _归纳: &str, _证据: &str) -> bool {
            true
        }
        fn 询问(&self, 格位名: &str, _问题: &str) -> String {
            format!("{格位名} 的回答")
        }
    }

    #[test]
    fn 模型播种无文档则代码为主() {
        let 根 = std::env::temp_dir().join("识海测试-模型-无文档");
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).unwrap();
        std::fs::write(根.join("样例.rs"), "fn 主函数() {}\n").unwrap();

        let 格位 = 造格位("");
        let 配置 = 模型配置 {
            密钥: "k".into(),
            地址: "未用".into(),
            模型: "m".into(),
        };
        let 存储 = 模型存储::打开(&根);
        match 模型播种(&存储, &格位, &配置, &根, &假界主).unwrap() {
            播种结果::已归纳(摘要) => {
                assert!(摘要.contains("fn 主函数"));
                assert_eq!(存储.读格位(&格位.名字).unwrap().len(), 1);
            }
            播种结果::需人类(_) => unreachable!("有代码时应由代码归纳，不应转人类"),
        }

        std::fs::remove_dir_all(&根).ok();
    }
}
