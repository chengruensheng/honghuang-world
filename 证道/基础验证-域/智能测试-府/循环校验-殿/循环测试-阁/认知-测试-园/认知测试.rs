#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use hm_agent::{智能体, 五层协作驱动器, 认知注入, 驱动结果};
    use hm_contract::Component;
    use hm_content_contract::{工具对话器, 对话消息, 工具调用, 模型响应, 消息角色 as 契约角色};
    use hm_cognition::{ContextManager, 上下文库, 模块, 图谱, 维度, 心智地图, 消息角色};
    use hm_error::{Error, Result};
    use hm_execute_contract::执行器;
    use tc_task::{AgentRole, Task, TaskBoard, TaskStatus};

    /// 记录对话器：捕获每次对话的完整消息序列（供三态装配断言）
    struct 记录对话器 {
        记录: Mutex<Vec<Vec<对话消息>>>,
        序列: Mutex<VecDeque<模型响应>>,
    }

    impl 记录对话器 {
        fn 新(序列: Vec<模型响应>) -> Self {
            记录对话器 { 记录: Mutex::new(Vec::new()), 序列: Mutex::new(序列.into()) }
        }
    }

    impl Component for 记录对话器 {
        fn name(&self) -> &'static str { "记录对话器" }
    }

    impl 工具对话器 for 记录对话器 {
        fn 对话(&self, 消息: Vec<对话消息>, _工具: Vec<serde_json::Value>) -> Result<模型响应> {
            self.记录.lock().expect("记录对话器 锁中毒").push(消息);
            self.序列
                .lock()
                .expect("记录对话器序列 锁中毒")
                .pop_front()
                .ok_or_else(|| Error::Other("对话序列已耗尽".into()))
        }
    }

    /// 模拟执行器：工具调用成功返回固定内容
    struct 模拟执行器;

    impl Component for 模拟执行器 {
        fn name(&self) -> &'static str { "模拟执行器" }
    }

    impl 执行器 for 模拟执行器 {
        fn 读文件(&self, _路径: &str) -> Result<String> { Ok("文件内容".into()) }
        fn 写文件(&self, _路径: &str, _内容: &str) -> Result<()> { Ok(()) }
        fn 运行命令(&self, _命令: &str) -> Result<String> { Ok("命令输出".into()) }
        fn 列目录(&self, _路径: &str) -> Result<String> { Ok("（空目录）".into()) }
        fn 按名找文件(&self, _模式: &str) -> Result<String> { Ok("（未找到）".into()) }
        fn 搜索内容(&self, _关键词: &str) -> Result<String> { Ok("（无匹配）".into()) }
        fn 精确编辑(&self, _路径: &str, _旧: &str, _新: &str) -> Result<String> { Ok("已编辑".into()) }
    }

    /// 辅助三态：图谱含 hm-linkage 模块；心智含 2 个已填格位（可信度 0.9 / 0.6）；空上下文库
    fn 辅助三态() -> (Arc<Mutex<图谱>>, Arc<Mutex<心智地图>>, Arc<Mutex<上下文库>>) {
        let mut 图谱 = 图谱::新();
        图谱.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动装配".into() });
        let mut 心智 = 心智地图::新();
        心智.写(维度::目标, "现况", "完成三态认知装配", 0.9, vec!["v1.41".into()])
            .expect("写格位应成功");
        心智.写(维度::内部, "角色", "三十六格位分层", 0.6, vec![])
            .expect("写格位应成功");
        (
            Arc::new(Mutex::new(图谱)),
            Arc::new(Mutex::new(心智)),
            Arc::new(Mutex::new(上下文库::新_带上限(1000))),
        )
    }

    // ==================== 认知注入 ====================

    #[test]
    fn 认知注入_初始注入含推拉片段() {
        let (图谱, 心智, 上下文) = 辅助三态();
        let 注入 = 认知注入::新(图谱, 心智, 上下文);
        let 段 = 注入.初始注入("完成 hm-linkage 的装配工作");

        assert!(段.contains("【格位·常驻】"), "应含推注入段，实际: {段}");
        assert!(段.contains("完成三态认知装配"), "应含可信度最高的格位摘要，实际: {段}");
        assert!(段.contains("【图谱·按需】"), "应含拉注入段，实际: {段}");
        assert!(段.contains("hm-linkage"), "任务关键词应拉到图谱模块节点，实际: {段}");

        // 推模式按可信度降序：目标(0.9) 排在 内部(0.6) 之前
        let 目标位置 = 段.find("完成三态认知装配").expect("目标格位应在段中");
        let 内部位置 = 段.find("三十六格位分层").expect("内部格位应在段中");
        assert!(目标位置 < 内部位置, "可信度 0.9 格位应在 0.6 之前");
    }

    #[test]
    fn 认知注入_空三态返回空串() {
        let 图谱 = Arc::new(Mutex::new(图谱::新()));
        let 心智 = Arc::new(Mutex::new(心智地图::新()));
        let 上下文 = Arc::new(Mutex::new(上下文库::新()));
        let 注入 = 认知注入::新(图谱, 心智, 上下文);

        assert!(注入.初始注入("任意任务").is_empty(), "空三态初始注入应为空串");
        assert!(注入.流注入().is_empty(), "空上下文流注入应为空串");
    }

    #[test]
    fn 认知注入_流注入为最近消息() {
        let (图谱, 心智, 上下文) = 辅助三态();
        let 注入 = 认知注入::新(图谱, 心智, 上下文);
        注入.记录(消息角色::用户, "第一问");
        注入.记录(消息角色::助手, "第一答");

        let 流 = 注入.流注入();
        assert!(流.contains("【临时·上下文流】"), "应含流注入段头，实际: {流}");
        assert!(流.contains("[用户] 第一问"), "应含用户消息，实际: {流}");
        assert!(流.contains("[助手] 第一答"), "应含助手消息，实际: {流}");
    }

    #[test]
    fn 认知注入_记录写入上下文库() {
        let (图谱, 心智, 上下文) = 辅助三态();
        let 注入 = 认知注入::新(图谱, 心智, 上下文.clone());
        注入.记录(消息角色::用户, "任务");
        注入.记录(消息角色::工具结果, "结果");

        let 库 = 上下文.lock().expect("上下文锁");
        assert_eq!(库.长度(), 2, "两条记录应写入上下文库");
        let 最近 = 库.最近(2);
        assert_eq!(最近[0].内容, "任务");
        assert_eq!(最近[1].内容, "结果");
    }

    #[test]
    fn 认知注入_上下文库超限丢最旧() {
        let 图谱 = Arc::new(Mutex::new(图谱::新()));
        let 心智 = Arc::new(Mutex::new(心智地图::新()));
        let 上下文 = Arc::new(Mutex::new(上下文库::新_带上限(3)));
        let 注入 = 认知注入::新(图谱, 心智, 上下文.clone());
        注入.记录(消息角色::用户, "一");
        注入.记录(消息角色::用户, "二");
        注入.记录(消息角色::用户, "三");
        注入.记录(消息角色::用户, "四");

        let 库 = 上下文.lock().expect("上下文锁");
        assert_eq!(库.长度(), 3, "超限应丢最旧");
        let 最近 = 库.最近(10);
        assert_eq!(最近[0].内容, "二", "最早的「一」应被丢弃");
        assert_eq!(最近[2].内容, "四");
    }

    // ==================== 智能体装配 ====================

    #[test]
    fn 智能体_装配认知初始消息三态段() {
        let (图谱, 心智, 上下文) = 辅助三态();
        let 注入 = 认知注入::新(图谱, 心智, 上下文);
        let 对话器 = Arc::new(记录对话器::新(vec![
            模型响应 { 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器.clone(), Arc::new(模拟执行器), 5).装配认知(注入);

        智能体.运行("完成 hm-linkage 装配".into()).expect("运行应成功");

        let 记录 = 对话器.记录.lock().expect("记录锁");
        assert_eq!(记录.len(), 1, "单轮对话");
        let 首轮 = &记录[0];
        assert!(首轮.len() >= 3, "消息应含 [系统提示, 三态段, 用户任务]，实际 {} 条", 首轮.len());
        assert_eq!(首轮[0].角色, 契约角色::System, "首条为系统提示");
        assert_eq!(首轮[1].角色, 契约角色::System, "第二条为三态注入段");
        let 三态段 = 首轮[1].内容.as_ref().expect("三态段应有内容");
        assert!(三态段.contains("【格位·常驻】"), "三态段应含推注入，实际: {三态段}");
        assert!(三态段.contains("hm-linkage"), "三态段应含拉注入，实际: {三态段}");
        assert_eq!(首轮[2].角色, 契约角色::User, "第三条为用户任务");
        assert_eq!(首轮[2].内容.as_deref(), Some("完成 hm-linkage 装配"));
    }

    #[test]
    fn 智能体_每轮流注入与过程记录() {
        let (图谱, 心智, 上下文) = 辅助三态();
        let 注入 = 认知注入::新(图谱, 心智, 上下文.clone());
        let 对话器 = Arc::new(记录对话器::新(vec![
            模型响应 {
                内容: None,
                工具调用: vec![工具调用 { id: "call_1".into(), 名称: "读文件".into(), 参数: r#"{"路径":"/tmp/a.txt"}"#.into() }],
            },
            模型响应 { 内容: Some("完成".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器.clone(), Arc::new(模拟执行器), 5).装配认知(注入);

        let 答复 = 智能体.运行("读取文件完成任务".into()).expect("运行应成功");
        assert_eq!(答复, "完成");

        let 记录 = 对话器.记录.lock().expect("记录锁");
        assert_eq!(记录.len(), 2, "两轮对话：工具轮 + 答复轮");
        // 第二轮消息末尾为流注入段（系统），含第一轮过程记录
        let 第二轮 = &记录[1];
        let 末条 = 第二轮.last().expect("第二轮应有消息");
        assert_eq!(末条.角色, 契约角色::System, "消息末尾应为流注入段");
        let 流段 = 末条.内容.as_ref().expect("流段应有内容");
        assert!(流段.contains("【临时·上下文流】"), "应含流注入段头，实际: {流段}");
        assert!(流段.contains("读取文件完成任务"), "流段应含任务记录，实际: {流段}");
        assert!(流段.contains("读文件"), "流段应含工具调用记录，实际: {流段}");

        // 上下文库应记录 任务/工具调用/工具结果/最终答复
        let 库 = 上下文.lock().expect("上下文锁");
        let 最近 = 库.最近(20);
        assert!(最近.iter().any(|m| m.角色 == 消息角色::用户 && m.内容.contains("读取文件完成任务")));
        assert!(最近.iter().any(|m| m.内容.contains("调用工具 读文件")), "应记录工具调用");
        assert!(最近.iter().any(|m| m.角色 == 消息角色::工具结果 && m.内容.contains("文件内容")));
        assert!(最近.iter().any(|m| m.角色 == 消息角色::助手 && m.内容 == "完成"), "应记录最终答复");
    }

    #[test]
    fn 智能体_未装配认知消息不变() {
        let 对话器 = Arc::new(记录对话器::新(vec![
            模型响应 { 内容: Some("直接答复".into()), 工具调用: vec![] },
        ]));
        let 智能体 = 智能体::new(对话器.clone(), Arc::new(模拟执行器), 5);

        智能体.运行("任务".into()).expect("运行应成功");

        let 记录 = 对话器.记录.lock().expect("记录锁");
        let 首轮 = &记录[0];
        assert_eq!(首轮.len(), 2, "未装配认知消息应为 [系统提示, 用户任务]");
        assert_eq!(首轮[0].角色, 契约角色::System);
        assert_eq!(首轮[1].角色, 契约角色::User);
        assert_eq!(首轮[1].内容.as_deref(), Some("任务"));
    }

    // ==================== 五层协作驱动器透传 ====================

    fn 临时路径(名: &str) -> String {
        let 目录 = std::env::temp_dir().join("zd-agent-认知");
        let _ = std::fs::create_dir_all(&目录);
        目录.join(名).to_string_lossy().to_string()
    }

    fn 设计样例() -> &'static str {
        r#"{"边界定义":{"模块":"a"},"安全区域":[],"契约":[{"契约名":"测试契约","方法":[{"名称":"方法一","签名":"fn 方法一()","描述":"做某事"}],"描述":"契约描述"}],"修改文件":[],"新建文件":[],"依赖":[]}"#
    }

    #[test]
    fn 驱动器_装配认知透传记录() {
        let mut 看板 = TaskBoard::新建(临时路径("认知看板"));
        let mut task = Task::新建(0, "设计任务".to_string(), "需求描述".to_string(), 0);
        task.status = TaskStatus::待圣人设计;
        看板.发布任务(task).expect("发布应成功");
        let 看板 = Arc::new(Mutex::new(看板));
        let 管理器 = Arc::new(Mutex::new(ContextManager::新(&临时路径("认知ctx"))));
        let (图谱, 心智, 库) = 辅助三态();
        let 注入 = 认知注入::新(图谱, 心智, 库.clone());
        let 对话器 = Arc::new(记录对话器::新(vec![
            模型响应 { 内容: Some(设计样例().into()), 工具调用: vec![] },
        ]));
        let 驱动器 = 五层协作驱动器::新(看板.clone(), 管理器, 对话器, Arc::new(模拟执行器), 10)
            .装配认知(注入);

        let 结果 = 驱动器.执行一轮().expect("驱动应成功");
        assert_eq!(
            结果,
            驱动结果::阶段完成 { 任务id: 1, 角色: AgentRole::圣人, 新状态: TaskStatus::待大罗金仙实现 }
        );

        // 临时态上下文库应记录本轮 阶段提示（用户）与 答复（助手）
        let 库 = 库.lock().expect("上下文锁");
        assert!(库.长度() >= 2, "临时态应至少记录提示与答复，实际 {}", 库.长度());
        let 最近 = 库.最近(20);
        assert!(
            最近.iter().any(|m| m.角色 == 消息角色::用户 && m.内容.contains("设计任务")),
            "临时态应记录含任务标题的阶段提示"
        );
        assert!(
            最近.iter().any(|m| m.角色 == 消息角色::助手),
            "临时态应记录助手答复"
        );
    }
}
