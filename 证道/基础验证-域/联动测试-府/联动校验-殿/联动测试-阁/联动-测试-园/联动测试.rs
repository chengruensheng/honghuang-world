#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Arc, Mutex};
#[cfg(test)]
use hm_linkage::五行装配;
#[cfg(test)]
use hm_domain_contract::任务仓库契约;
#[cfg(test)]
use hm_contract::Component;
#[cfg(test)]
use tc_task::{Task, TaskStatus};
#[cfg(test)]
use lj_iteration::{Version, 迭代状态};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 木生火_任务完成开启迭代() {
        let 装配 = 五行装配::装配();
        let 任务 = 装配.任务仓库.lock().unwrap().创建("任务一".into(), "验证诞生".into()).unwrap();
        装配.任务仓库.lock().unwrap().推进(任务, TaskStatus::进行中).unwrap();
        装配.任务仓库.lock().unwrap().推进(任务, TaskStatus::已完成).unwrap();
        let 迭代日志锁 = 装配.迭代日志.lock().unwrap();
        let 迭代 = 迭代日志锁.全部();
        assert_eq!(迭代.len(), 1);
        assert_eq!(迭代[0].变更说明, "任务完成: 任务一：验证诞生");
    }

    #[test]
    fn 火生土_迭代完成写入记忆() {
        let 装配 = 五行装配::装配();
        let 迭代 = 装配.迭代日志.lock().unwrap().开启(Version::new(1, 0, 0), "迭代一".into()).unwrap();
        装配.迭代日志.lock().unwrap().完成(迭代).unwrap();
        let 记忆库锁 = 装配.记忆库.lock().unwrap();
        let 记忆 = 记忆库锁.全部();
        assert_eq!(记忆.len(), 1);
        assert_eq!(记忆[0].内容, "迭代一");
        assert_eq!(记忆[0].标签, "迭代产出");
    }

    #[test]
    fn 土生金_记忆写入反馈规则() {
        let 装配 = 五行装配::装配();
        装配.记忆库.lock().unwrap().写入("经验内容".into(), "经验".into()).unwrap();
        let 规则库锁 = 装配.规则库.lock().unwrap();
        let 规则 = 规则库锁.全部();
        assert_eq!(规则.len(), 1);
        assert_eq!(规则[0].名称, "经验");
        assert_eq!(规则[0].结论, "经验内容");
        assert!(!规则[0].条件.is_empty());
    }

    #[test]
    fn 金生水_规则命中触发事件() {
        let 装配 = 五行装配::装配();
        装配.规则库.lock().unwrap().添加规则("规则一", vec![("键".into(), "值".into())], "结论一", 1).unwrap();
        let 规则库 = 装配.规则库.lock().unwrap();
        let 命中 = 规则库.评估(&[("键".into(), "值".into())]);
        assert_eq!(命中.len(), 1);
        let 事件总线锁 = 装配.事件总线.lock().unwrap();
        let 事件 = 事件总线锁.全部();
        assert_eq!(事件.len(), 1);
        assert_eq!(事件[0].类型, "规则一");
    }

    #[test]
    fn 水生木_事件发布创建任务() {
        let 装配 = 五行装配::装配();
        装配.事件总线.lock().unwrap().发布("类型一".into(), vec![("来源".into(), "自动化".into())]);
        let 任务仓库锁 = 装配.任务仓库.lock().unwrap();
        let 任务 = 任务仓库锁.全部();
        assert_eq!(任务.len(), 1);
        assert_eq!(任务[0].title, "事件驱动: 类型一");
        assert_eq!(任务[0].description, "来源=自动化");
    }

    #[test]
    fn 五行相生完整闭环() {
        let 装配 = 五行装配::装配();
        // 木生火：任务完成 → 开启迭代
        let 任务 = 装配.任务仓库.lock().unwrap().创建("闭环任务".into(), "".into()).unwrap();
        装配.任务仓库.lock().unwrap().推进(任务, TaskStatus::进行中).unwrap();
        装配.任务仓库.lock().unwrap().推进(任务, TaskStatus::已完成).unwrap();
        assert_eq!(装配.迭代日志.lock().unwrap().全部().len(), 1);
        // 火生土 + 土生金：迭代完成 → 记忆 + 规则
        let 迭代 = 装配
            .迭代日志
            .lock()
            .unwrap()
            .全部()
            .iter()
            .find(|it| it.status == 迭代状态::进行中)
            .map(|it| it.id)
            .unwrap();
        装配.迭代日志.lock().unwrap().完成(迭代).unwrap();
        assert_eq!(装配.记忆库.lock().unwrap().全部().len(), 1);
        assert_eq!(装配.规则库.lock().unwrap().全部().len(), 1);
        // 金生水 + 水生木：规则命中（条件匹配记忆标签）→ 事件 + 任务
        装配.规则库.lock().unwrap().评估(&[("标签".into(), "迭代产出".into())]);
        assert_eq!(装配.事件总线.lock().unwrap().全部().len(), 1);
        assert_eq!(装配.任务仓库.lock().unwrap().全部().len(), 2);
    }

    #[test]
    fn 运行期注入_mock替换真实任务仓库() {
        struct 模拟任务仓库 {
            创建次数: Arc<AtomicU64>,
        }

        impl Component for 模拟任务仓库 {
            fn name(&self) -> &'static str { "模拟任务仓库" }
        }

        impl 任务仓库契约<Task, TaskStatus> for 模拟任务仓库 {
            fn 创建(&mut self, _标题: String, _描述: String) -> hm_error::Result<u64> {
                self.创建次数.fetch_add(1, Ordering::SeqCst);
                Ok(1)
            }
            fn 查询(&self, _id: u64) -> Option<&Task> { None }
            fn 全部(&self) -> Vec<&Task> { Vec::new() }
            fn 推进(&mut self, _id: u64, _状态: TaskStatus) -> hm_error::Result<()> { Ok(()) }

        }

        let 计数 = Arc::new(AtomicU64::new(0));
        let mock: Arc<Mutex<dyn 任务仓库契约<Task, TaskStatus>>> =
            Arc::new(Mutex::new(模拟任务仓库 { 创建次数: 计数.clone() }));

        // 生产装配体 → 注入 mock 替换真实任务仓库（府可插拔）
        let mut 装配 = 五行装配::装配();
        装配.任务仓库 = mock;

        let id = 装配.任务仓库.lock().unwrap().创建("mock 任务".into(), "".into()).unwrap();
        assert_eq!(id, 1);
        assert_eq!(计数.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn 金克木_任务过盛拒绝创建() {
        let 装配 = 五行装配::装配带上限(2);
        装配.任务仓库.lock().unwrap().创建("任务一".into(), "".into()).unwrap();
        装配.任务仓库.lock().unwrap().创建("任务二".into(), "".into()).unwrap();
        // 待受理任务已达上限 2，第 3 个创建被拒绝（约束增长，而非删除已有）
        let 结果 = 装配.任务仓库.lock().unwrap().创建("任务三".into(), "".into());
        assert!(结果.is_err());
        assert_eq!(装配.任务仓库.lock().unwrap().全部().len(), 2);
    }

    #[test]
    fn 木克土_记忆过盛标注归档() {
        let 装配 = 五行装配::装配带上限(2);
        装配.记忆库.lock().unwrap().写入("记忆一".into(), "演示".into()).unwrap();
        装配.记忆库.lock().unwrap().写入("记忆二".into(), "演示".into()).unwrap();
        装配.记忆库.lock().unwrap().写入("记忆三".into(), "演示".into()).unwrap();
        // 记忆过盛时第 3 个标注待归档（降级，而非删除旧记忆）
        let 归档数 = 装配.记忆库.lock().unwrap().全部().iter().filter(|m| m.归档).count();
        assert_eq!(归档数, 1);
        assert_eq!(装配.记忆库.lock().unwrap().全部().len(), 3);
    }

    #[test]
    fn 土克水_记忆写入去重事件() {
        let 装配 = 五行装配::装配();
        装配.事件总线.lock().unwrap().发布("重复事件".into(), vec![]);
        装配.事件总线.lock().unwrap().发布("重复事件".into(), vec![]);
        装配.事件总线.lock().unwrap().发布("重复事件".into(), vec![]);
        装配.记忆库.lock().unwrap().写入("新记忆".into(), "演示".into()).unwrap();
        // 土克水：记忆写入去重同类型事件（3 → 1）
        assert_eq!(装配.事件总线.lock().unwrap().全部().len(), 1);
    }

    #[test]
    fn 水克火_迭代过盛拒绝开启() {
        let 装配 = 五行装配::装配带上限(2);
        装配.迭代日志.lock().unwrap().开启(Version::new(1, 0, 0), "迭代一".into()).unwrap();
        装配.迭代日志.lock().unwrap().开启(Version::new(1, 0, 0), "迭代二".into()).unwrap();
        // 进行中迭代已达上限 2，第 3 个开启被拒绝
        let 结果 = 装配.迭代日志.lock().unwrap().开启(Version::new(1, 0, 0), "迭代三".into());
        assert!(结果.is_err());
        assert_eq!(装配.迭代日志.lock().unwrap().全部().len(), 2);
    }

    #[test]
    fn 火克金_规则达上限拒绝新增() {
        let 装配 = 五行装配::装配带上限(2);
        装配.规则库.lock().unwrap().添加规则("规则一", vec![("键".into(), "值".into())], "结论一", 1).unwrap();
        装配.规则库.lock().unwrap().添加规则("规则二", vec![("键".into(), "值".into())], "结论二", 1).unwrap();
        // 规则达上限 2，严格数量上限：即使更高优先级也一律拒绝新增
        assert!(装配.规则库.lock().unwrap().添加规则("规则三", vec![("键".into(), "值".into())], "结论三", 1).is_err());
        assert!(装配.规则库.lock().unwrap().添加规则("规则四", vec![("键".into(), "值".into())], "结论四", 9).is_err());
        assert_eq!(装配.规则库.lock().unwrap().全部().len(), 2);
    }

    #[test]
    fn 容器注册五引擎与信号总线() {
        let 装配 = 五行装配::装配();
        let 名 = 装配.容器.组件名();
        assert_eq!(名.len(), 6);
        assert!(名.contains(&"任务仓库"));
        assert!(名.contains(&"迭代日志"));
        assert!(名.contains(&"记忆库"));
        assert!(名.contains(&"规则库"));
        assert!(名.contains(&"事件总线"));
        assert!(名.contains(&"信号总线"));
        // 按名称获取命中
        assert_eq!(装配.容器.按名称获取("任务仓库").unwrap().name(), "任务仓库");
        // 按名称获取未命中返回空
        assert!(装配.容器.按名称获取("不存在的组件").is_none());
    }
}
