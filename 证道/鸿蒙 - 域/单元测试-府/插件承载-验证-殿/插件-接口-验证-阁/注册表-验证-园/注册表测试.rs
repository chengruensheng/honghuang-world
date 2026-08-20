//! 注册表 - 验证 - 园 · 注册表测试：插件注册表的注册 / 查找 / 依赖检查 / 批量注册。
//! 设计依据：层级结构-设计.md §三边界模型、§五鸿蒙地基模型。
//! 仅经 chajian_fu lib 根 pub 符号测试，不深链、不碰私有实现。

#[cfg(test)]
mod 测试 {
    use chajian_fu::{府插件, 批量注册, 插件上下文};

    /// 测试用插件：无依赖，应用为空操作。
    struct 无依赖插件 {
        名: &'static str,
    }
    impl 府插件 for 无依赖插件 {
        fn 名称(&self) -> &str {
            self.名
        }
        fn 注入(&self) -> Vec<&str> {
            vec![]
        }
        fn 应用(
            &self,
            _ctx: &mut 插件上下文,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    /// 测试用插件：依赖指定插件，应用为空操作。
    struct 依赖插件 {
        名: &'static str,
        依赖名: &'static str,
    }
    impl 府插件 for 依赖插件 {
        fn 名称(&self) -> &str {
            self.名
        }
        fn 注入(&self) -> Vec<&str> {
            vec![self.依赖名]
        }
        fn 应用(
            &self,
            _ctx: &mut 插件上下文,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    #[test]
    fn 注册单插件后能查找() {
        let mut ctx = 插件上下文::新();
        ctx.注册(Box::new(无依赖插件 { 名: "甲府" })).unwrap();
        assert_eq!(ctx.查找("甲府"), Some("甲府".to_string()));
    }

    #[test]
    fn 查找不存在返回空() {
        let ctx = 插件上下文::新();
        assert_eq!(ctx.查找("不存在"), None);
    }

    #[test]
    fn 依赖未满足注册失败() {
        let mut ctx = 插件上下文::新();
        let 结果 = ctx.注册(Box::new(依赖插件 {
            名: "乙府",
            依赖名: "甲府",
        }));
        assert!(结果.is_err());
    }

    #[test]
    fn 依赖已满足注册成功() {
        let mut ctx = 插件上下文::新();
        ctx.注册(Box::new(无依赖插件 { 名: "甲府" })).unwrap();
        ctx.注册(Box::new(依赖插件 {
            名: "乙府",
            依赖名: "甲府",
        }))
        .unwrap();
        assert_eq!(ctx.查找("乙府"), Some("乙府".to_string()));
    }

    #[test]
    fn 已注册列表正确() {
        let mut ctx = 插件上下文::新();
        ctx.注册(Box::new(无依赖插件 { 名: "甲府" })).unwrap();
        ctx.注册(Box::new(无依赖插件 { 名: "乙府" })).unwrap();
        let 已注册 = ctx.已注册();
        assert_eq!(已注册.len(), 2);
        assert!(已注册.contains(&"甲府".to_string()));
        assert!(已注册.contains(&"乙府".to_string()));
    }

    #[test]
    fn 批量注册按顺序() {
        let mut ctx = 插件上下文::新();
        let 插件们: Vec<Box<dyn 府插件>> = vec![
            Box::new(无依赖插件 { 名: "甲府" }),
            Box::new(依赖插件 {
                名: "乙府",
                依赖名: "甲府",
            }),
        ];
        批量注册(&mut ctx, 插件们).unwrap();
        assert_eq!(ctx.已注册().len(), 2);
    }

    #[test]
    fn 批量注册依赖未满足提前失败() {
        let mut ctx = 插件上下文::新();
        // 依赖方在前，被依赖方在后——应在依赖方处提前失败
        let 插件们: Vec<Box<dyn 府插件>> = vec![
            Box::new(依赖插件 {
                名: "乙府",
                依赖名: "甲府",
            }),
            Box::new(无依赖插件 { 名: "甲府" }),
        ];
        let 结果 = 批量注册(&mut ctx, 插件们);
        assert!(结果.is_err());
        assert_eq!(ctx.已注册().len(), 0);
    }
}
