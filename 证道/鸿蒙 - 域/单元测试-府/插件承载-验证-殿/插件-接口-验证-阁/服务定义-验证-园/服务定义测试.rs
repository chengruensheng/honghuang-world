//! 服务定义 - 验证 - 园 · 服务定义测试：Service Definition 机制的注册 / 查找 / 向下转型。
//! 设计依据：层级结构-设计.md §三边界模型 Service Definition 机制。
//! 仅经 chajian_fu lib 根 pub 符号测试，不深链、不碰私有实现。

#[cfg(test)]
mod 测试 {
    use chajian_fu::{府插件, 插件上下文};
    use std::sync::Arc;

    /// 测试用服务 trait。
    trait 示例服务: Send + Sync {
        fn 取值(&self) -> i32;
    }

    /// 测试用服务实现。
    struct 示例服务实例 {
        值: i32,
    }
    impl 示例服务 for 示例服务实例 {
        fn 取值(&self) -> i32 {
            self.值
        }
    }

    /// 共存测试用甲服务 trait。
    trait 甲服务: Send + Sync {
        fn 甲(&self) -> &str;
    }
    struct 甲实例;
    impl 甲服务 for 甲实例 {
        fn 甲(&self) -> &str {
            "甲"
        }
    }

    /// 共存测试用乙服务 trait。
    trait 乙服务: Send + Sync {
        fn 乙(&self) -> &str;
    }
    struct 乙实例;
    impl 乙服务 for 乙实例 {
        fn 乙(&self) -> &str {
            "乙"
        }
    }

    #[test]
    fn 注册服务后能查找() {
        let mut ctx = 插件上下文::新();
        let 服务: Arc<dyn 示例服务> = Arc::new(示例服务实例 { 值: 42 });
        ctx.注册服务(服务).unwrap();
        let 查到 = ctx.查找服务::<Arc<dyn 示例服务>>();
        assert!(查到.is_some());
        assert_eq!(查到.unwrap().取值(), 42);
    }

    #[test]
    fn 查找未注册服务返回空() {
        let ctx = 插件上下文::新();
        let 查到 = ctx.查找服务::<Arc<dyn 示例服务>>();
        assert!(查到.is_none());
    }

    #[test]
    fn 重复注册同类型服务失败() {
        let mut ctx = 插件上下文::新();
        let 服务1: Arc<dyn 示例服务> = Arc::new(示例服务实例 { 值: 1 });
        let 服务2: Arc<dyn 示例服务> = Arc::new(示例服务实例 { 值: 2 });
        ctx.注册服务(服务1).unwrap();
        let 结果 = ctx.注册服务(服务2);
        assert!(结果.is_err());
    }

    #[test]
    fn 不同类型服务可共存() {
        let mut ctx = 插件上下文::新();

        let 甲: Arc<dyn 甲服务> = Arc::new(甲实例);
        let 乙: Arc<dyn 乙服务> = Arc::new(乙实例);
        ctx.注册服务(甲).unwrap();
        ctx.注册服务(乙).unwrap();

        assert_eq!(ctx.查找服务::<Arc<dyn 甲服务>>().unwrap().甲(), "甲");
        assert_eq!(ctx.查找服务::<Arc<dyn 乙服务>>().unwrap().乙(), "乙");
    }

    /// 测试用插件：在应用()中注册服务。
    struct 服务插件;
    impl chajian_fu::府插件 for 服务插件 {
        fn 名称(&self) -> &str {
            "服务插件"
        }
        fn 注入(&self) -> Vec<&str> {
            vec![]
        }
        fn 应用(
            &self,
            ctx: &mut 插件上下文,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let 服务: Arc<dyn 示例服务> = Arc::new(示例服务实例 { 值: 100 });
            ctx.注册服务(服务)?;
            Ok(())
        }
    }

    #[test]
    fn 插件应用注册服务能查找() {
        let mut ctx = 插件上下文::新();
        // 注册()只入插件注册表不调应用()；应用()才注册服务到服务表
        ctx.注册(Box::new(服务插件)).unwrap();
        服务插件.应用(&mut ctx).unwrap();
        let 查到 = ctx.查找服务::<Arc<dyn 示例服务>>();
        assert!(查到.is_some());
        assert_eq!(查到.unwrap().取值(), 100);
    }

    #[test]
    fn 三府服务trait注册查找() {
        use daoshu_fu::道术服务;
        use shihai_fu::识海服务;
        use tianting_fu::天庭服务;

        let mut ctx = 插件上下文::新();
        // 按依赖顺序应用三府插件——应用()中注册各府服务到服务表
        // 注册()只入注册表不调应用()，故此处直接调应用()注册服务
        shihai_fu::识海插件.应用(&mut ctx).unwrap();
        daoshu_fu::道术插件.应用(&mut ctx).unwrap();
        tianting_fu::天庭插件.应用(&mut ctx).unwrap();

        // 查找三府服务
        let 识海 = ctx.查找服务::<Arc<dyn 识海服务>>();
        assert!(识海.is_some(), "识海服务应已注册");
        assert!(
            识海.unwrap().回想("测试").is_err(),
            "占位实现应返回未实现错误"
        );

        let 道术 = ctx.查找服务::<Arc<dyn 道术服务>>();
        assert!(道术.is_some(), "道术服务应已注册");
        assert!(
            道术.unwrap().执行任务("测试").is_err(),
            "占位实现应返回未实现错误"
        );

        let 天庭 = ctx.查找服务::<Arc<dyn 天庭服务>>();
        assert!(天庭.is_some(), "天庭服务应已注册");
        assert!(
            天庭.unwrap().调度要求("测试").is_err(),
            "占位实现应返回未实现错误"
        );
    }
}
