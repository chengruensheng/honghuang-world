#[cfg(test)]
mod tests {
    use tc_task::{
        AgentRole, Task, TaskBoard, TaskStatus,
        任务标识, 层级记录, 层级记录状态, 产物记录, 回退记录, 五行层级, ImplementationDoc,
    };

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("tc_task_taskid_test_{名}.jsonl"))
            .to_string_lossy()
            .into_owned()
    }

    /// 测试1：任务标识生成——UUID 唯一、时间戳正确、父任务/依赖任务记录正确
    #[test]
    fn 任务标识生成_uuid唯一且元数据正确() {
        let a = 任务标识::生成("任务A", None, Vec::new());
        let b = 任务标识::生成("任务B", Some(a.任务id), vec![a.任务id]);

        assert_ne!(a.任务id, b.任务id, "两次生成的 UUID 必须唯一");
        assert_eq!(a.任务名, "任务A");
        assert!(a.发布时间 > 0, "发布时间应为有效时间戳");
        assert!(b.发布时间 >= a.发布时间, "时间线连续");
        assert_eq!(a.父任务id, None);
        assert_eq!(b.父任务id, Some(a.任务id), "父任务记录正确");
        assert_eq!(b.依赖任务, vec![a.任务id], "依赖任务记录正确");
    }

    /// 测试2：层级记录——发布→承接→提交 产生完整层级历史，时间线连续
    #[test]
    fn 层级历史_发布承接提交时间线连续() {
        let mut 看板 = TaskBoard::新建(临时路径("层级"));
        let mut 新任务 = Task::新建(0, "层级任务".into(), "描述".into(), 100);
        新任务.status = TaskStatus::待圣人设计;
        let id = 看板.发布任务(新任务).unwrap();
        {
            let 任务 = 看板.查询(id).unwrap();
            assert!(!任务.任务标识.未初始化(), "发布后任务标识应已生成");
            assert_eq!(任务.当前层级, 五行层级::木, "发布后当前层级为木");
            assert!(任务.层级历史.is_empty(), "发布本身不产生层级记录");
        }

        看板.承接任务(id, AgentRole::圣人).unwrap();
        看板.提交任务(id, AgentRole::圣人, TaskStatus::待大罗金仙实现).unwrap();
        看板.承接任务(id, AgentRole::大罗金仙).unwrap();
        看板.提交任务(id, AgentRole::大罗金仙, TaskStatus::待准圣验收).unwrap();

        let 任务 = 看板.查询(id).unwrap();
        assert_eq!(任务.层级历史.len(), 2, "两次承接产生两条层级记录");
        assert_eq!(任务.层级历史[0].层级, 五行层级::火, "圣人为火层级");
        assert_eq!(任务.层级历史[0].状态, 层级记录状态::已完成);
        assert!(任务.层级历史[0].完成时间.is_some());
        assert!(
            任务.层级历史[0].完成时间.unwrap() >= 任务.层级历史[0].开始时间,
            "完成时间不早于开始时间"
        );
        assert_eq!(任务.层级历史[0].处理者id, "圣人");
        assert_eq!(任务.层级历史[1].层级, 五行层级::土, "大罗金仙为土层级");
        assert_eq!(任务.当前层级, 五行层级::土, "提交后当前层级更新为土");
    }

    /// 测试3：产物追溯——按文件路径查到具体层级、处理者、时间
    #[test]
    fn 产物追溯_按文件路径查到层级处理者() {
        let 标识 = 任务标识::生成("产物任务", None, Vec::new());
        let 记录 = 层级记录::开始(五行层级::土, "大罗金仙", 100)
            .完成(200, vec![产物记录::新("src/加法.rs", "实现加法", 标识.任务id)], "提交");
        let 历史 = vec![记录.clone()];

        let 找到 = 标识.查找产物(&历史, "src/加法.rs").unwrap();
        assert_eq!(找到.内容摘要, "实现加法");
        assert_eq!(找到.关联任务id, 标识.任务id);
        assert_eq!(记录.处理者id, "大罗金仙");
        assert_eq!(记录.完成时间, Some(200));
        // 不存在的路径查不到
        assert!(标识.查找产物(&历史, "src/不存在.rs").is_none());
    }

    /// 测试3b：TaskBoard 集成——大罗金仙提交时按实现文档自动生成产物记录
    #[test]
    fn 产物追溯_大罗金仙提交自动生成产物() {
        let mut 看板 = TaskBoard::新建(临时路径("产物"));
        let mut 新任务 = Task::新建(0, "集成产物".into(), "描述".into(), 100);
        新任务.status = TaskStatus::待圣人设计;
        let id = 看板.发布任务(新任务).unwrap();

        看板.承接任务(id, AgentRole::圣人).unwrap();
        看板.提交任务(id, AgentRole::圣人, TaskStatus::待大罗金仙实现).unwrap();
        看板.承接任务(id, AgentRole::大罗金仙).unwrap();
        let 实现: ImplementationDoc = serde_json::from_str(
            r#"{"代码变更":[{"文件路径":"src/加法.rs","变更类型":"新增","摘要":"实现加法函数"}],"自检":{"通过":true,"边界合规":true,"契约合规":true},"created_at":100}"#,
        )
        .unwrap();
        看板.更新实现文档(id, 实现).unwrap();
        看板.提交任务(id, AgentRole::大罗金仙, TaskStatus::待准圣验收).unwrap();

        let 任务 = 看板.查询(id).unwrap();
        let 标识 = &任务.任务标识;
        let 找到 = 标识.查找产物(&任务.层级历史, "src/加法.rs").unwrap();
        assert_eq!(找到.内容摘要, "实现加法函数");
        let 土记录 = 任务.层级历史.iter().find(|r| r.层级 == 五行层级::土).unwrap();
        assert_eq!(土记录.处理者id, "大罗金仙");
        assert!(土记录.完成时间.is_some(), "土层级记录应有完成时间");
    }

    /// 测试4：旧数据兼容——旧 JSONL（无新字段）可正常加载，新字段为默认值
    #[test]
    fn 旧数据兼容_加载后新字段默认值() {
        let 路径 = 临时路径("旧数据");
        std::fs::write(&路径, r#"{"id":1,"title":"旧任务","description":"d","status":"待受理","created_at":100}"#).unwrap();
        let 看板 = TaskBoard::加载(&路径).unwrap();
        let 任务 = 看板.查询(1).unwrap();
        assert!(任务.任务标识.未初始化(), "旧数据任务标识为空");
        assert!(任务.层级历史.is_empty());
        assert_eq!(任务.回退来源, None);
        assert!(!任务.召回标记);
        assert_eq!(任务.当前层级, 五行层级::木);
    }

    /// 测试5：回退记录——写入后可查询，回退次数递增
    #[test]
    fn 回退记录_写入查询与次数递增() {
        let mut 任务 = Task::新建(1, "回退任务".into(), "描述".into(), 100);
        assert_eq!(任务.回退来源, None);

        任务.回退来源 = Some(回退记录::新(200, 五行层级::金, 五行层级::土, "实现Bug", "函数返回值不对", 1));
        assert_eq!(任务.回退来源.as_ref().unwrap().回退次数, 1);
        assert_eq!(任务.回退来源.as_ref().unwrap().来源层级, 五行层级::金);
        assert_eq!(任务.回退来源.as_ref().unwrap().目标层级, 五行层级::土);

        // 第二次回退：次数递增到 2，历史保留（不覆盖层级历史）
        任务.层级历史 = vec![层级记录::开始(五行层级::土, "大罗金仙", 100)];
        任务.回退来源 = Some(回退记录::新(300, 五行层级::金, 五行层级::土, "实现Bug", "仍不正确", 2));
        assert_eq!(任务.回退来源.as_ref().unwrap().回退次数, 2);
        assert_eq!(任务.层级历史.len(), 1, "回退不覆盖既有层级历史");
    }

    /// 测试6：召回标记——设置后可查询，不影响其他字段
    #[test]
    fn 召回标记_设置查询且不影响其他字段() {
        let mut 任务 = Task::新建(1, "召回任务".into(), "描述".into(), 100);
        assert!(!任务.召回标记);
        任务.召回标记 = true;
        assert!(任务.召回标记);
        assert_eq!(任务.title, "召回任务", "召回不影响任务名");
        assert_eq!(任务.status, TaskStatus::待受理, "召回不影响状态");
        assert!(任务.状态历史.is_empty(), "召回不影响状态历史");
        assert_eq!(任务.当前层级, 五行层级::木);
    }

    /// 测试7：按 UUID 查询与改写（TaskBoard 新增 API）
    #[test]
    fn 按标识查询与改写() {
        let mut 看板 = TaskBoard::新建(临时路径("查询标识"));
        let id = 看板.发布任务(Task::新建(0, "标识查询".into(), "描述".into(), 100)).unwrap();
        let uuid = 看板.查询(id).unwrap().任务标识.任务id;

        assert_eq!(看板.查询标识(&uuid).unwrap().id, id, "按 UUID 查到同一任务");
        看板.按标识改写(&uuid, |t| {
            t.召回标记 = true;
            t.回退来源 = Some(回退记录::新(300, 五行层级::金, 五行层级::火, "设计缺陷", "循环依赖", 1));
        })
        .unwrap();
        let 任务 = 看板.查询(id).unwrap();
        assert!(任务.召回标记, "按 UUID 改写生效");
        assert_eq!(任务.回退来源.as_ref().unwrap().目标层级, 五行层级::火);
    }

    /// 测试8：追溯路径——返回任务经过的层级路径
    #[test]
    fn 追溯路径_返回经过层级() {
        let 标识 = 任务标识::生成("路径任务", None, Vec::new());
        let 历史 = vec![
            层级记录::开始(五行层级::火, "圣人", 100),
            层级记录::开始(五行层级::土, "大罗金仙", 200),
            层级记录::开始(五行层级::金, "准圣", 300),
        ];
        assert_eq!(标识.追溯路径(&历史), vec![五行层级::火, 五行层级::土, 五行层级::金]);
    }
}
