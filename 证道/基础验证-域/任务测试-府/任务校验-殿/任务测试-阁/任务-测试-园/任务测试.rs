#[cfg(test)]
mod tests {
    use tc_task::{
        AgentRole, Task, TaskBoard, TaskPriority, TaskScene, TaskStatus,
        RequirementDoc, DesignDoc, ImplementationDoc, SelfCheckResult,
        VerificationDoc, VerificationRound, FinalAcceptanceDoc,
    };

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("tc_task_board_test_{名}.jsonl"))
            .to_string_lossy()
            .into_owned()
    }

    fn 造任务(标题: &str) -> Task {
        let mut task = Task::新建(0, 标题.into(), "测试描述".into(), 100);
        task.status = TaskStatus::待圣人设计;
        task.发起人 = AgentRole::道祖;
        task
    }

    #[test]
    fn 任务诞生后状态为待受理() {
        let task = Task::新建(1, "第一个任务".into(), "验证诞生".into(), 100);
        assert_eq!(task.title, "第一个任务");
        assert_eq!(task.status, TaskStatus::待受理);
    }

    #[test]
    fn 任务新建时五层元信息有默认值() {
        let task = Task::新建(1, "默认值验证".into(), "".into(), 100);
        assert_eq!(task.场景, TaskScene::理解);
        assert_eq!(task.优先级, TaskPriority::P2);
        assert_eq!(task.发起人, AgentRole::道祖);
        assert_eq!(task.当前承接人, AgentRole::道祖);
        assert!(task.承接历史.is_empty());
        assert!(task.需求文档.is_none());
        assert!(task.设计文档.is_none());
        assert!(task.实现文档.is_none());
        assert!(task.验收文档.is_none());
        assert!(task.终审文档.is_none());
        assert!(task.状态历史.is_empty());
        assert_eq!(task.修复轮次, 0);
    }

    #[test]
    fn 五层状态_待受理可流转到待圣人设计() {
        assert!(TaskStatus::待受理.可流转到(&TaskStatus::待圣人设计));
    }

    #[test]
    fn 五层状态_完整流转路径到已完成() {
        let 路径 = [
            TaskStatus::待受理,
            TaskStatus::待圣人设计,
            TaskStatus::圣人设计中,
            TaskStatus::待大罗金仙实现,
            TaskStatus::大罗金仙实现中,
            TaskStatus::待准圣验收,
            TaskStatus::准圣验收中,
            TaskStatus::待道祖终审,
            TaskStatus::道祖终审中,
            TaskStatus::已完成,
        ];
        for i in 0..路径.len() - 1 {
            assert!(
                路径[i].可流转到(&路径[i + 1]),
                "{:?} 应可流转到 {:?}",
                路径[i],
                路径[i + 1]
            );
        }
    }

    #[test]
    fn 五层状态_验收不通过流转到待修复() {
        assert!(TaskStatus::准圣验收中.可流转到(&TaskStatus::待修复));
    }

    #[test]
    fn 五层状态_待修复流转回大罗金仙实现中() {
        assert!(TaskStatus::待修复.可流转到(&TaskStatus::大罗金仙实现中));
    }

    #[test]
    fn 五层状态_道祖终审不通过流转到待修复() {
        assert!(TaskStatus::道祖终审中.可流转到(&TaskStatus::待修复));
    }

    #[test]
    fn 五层状态_待受理不可直接到已完成() {
        assert!(!TaskStatus::待受理.可流转到(&TaskStatus::已完成));
    }

    #[test]
    fn 五层状态_圣人设计中不可跳到准圣验收中() {
        assert!(!TaskStatus::圣人设计中.可流转到(&TaskStatus::准圣验收中));
    }

    #[test]
    fn 看板_发布任务自动分配序号并记录状态历史() {
        let path = 临时路径("发布");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("发布测试")).expect("发布应成功");
        assert_eq!(id, 1);
        let task = board.查询(id).expect("任务应存在");
        assert_eq!(task.title, "发布测试");
        assert_eq!(task.status, TaskStatus::待圣人设计);
        assert_eq!(task.状态历史.len(), 1);
        assert_eq!(task.状态历史[0].新状态, TaskStatus::待圣人设计);
        assert_eq!(task.状态历史[0].备注.as_deref(), Some("发布"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_圣人承接待圣人设计任务() {
        let path = 临时路径("承接");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("承接测试")).expect("发布应成功");
        board.承接任务(id, AgentRole::圣人).expect("圣人承接应成功");
        let task = board.查询(id).expect("任务应存在");
        assert_eq!(task.status, TaskStatus::圣人设计中);
        assert_eq!(task.当前承接人, AgentRole::圣人);
        assert_eq!(task.承接历史, vec![AgentRole::圣人]);
        assert_eq!(task.状态历史.len(), 2);
        assert_eq!(task.状态历史[1].新状态, TaskStatus::圣人设计中);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_角色不符承接被拒绝() {
        let path = 临时路径("角色不符");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("角色不符测试")).expect("发布应成功");
        let result = board.承接任务(id, AgentRole::大罗金仙);
        assert!(result.is_err());
        let err = result.expect_err("应报错");
        assert!(err.to_string().contains("角色不符"));
        let task = board.查询(id).expect("任务应存在");
        assert_eq!(task.status, TaskStatus::待圣人设计, "状态不应改变");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_五层完整流转到已完成() {
        let path = 临时路径("完整流转");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("完整流转测试")).expect("发布应成功");

        board.承接任务(id, AgentRole::圣人).expect("圣人承接");
        board.提交任务(id, AgentRole::圣人, TaskStatus::待大罗金仙实现).expect("圣人提交");

        board.承接任务(id, AgentRole::大罗金仙).expect("大罗金仙承接");
        board.提交任务(id, AgentRole::大罗金仙, TaskStatus::待准圣验收).expect("大罗金仙提交");

        board.承接任务(id, AgentRole::准圣).expect("准圣承接");
        board.提交任务(id, AgentRole::准圣, TaskStatus::待道祖终审).expect("准圣提交通过");

        board.承接任务(id, AgentRole::道祖).expect("道祖承接");
        board.提交任务(id, AgentRole::道祖, TaskStatus::已完成).expect("道祖终审通过");

        let task = board.查询(id).expect("任务应存在");
        assert_eq!(task.status, TaskStatus::已完成);
        assert_eq!(task.状态历史.len(), 9);
        assert_eq!(task.承接历史, vec![AgentRole::圣人, AgentRole::大罗金仙, AgentRole::准圣, AgentRole::道祖]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_验收不通过进入待修复且修复轮次递增() {
        let path = 临时路径("待修复");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("修复测试")).expect("发布应成功");

        board.承接任务(id, AgentRole::圣人).expect("圣人承接");
        board.提交任务(id, AgentRole::圣人, TaskStatus::待大罗金仙实现).expect("圣人提交");

        board.承接任务(id, AgentRole::大罗金仙).expect("大罗金仙承接");
        board.提交任务(id, AgentRole::大罗金仙, TaskStatus::待准圣验收).expect("大罗金仙提交");

        board.承接任务(id, AgentRole::准圣).expect("准圣承接");
        board.提交任务(id, AgentRole::准圣, TaskStatus::待修复).expect("验收不通过");

        let task = board.查询(id).expect("任务应存在");
        assert_eq!(task.status, TaskStatus::待修复);
        assert_eq!(task.修复轮次, 1);

        board.承接任务(id, AgentRole::大罗金仙).expect("大罗金仙修复承接");
        let task = board.查询(id).expect("任务应存在");
        assert_eq!(task.status, TaskStatus::大罗金仙实现中);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_道祖终审不通过进入待修复() {
        let path = 临时路径("终审不通过");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("终审不通过测试")).expect("发布应成功");

        board.承接任务(id, AgentRole::圣人).expect("圣人承接");
        board.提交任务(id, AgentRole::圣人, TaskStatus::待大罗金仙实现).expect("圣人提交");

        board.承接任务(id, AgentRole::大罗金仙).expect("大罗金仙承接");
        board.提交任务(id, AgentRole::大罗金仙, TaskStatus::待准圣验收).expect("大罗金仙提交");

        board.承接任务(id, AgentRole::准圣).expect("准圣承接");
        board.提交任务(id, AgentRole::准圣, TaskStatus::待道祖终审).expect("准圣通过");

        board.承接任务(id, AgentRole::道祖).expect("道祖承接");
        board.提交任务(id, AgentRole::道祖, TaskStatus::待修复).expect("道祖终审不通过");

        let task = board.查询(id).expect("任务应存在");
        assert_eq!(task.status, TaskStatus::待修复);
        assert_eq!(task.修复轮次, 1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_筛选按状态和角色() {
        let path = 临时路径("筛选");
        let mut board = TaskBoard::新建(&path);
        let id1 = board.发布任务(造任务("任务一")).expect("发布1");
        let _id2 = board.发布任务(造任务("任务二")).expect("发布2");

        board.承接任务(id1, AgentRole::圣人).expect("圣人承接任务一");

        let 待圣人 = board.筛选(Some(TaskStatus::待圣人设计), None);
        assert_eq!(待圣人.len(), 1);
        assert_eq!(待圣人[0].title, "任务二");

        let 圣人设计中 = board.筛选(Some(TaskStatus::圣人设计中), None);
        assert_eq!(圣人设计中.len(), 1);
        assert_eq!(圣人设计中[0].title, "任务一");

        let 圣人角色 = board.筛选(None, Some(AgentRole::圣人));
        assert_eq!(圣人角色.len(), 1);
        assert_eq!(圣人角色[0].title, "任务一");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_更新需求文档() {
        let path = 临时路径("需求文档");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("需求文档测试")).expect("发布");
        let doc = RequirementDoc {
            目标: "实现任务看板".into(),
            背景: "需要五层协作".into(),
            约束: vec!["依赖单向".into()],
            功能需求: vec!["发布任务".into()],
            非功能需求: vec!["零警告编译".into()],
            验收标准: vec!["测试全绿".into()],
            优先级: TaskPriority::P1,
            created_at: 200,
        };
        board.更新需求文档(id, doc).expect("更新需求文档应成功");
        let task = board.查询(id).expect("任务应存在");
        let doc = task.需求文档.as_ref().expect("需求文档应存在");
        assert_eq!(doc.目标, "实现任务看板");
        assert_eq!(doc.约束, vec!["依赖单向".to_string()]);
        assert_eq!(doc.优先级, TaskPriority::P1);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_更新设计文档() {
        let path = 临时路径("设计文档");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("设计文档测试")).expect("发布");
        let doc = DesignDoc {
            边界定义: std::collections::HashMap::new(),
            安全区域: vec!["鸿蒙层".into()],
            契约: vec![],
            修改文件: vec!["模块.rs".into()],
            新建文件: vec!["看板数据.rs".into()],
            依赖: vec![],
            created_at: 200,
        };
        board.更新设计文档(id, doc).expect("更新设计文档应成功");
        let task = board.查询(id).expect("任务应存在");
        let doc = task.设计文档.as_ref().expect("设计文档应存在");
        assert_eq!(doc.安全区域, vec!["鸿蒙层".to_string()]);
        assert_eq!(doc.修改文件, vec!["模块.rs".to_string()]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_更新实现文档() {
        let path = 临时路径("实现文档");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("实现文档测试")).expect("发布");
        let doc = ImplementationDoc {
            代码变更: vec![],
            工具调用: vec![],
            自检: SelfCheckResult {
                通过: true,
                边界合规: true,
                契约合规: true,
                问题: vec![],
            },
            created_at: 200,
        };
        board.更新实现文档(id, doc).expect("更新实现文档应成功");
        let task = board.查询(id).expect("任务应存在");
        let doc = task.实现文档.as_ref().expect("实现文档应存在");
        assert!(doc.自检.通过);
        assert!(doc.自检.边界合规);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_更新验收文档() {
        let path = 临时路径("验收文档");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("验收文档测试")).expect("发布");
        let doc = VerificationDoc {
            轮次: vec![VerificationRound {
                轮次: 1,
                通过: true,
                边界检查: true,
                契约检查: true,
                安全检查: true,
                事实检查: true,
                完整性检查: true,
                问题: vec![],
                建议: "通过".into(),
            }],
            最终结果: true,
            created_at: 200,
        };
        board.更新验收文档(id, doc).expect("更新验收文档应成功");
        let task = board.查询(id).expect("任务应存在");
        let doc = task.验收文档.as_ref().expect("验收文档应存在");
        assert!(doc.最终结果);
        assert_eq!(doc.轮次.len(), 1);
        assert!(doc.轮次[0].通过);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_更新终审文档() {
        let path = 临时路径("终审文档");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("终审文档测试")).expect("发布");
        let doc = FinalAcceptanceDoc {
            通过: true,
            需求满足度: 95,
            可维护性: 90,
            代码质量: 88,
            风险评估: "低风险".into(),
            评语: "质量良好".into(),
            created_at: 200,
        };
        board.更新终审文档(id, doc).expect("更新终审文档应成功");
        let task = board.查询(id).expect("任务应存在");
        let doc = task.终审文档.as_ref().expect("终审文档应存在");
        assert!(doc.通过);
        assert_eq!(doc.需求满足度, 95);
        assert_eq!(doc.风险评估, "低风险");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_jsonl持久化往返() {
        let path = 临时路径("持久化");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("持久化测试")).expect("发布");
        board.承接任务(id, AgentRole::圣人).expect("圣人承接");

        let loaded = TaskBoard::加载(&path).expect("加载应成功");
        let task = loaded.查询(id).expect("任务应存在");
        assert_eq!(task.title, "持久化测试");
        assert_eq!(task.status, TaskStatus::圣人设计中);
        assert_eq!(task.当前承接人, AgentRole::圣人);
        assert_eq!(task.承接历史, vec![AgentRole::圣人]);
        assert_eq!(task.状态历史.len(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_加载不存在的文件返回空看板() {
        let path = 临时路径("不存在");
        let board = TaskBoard::加载(&path).expect("加载应成功");
        assert_eq!(board.全部().len(), 0);
    }

    #[test]
    fn 看板_提交时角色不符被拒绝() {
        let path = 临时路径("提交角色不符");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("提交角色不符测试")).expect("发布");
        board.承接任务(id, AgentRole::圣人).expect("圣人承接");
        let result = board.提交任务(id, AgentRole::大罗金仙, TaskStatus::待大罗金仙实现);
        assert!(result.is_err());
        assert!(result.expect_err("应报错").to_string().contains("角色不符"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 看板_非法状态流转被拒绝() {
        let path = 临时路径("非法流转");
        let mut board = TaskBoard::新建(&path);
        let id = board.发布任务(造任务("非法流转测试")).expect("发布");
        board.承接任务(id, AgentRole::圣人).expect("圣人承接");
        let result = board.提交任务(id, AgentRole::圣人, TaskStatus::已完成);
        assert!(result.is_err());
        assert!(result.expect_err("应报错").to_string().contains("非法"));
        let task = board.查询(id).expect("任务应存在");
        assert_eq!(task.status, TaskStatus::圣人设计中, "状态不应改变");
        std::fs::remove_file(&path).ok();
    }
}
