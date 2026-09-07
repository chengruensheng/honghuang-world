#[cfg(test)]
mod tests {
    use tc_task::{任务标识, 任务依赖图};

    /// 测试1：依赖图查询——添加任务和边后，查询依赖/被依赖正确
    #[test]
    fn 依赖图_查询依赖与被依赖() {
        let a = 任务标识::生成("任务A", None, Vec::new());
        let b = 任务标识::生成("任务B", None, Vec::new());
        let c = 任务标识::生成("任务C", None, Vec::new());
        let mut 图 = 任务依赖图::新();
        图.添加任务(a.clone());
        图.添加任务(b.clone());
        图.添加任务(c.clone());
        图.添加依赖(a.任务id, b.任务id); // A 依赖 B
        图.添加依赖(c.任务id, b.任务id); // C 依赖 B

        assert_eq!(图.查询依赖(a.任务id), vec![b.任务id], "A 依赖谁");
        assert_eq!(图.查询依赖(c.任务id), vec![b.任务id]);
        assert!(图.查询依赖(b.任务id).is_empty(), "B 不依赖任何人");
        assert_eq!(图.查询被依赖(b.任务id).len(), 2, "B 被 A、C 依赖");
        assert!(图.查询被依赖(a.任务id).is_empty());
    }

    /// 测试2：传递被依赖——深度2递归查询正确，去重
    #[test]
    fn 依赖图_传递被依赖深度二去重() {
        let a = 任务标识::生成("任务A", None, Vec::new());
        let b = 任务标识::生成("任务B", None, Vec::new());
        let c = 任务标识::生成("任务C", None, Vec::new());
        let mut 图 = 任务依赖图::新();
        图.添加任务(a.clone());
        图.添加任务(b.clone());
        图.添加任务(c.clone());
        图.添加依赖(b.任务id, a.任务id); // B 依赖 A
        图.添加依赖(c.任务id, b.任务id); // C 依赖 B

        let 结果 = 图.查询传递被依赖(a.任务id, 2);
        assert_eq!(结果.len(), 2, "A 的上游链为 B、C");
        assert!(结果.contains(&b.任务id));
        assert!(结果.contains(&c.任务id));

        // 深度1 只返回直接被依赖
        let 深度1 = 图.查询传递被依赖(a.任务id, 1);
        assert_eq!(深度1, vec![b.任务id]);

        // 去重：两条路径指向同一任务时不重复
        图.添加依赖(c.任务id, a.任务id); // C 也直接依赖 A
        let 去重 = 图.查询传递被依赖(a.任务id, 2);
        assert_eq!(去重.len(), 2, "C 出现两次应去重");
    }

    /// 测试3：循环依赖检测——A 依赖 B、B 依赖 A → 检测到循环
    #[test]
    fn 依赖图_循环依赖检测() {
        let a = 任务标识::生成("任务A", None, Vec::new());
        let b = 任务标识::生成("任务B", None, Vec::new());
        let mut 图 = 任务依赖图::新();
        图.添加任务(a.clone());
        图.添加任务(b.clone());
        图.添加依赖(a.任务id, b.任务id);
        图.添加依赖(b.任务id, a.任务id);

        let 环 = 图.检测循环依赖();
        assert!(环.is_some(), "A↔B 应检测到循环");
        let 环 = 环.unwrap();
        assert!(环.len() >= 3, "环路径应含起点重复（A→B→A）");
        assert!(环.contains(&a.任务id) && 环.contains(&b.任务id));

        // 无环时返回 None
        let mut 图2 = 任务依赖图::新();
        图2.添加任务(a.clone());
        图2.添加任务(b.clone());
        图2.添加依赖(a.任务id, b.任务id);
        assert!(图2.检测循环依赖().is_none(), "单向依赖无环");
    }

    /// 测试4：移除任务——节点与相关边一并移除
    #[test]
    fn 依赖图_移除任务() {
        let a = 任务标识::生成("任务A", None, Vec::new());
        let b = 任务标识::生成("任务B", None, Vec::new());
        let mut 图 = 任务依赖图::新();
        图.添加任务(a.clone());
        图.添加任务(b.clone());
        图.添加依赖(a.任务id, b.任务id);
        图.移除任务(a.任务id);
        assert!(图.查询依赖(a.任务id).is_empty());
        assert!(图.查询被依赖(b.任务id).is_empty(), "相关边一并移除");
        assert!(!图.节点.contains_key(&a.任务id));
        assert!(图.节点.contains_key(&b.任务id));
    }
}
