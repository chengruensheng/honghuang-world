#[cfg(test)]
mod tests {
    use tc_task::{AgentRole, Task, TaskBoard, TaskStatus, 五行层级};

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("tc_task_rollback_test_{名}.jsonl"))
            .to_string_lossy()
            .into_owned()
    }

    /// 发布一个「待圣人设计」任务
    fn 发布设计任务(看板: &mut TaskBoard, 序号: u64, 名: &str) -> u64 {
        let mut 新任务 = Task::新建(序号, 名.into(), "描述".into(), 100);
        新任务.status = TaskStatus::待圣人设计;
        看板.发布任务(新任务).unwrap()
    }

    /// 测试1：根源=土（实现Bug）→ 状态=待修复（过渡期别名），回退记录完整
    #[test]
    fn 定向回退_土_待修复别名且记录完整() {
        let mut 看板 = TaskBoard::新建(临时路径("土"));
        let id = 发布设计任务(&mut 看板, 0, "回退土");

        let (新状态, 次数) = 看板.定向回退(id, 五行层级::土, "函数返回值不对").unwrap();
        assert_eq!(新状态, TaskStatus::待修复);
        assert_eq!(次数, 1);

        let 任务 = 看板.查询(id).unwrap();
        assert_eq!(任务.当前层级, 五行层级::土);
        let 回退 = 任务.回退来源.as_ref().unwrap();
        assert_eq!(回退.来源层级, 五行层级::金);
        assert_eq!(回退.目标层级, 五行层级::土);
        assert_eq!(回退.回退次数, 1);
        assert_eq!(回退.原因, "实现Bug");
        assert_eq!(回退.错误描述, "函数返回值不对");
        assert!(回退.回退时间 >= 100);
        // 状态历史留痕
        assert!(任务.状态历史.iter().any(|s| s.备注.as_deref().unwrap_or("").contains("定向回退")));
    }

    /// 测试2：根源=火（设计缺陷）→ 状态=待圣人设计，当前层级=火
    #[test]
    fn 定向回退_火_待圣人设计() {
        let mut 看板 = TaskBoard::新建(临时路径("火"));
        let id = 发布设计任务(&mut 看板, 0, "回退火");

        let (新状态, _) = 看板.定向回退(id, 五行层级::火, "循环依赖").unwrap();
        assert_eq!(新状态, TaskStatus::待圣人设计);
        let 任务 = 看板.查询(id).unwrap();
        assert_eq!(任务.当前层级, 五行层级::火);
        assert_eq!(任务.回退来源.as_ref().unwrap().原因, "设计缺陷");
        assert_eq!(任务.回退来源.as_ref().unwrap().目标层级, 五行层级::火);
    }

    /// 测试3：根源=木（需求偏差）→ 状态=待道祖澄清，当前层级=木
    #[test]
    fn 定向回退_木_待道祖澄清() {
        let mut 看板 = TaskBoard::新建(临时路径("木"));
        let id = 发布设计任务(&mut 看板, 0, "回退木");

        let (新状态, _) = 看板.定向回退(id, 五行层级::木, "做的不是想要的").unwrap();
        assert_eq!(新状态, TaskStatus::待道祖澄清);
        let 任务 = 看板.查询(id).unwrap();
        assert_eq!(任务.当前层级, 五行层级::木);
        assert_eq!(任务.回退来源.as_ref().unwrap().原因, "需求偏差/回退超限升级道祖");
    }

    /// 测试4：回退次数上限 3 次，第 4 次强制升级木（道祖澄清）
    #[test]
    fn 定向回退_超限三次后强制升级木() {
        let mut 看板 = TaskBoard::新建(临时路径("超限"));
        let id = 发布设计任务(&mut 看板, 0, "反复回退");

        let 状态1 = 看板.定向回退(id, 五行层级::土, "错误1").unwrap();
        assert_eq!(状态1.0, TaskStatus::待修复);
        assert_eq!(状态1.1, 1);
        let 状态2 = 看板.定向回退(id, 五行层级::土, "错误2").unwrap();
        assert_eq!(状态2.1, 2, "回退次数递增");
        let 状态3 = 看板.定向回退(id, 五行层级::土, "错误3").unwrap();
        assert_eq!(状态3.1, 3);
        // 第 4 次仍指定根源=土，但累计 >3 强制升级到木
        let 状态4 = 看板.定向回退(id, 五行层级::土, "错误4").unwrap();
        assert_eq!(状态4.1, 4);
        assert_eq!(状态4.0, TaskStatus::待道祖澄清, "超限强制升级道祖");

        let 任务 = 看板.查询(id).unwrap();
        assert_eq!(任务.当前层级, 五行层级::木);
        assert_eq!(任务.回退来源.as_ref().unwrap().回退次数, 4);
        assert_eq!(任务.回退来源.as_ref().unwrap().目标层级, 五行层级::木);
    }

    /// 测试5：层级历史保留——回退不覆盖既有层级记录
    #[test]
    fn 定向回退_历史保留() {
        let mut 看板 = TaskBoard::新建(临时路径("保留"));
        let id = 发布设计任务(&mut 看板, 0, "历史保留");
        // 先走一层：圣人设计完成
        看板.承接任务(id, AgentRole::圣人).unwrap();
        看板.提交任务(id, AgentRole::圣人, TaskStatus::待大罗金仙实现).unwrap();
        assert_eq!(看板.查询(id).unwrap().层级历史.len(), 1);

        看板.定向回退(id, 五行层级::火, "设计缺陷").unwrap();
        let 任务 = 看板.查询(id).unwrap();
        assert_eq!(任务.层级历史.len(), 1, "回退不覆盖历史，只追加回退记录");
        assert_eq!(任务.层级历史[0].层级, 五行层级::火);
        assert!(任务.回退来源.is_some());
    }

    /// 测试6：回退后重新流转——回退到设计者→重新设计→实现→验收通过→终审，全链路可走通
    #[test]
    fn 回退后重新流转_全链路() {
        let mut 看板 = TaskBoard::新建(临时路径("全链路"));
        let id = 发布设计任务(&mut 看板, 0, "重新流转");

        // 第一轮：设计→实现→验收失败（提交到待修复）
        看板.承接任务(id, AgentRole::圣人).unwrap();
        看板.提交任务(id, AgentRole::圣人, TaskStatus::待大罗金仙实现).unwrap();
        看板.承接任务(id, AgentRole::大罗金仙).unwrap();
        看板.提交任务(id, AgentRole::大罗金仙, TaskStatus::待准圣验收).unwrap();
        看板.承接任务(id, AgentRole::准圣).unwrap();
        看板.提交任务(id, AgentRole::准圣, TaskStatus::待修复).unwrap();

        // 定向回退到设计者
        let (新状态, 次数) = 看板.定向回退(id, 五行层级::火, "循环依赖").unwrap();
        assert_eq!(新状态, TaskStatus::待圣人设计);
        assert_eq!(次数, 1);

        // 第二轮：重新设计→实现→验收通过→进入终审
        看板.承接任务(id, AgentRole::圣人).unwrap();
        看板.提交任务(id, AgentRole::圣人, TaskStatus::待大罗金仙实现).unwrap();
        看板.承接任务(id, AgentRole::大罗金仙).unwrap();
        看板.提交任务(id, AgentRole::大罗金仙, TaskStatus::待准圣验收).unwrap();
        看板.承接任务(id, AgentRole::准圣).unwrap();
        看板.提交任务(id, AgentRole::准圣, TaskStatus::待道祖终审).unwrap();

        let 任务 = 看板.查询(id).unwrap();
        assert_eq!(任务.status, TaskStatus::待道祖终审, "验收通过后进入终审");
        let 火记录数 = 任务.层级历史.iter().filter(|r| r.层级 == 五行层级::火).count();
        assert_eq!(火记录数, 2, "两轮设计产生两条火层级记录");
        let 土记录数 = 任务.层级历史.iter().filter(|r| r.层级 == 五行层级::土).count();
        assert_eq!(土记录数, 2);
        // 回退记录保留，目标=火
        let 回退 = 任务.回退来源.as_ref().unwrap();
        assert_eq!(回退.目标层级, 五行层级::火);
        assert_eq!(回退.回退次数, 1);
        // 状态历史包含完整流转
        assert!(任务.状态历史.len() >= 8, "多次流转状态留痕");
    }
}
