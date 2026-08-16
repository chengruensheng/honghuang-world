//! 派发 - 落单 - 园 · 派发落单测试：设计方案拆解为执行任务。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::工作流级别;
    use tianting_fu::{拆解为任务, 拆解项, 设计方案};

    #[test]
    fn 拆解为任务_测试() {
        let 方案 = 设计方案 {
            要求id: "r1".to_string(),
            设计: "设计".to_string(),
            拆解: vec![拆解项 {
                目标: "写个函数".to_string(),
                执行层角色: vec!["duobao".to_string()],
                工作流: "L2_script".to_string(),
            }],
            自评: "自评".to_string(),
        };
        let 任务们 = 拆解为任务(&方案);
        assert_eq!(任务们.len(), 1);
        assert_eq!(任务们[0].目标, "写个函数");
        assert_eq!(任务们[0].工作流, 工作流级别::脚本);
    }
}
