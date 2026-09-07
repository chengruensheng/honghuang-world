#[cfg(test)]
mod tests {
    use tc_task::{AgentRole, TaskStatus, 五行层级, 状态归属角色, 状态层级标签};

    /// 测试1：状态层级标签 全状态覆盖（含召回/澄清/清理状态）
    #[test]
    fn 状态层级标签_全状态覆盖() {
        use TaskStatus::*;
        // 火=设计层
        assert_eq!(状态层级标签(待圣人设计), 五行层级::火);
        assert_eq!(状态层级标签(圣人设计中), 五行层级::火);
        assert_eq!(状态层级标签(待重新设计), 五行层级::火);
        // 土=实现层
        assert_eq!(状态层级标签(待大罗金仙实现), 五行层级::土);
        assert_eq!(状态层级标签(大罗金仙实现中), 五行层级::土);
        assert_eq!(状态层级标签(待修复), 五行层级::土);
        assert_eq!(状态层级标签(待重新实现), 五行层级::土);
        // 金=验收层
        assert_eq!(状态层级标签(待准圣验收), 五行层级::金);
        assert_eq!(状态层级标签(准圣验收中), 五行层级::金);
        assert_eq!(状态层级标签(待道祖终审), 五行层级::金);
        assert_eq!(状态层级标签(道祖终审中), 五行层级::金);
        assert_eq!(状态层级标签(待重新验收), 五行层级::金);
        assert_eq!(状态层级标签(已完成), 五行层级::金);
        // 水=清理层
        assert_eq!(状态层级标签(待清理), 五行层级::水);
        assert_eq!(状态层级标签(清理中), 五行层级::水);
        assert_eq!(状态层级标签(待重新清理), 五行层级::水);
        assert_eq!(状态层级标签(清理完成), 五行层级::水);
        // 木=需求层
        assert_eq!(状态层级标签(待受理), 五行层级::木);
        assert_eq!(状态层级标签(进行中), 五行层级::木);
        assert_eq!(状态层级标签(已取消), 五行层级::木);
        assert_eq!(状态层级标签(待道祖澄清), 五行层级::木);
        assert_eq!(状态层级标签(道祖澄清中), 五行层级::木);
    }

    /// 测试2：五行层级::归属角色 与 From<AgentRole> 双向一致
    #[test]
    fn 五行层级归属角色_与角色映射双向一致() {
        for 角色 in [
            AgentRole::道祖,
            AgentRole::圣人,
            AgentRole::大罗金仙,
            AgentRole::准圣,
            AgentRole::太乙金仙,
        ] {
            let 层级 = 五行层级::from(角色);
            assert_eq!(层级.归属角色(), Some(角色), "{:?} 应映射回自身", 角色);
        }
    }

    /// 测试3：五行层级 全部遍历（生序）与 阶段名 断言
    #[test]
    fn 五行层级全部与阶段名() {
        let 全部 = 五行层级::全部();
        assert_eq!(全部, [五行层级::木, 五行层级::火, 五行层级::土, 五行层级::金, 五行层级::水]);
        assert_eq!(五行层级::木.阶段名(), "需求");
        assert_eq!(五行层级::火.阶段名(), "设计");
        assert_eq!(五行层级::土.阶段名(), "实现");
        assert_eq!(五行层级::金.阶段名(), "验收");
        assert_eq!(五行层级::水.阶段名(), "清理");
        // 生序连续
        for (i, 层) in 全部.iter().enumerate() {
            assert_eq!(层.序() as usize, i + 1, "生序应连续");
        }
    }

    /// 测试4：状态归属角色（收敛后）各状态归属正确（回归行为）
    #[test]
    fn 状态归属角色_各状态归属正确() {
        use TaskStatus::*;
        assert_eq!(状态归属角色(待圣人设计), Some(AgentRole::圣人));
        assert_eq!(状态归属角色(圣人设计中), Some(AgentRole::圣人));
        assert_eq!(状态归属角色(待大罗金仙实现), Some(AgentRole::大罗金仙));
        assert_eq!(状态归属角色(大罗金仙实现中), Some(AgentRole::大罗金仙));
        assert_eq!(状态归属角色(待修复), Some(AgentRole::大罗金仙));
        assert_eq!(状态归属角色(待准圣验收), Some(AgentRole::准圣));
        assert_eq!(状态归属角色(准圣验收中), Some(AgentRole::准圣));
        assert_eq!(状态归属角色(待道祖终审), Some(AgentRole::道祖));
        assert_eq!(状态归属角色(道祖终审中), Some(AgentRole::道祖));
        assert_eq!(状态归属角色(待道祖澄清), Some(AgentRole::道祖));
        assert_eq!(状态归属角色(道祖澄清中), Some(AgentRole::道祖));
        assert_eq!(状态归属角色(待清理), Some(AgentRole::太乙金仙));
        assert_eq!(状态归属角色(清理中), Some(AgentRole::太乙金仙));
        // 无操作角色：终态/召回态/进行中原生态
        assert_eq!(状态归属角色(待受理), None);
        assert_eq!(状态归属角色(已完成), None);
        assert_eq!(状态归属角色(已取消), None);
        assert_eq!(状态归属角色(清理完成), None);
        assert_eq!(状态归属角色(待重新设计), None);
        assert_eq!(状态归属角色(待重新实现), None);
        assert_eq!(状态归属角色(待重新验收), None);
        assert_eq!(状态归属角色(待重新清理), None);
    }

    /// 测试5：层级标签与角色归属的一致性契约——同层内不同操作状态可映射到同一层级
    #[test]
    fn 层级标签_同层不同操作状态归一() {
        // 验收层：准圣验收中 与 道祖终审中 同属金层（层级是处理域，角色是操作者）
        assert_eq!(状态层级标签(TaskStatus::准圣验收中), 状态层级标签(TaskStatus::道祖终审中));
        assert_eq!(状态层级标签(TaskStatus::准圣验收中), 五行层级::金);
        // 需求层：澄清与待受理同属木层
        assert_eq!(状态层级标签(TaskStatus::待道祖澄清), 五行层级::木);
        assert_eq!(状态层级标签(TaskStatus::待受理), 五行层级::木);
    }
}
