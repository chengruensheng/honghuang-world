#[cfg(test)]
mod tests {
    use hm_agent::{召回器, 召回事件状态};
    use tc_task::{Task, TaskBoard, TaskStatus, 任务标识, 任务依赖图};

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("hm_agent_recall_test_{名}.jsonl"))
            .to_string_lossy()
            .into_owned()
    }

    /// 发布一个任务并直接改状态（绕过状态机，用于构造各层级进行中场景；
    /// id 传 0 由看板自动分配，避免与已加载任务撞号）
    fn 发布并改状态(看板: &mut TaskBoard, 序号: u64, 名: &str, 状态: TaskStatus) -> uuid::Uuid {
        let mut 新任务 = Task::新建(0, format!("{名}#{序号}"), "描述".into(), 100);
        新任务.status = 状态;
        let id = 看板.发布任务(新任务).unwrap();
        看板.查询(id).unwrap().任务标识.任务id
    }

    /// 测试1：影响分析——任务 A 被 B、C 依赖 → 返回 [B, C]（按依赖图被依赖链）
    #[test]
    fn 影响分析_被依赖链() {
        let a = 任务标识::生成("A", None, Vec::new());
        let b = 任务标识::生成("B", None, Vec::new());
        let c = 任务标识::生成("C", None, Vec::new());
        let mut 图 = 任务依赖图::新();
        图.添加任务(a.clone());
        图.添加任务(b.clone());
        图.添加任务(c.clone());
        图.添加依赖(b.任务id, a.任务id); // B 依赖 A
        图.添加依赖(c.任务id, a.任务id); // C 依赖 A

        let 影响 = 召回器::新().影响分析(a.任务id, &图);
        assert_eq!(影响.len(), 2);
        assert!(影响.contains(&b.任务id));
        assert!(影响.contains(&c.任务id));
    }

    /// 测试2：影响分析深度链——A←B←C 时 A 回退，B、C 都被召回
    #[test]
    fn 影响分析_传递链() {
        let a = 任务标识::生成("A", None, Vec::new());
        let b = 任务标识::生成("B", None, Vec::new());
        let c = 任务标识::生成("C", None, Vec::new());
        let mut 图 = 任务依赖图::新();
        图.添加任务(a.clone());
        图.添加任务(b.clone());
        图.添加任务(c.clone());
        图.添加依赖(b.任务id, a.任务id);
        图.添加依赖(c.任务id, b.任务id);

        let 影响 = 召回器::新().影响分析(a.任务id, &图);
        assert_eq!(影响.len(), 2, "深度2 传递被依赖应返回 B、C");
        assert!(影响.contains(&b.任务id) && 影响.contains(&c.任务id));
    }

    /// 测试3：执行召回——设计中/实现中/验收中/已完成/清理中 分别置为对应「待重新*」状态
    #[test]
    fn 执行召回_四种状态映射() {
        let mut 看板 = TaskBoard::新建(临时路径("四态"));
        let 设计中 = 发布并改状态(&mut 看板, 0, "设计中", TaskStatus::圣人设计中);
        let 实现中 = 发布并改状态(&mut 看板, 1, "实现中", TaskStatus::大罗金仙实现中);
        let 验收中 = 发布并改状态(&mut 看板, 2, "验收中", TaskStatus::准圣验收中);
        let 已完成 = 发布并改状态(&mut 看板, 3, "已完成", TaskStatus::已完成);
        let 清理中 = 发布并改状态(&mut 看板, 4, "清理中", TaskStatus::清理中);
        let 已取消 = 发布并改状态(&mut 看板, 5, "已取消", TaskStatus::已取消);
        let 触发 = 任务标识::生成("触发", None, vec![设计中]).任务id;

        let 召回器 = 召回器::新();
        let 事件们 = 召回器.执行召回(
            触发,
            vec![设计中, 实现中, 验收中, 已完成, 清理中, 已取消],
            "上游需求变动",
            &mut 看板,
        );

        assert_eq!(事件们.len(), 1, "一次召回产生一个事件");
        let 事件 = &事件们[0];
        assert_eq!(事件.影响任务.len(), 5, "已取消不应被召回");
        assert_eq!(事件.状态, 召回事件状态::召回中);
        assert_eq!(事件.召回原因, "上游需求变动");
        assert_eq!(事件.触发任务id, 触发);

        // 状态与召回标记断言
        let 断言 = |id: uuid::Uuid, 期望: TaskStatus| {
            let t = 看板.查询标识(&id).unwrap();
            assert_eq!(t.status, 期望);
            assert!(t.召回标记, "应打召回标记");
        };
        断言(设计中, TaskStatus::待重新设计);
        断言(实现中, TaskStatus::待重新实现);
        断言(验收中, TaskStatus::待重新验收);
        断言(已完成, TaskStatus::待重新验收);
        断言(清理中, TaskStatus::待重新清理);
        assert!(!看板.查询标识(&已取消).unwrap().召回标记, "已取消任务不召回");
    }

    /// 测试4：重复召回跳过——已召回任务再次执行召回不产生新事件
    #[test]
    fn 执行召回_已召回跳过() {
        let mut 看板 = TaskBoard::新建(临时路径("重复"));
        let 实现中 = 发布并改状态(&mut 看板, 0, "实现中", TaskStatus::大罗金仙实现中);
        let 触发 = 任务标识::生成("触发", None, Vec::new()).任务id;

        let 召回器 = 召回器::新();
        let 事件1 = 召回器.执行召回(触发, vec![实现中], "原因一", &mut 看板);
        assert_eq!(事件1.len(), 1);
        let 事件2 = 召回器.执行召回(触发, vec![实现中], "原因二", &mut 看板);
        assert!(事件2.is_empty(), "已召回任务跳过，不产生新事件");
        assert_eq!(召回器.事件记录.lock().unwrap().len(), 1);
    }

    /// 测试5：解除召回——触发任务重新完成后，被召回任务恢复原状态，事件状态=已解除
    #[test]
    fn 解除召回_恢复状态与事件() {
        let mut 看板 = TaskBoard::新建(临时路径("解除"));
        let 设计中 = 发布并改状态(&mut 看板, 0, "设计中", TaskStatus::圣人设计中);
        let 实现中 = 发布并改状态(&mut 看板, 1, "实现中", TaskStatus::大罗金仙实现中);
        let 验收中 = 发布并改状态(&mut 看板, 2, "验收中", TaskStatus::准圣验收中);
        let 清理中 = 发布并改状态(&mut 看板, 3, "清理中", TaskStatus::清理中);
        let 触发 = 任务标识::生成("触发", None, vec![]).任务id;

        let 召回器 = 召回器::新();
        召回器.执行召回(触发, vec![设计中, 实现中, 验收中, 清理中], "接口变更", &mut 看板);

        // 触发任务完成（模拟重新通过验收）
        let 图 = 看板.构建依赖图();
        let 恢复数 = 召回器.解除召回(触发, &图, &mut 看板);
        assert_eq!(恢复数, 4, "4 个被召回任务全部恢复");

        let 断言 = |id: uuid::Uuid, 期望: TaskStatus| {
            let t = 看板.查询标识(&id).unwrap();
            assert_eq!(t.status, 期望);
            assert!(!t.召回标记, "召回标记应清除");
        };
        断言(设计中, TaskStatus::待圣人设计);
        断言(实现中, TaskStatus::待大罗金仙实现);
        断言(验收中, TaskStatus::待准圣验收);
        断言(清理中, TaskStatus::待清理);

        // 事件状态已解除
        let 记录 = 召回器.事件记录.lock().unwrap();
        assert_eq!(记录[0].状态, 召回事件状态::已解除);
    }

    /// 测试6：TaskBoard 构建依赖图——任务标识依赖任务自动成边
    #[test]
    fn 构建依赖图_从任务集自动成边() {
        let mut 看板 = TaskBoard::新建(临时路径("成边"));
        let a = 发布并改状态(&mut 看板, 0, "A", TaskStatus::待圣人设计);
        let b = 发布并改状态(&mut 看板, 1, "B", TaskStatus::待圣人设计);
        看板.按标识改写(&a, |t| t.任务标识.依赖任务 = vec![b]).unwrap();

        let 图 = 看板.构建依赖图();
        assert!(图.查询依赖(a).contains(&b), "A 依赖 B 成边");
        assert!(图.查询被依赖(b).contains(&a), "B 被 A 依赖");
    }
}
