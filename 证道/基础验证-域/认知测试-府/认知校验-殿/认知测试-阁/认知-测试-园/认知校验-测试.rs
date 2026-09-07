#[cfg(test)]
mod tests {
    use hm_cognition::{
        ToolCallRecord,
        依赖边, 图谱, 符号, 符号种类, 模块, 维度, 格位, 心智地图, 消息角色,
        默认三十六格位, 注入器, 压缩器, 上下文库, 纠错引擎,
        规则提炼器, 上下文压缩阈值, 压缩滑动窗口, 晋升重复阈值, 迭代线索,
    };

    #[test]
    fn 工具调用记录_序列化往返保持一致() {
        let record = ToolCallRecord {
            工具名: "edit_file".into(),
            参数: "src/lib.rs".into(),
            结果摘要: "修改了3行".into(),
            时间戳: 999,
        };
        let json = serde_json::to_string(&record).expect("序列化应成功");
        let 还原: ToolCallRecord = serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(还原.工具名, "edit_file");
        assert_eq!(还原.参数, "src/lib.rs");
        assert_eq!(还原.结果摘要, "修改了3行");
        assert_eq!(还原.时间戳, 999);
    }
    #[test]
    fn 心智地图_默认三十六格位骨架完整() {
        let 地图 = 心智地图::默认三十六格位();
        assert_eq!(地图.格位集.len(), 36);
        for 维度值 in 维度::全部维度() {
            let 每维 = 地图.按维度查询(维度值);
            assert_eq!(每维.len(), 6, "每个维度应有 6 个格位");
        }
        assert!(地图.全部().iter().all(|格位| 格位.摘要.is_empty()));
        assert!(地图.全部().iter().all(|格位| 格位.可信度 == 0.0));
    }

    #[test]
    fn 格位_摘要超限截断() {
        let 长摘要: String = "规则".repeat(40);
        let 格位 = 格位::新(维度::规则, "禁止", 长摘要.clone(), 0.9, vec![], None);
        assert_eq!(格位.摘要.chars().count(), 80, "规则维度摘要应截断到 80 字");
    }

    #[test]
    fn 格位_可信度越界夹紧() {
        let 高 = 格位::新(维度::规则, "禁止", "x", 1.5, vec![], None);
        assert_eq!(高.可信度, 1.0);
        let 低 = 格位::新(维度::规则, "禁止", "x", -0.2, vec![], None);
        assert_eq!(低.可信度, 0.0);
    }

    #[test]
    fn 够用判定_全达标() {
        let mut 地图 = 心智地图::新();
        地图.写(维度::外在, "结构", "项目共 5 个模块", 0.8, vec!["模块".into()])
            .expect("写入应成功");
        let 信号 = 地图.够用(维度::外在, "结构");
        assert!(信号.够用(), "全达标应够用");
        assert!(信号.不达标项.is_empty());
    }

    #[test]
    fn 够用判定_置信度不足() {
        let mut 地图 = 心智地图::新();
        地图.写(维度::规则, "禁止", "禁止删除数据", 0.90, vec!["证据".into()])
            .expect("写入应成功");
        let 信号 = 地图.够用(维度::规则, "禁止");
        assert!(!信号.够用(), "规则 0.90 < 0.95 应不够用");
        assert!(信号.不达标项.iter().any(|项| 项.contains("S2")));
    }

    #[test]
    fn 够用判定_无证据() {
        let mut 地图 = 心智地图::新();
        地图.写(维度::执行, "命令", "cargo build", 0.9, vec![]).expect("写入应成功");
        let 信号 = 地图.够用(维度::执行, "命令");
        assert!(!信号.够用());
        assert!(信号.不达标项.iter().any(|项| 项.contains("S4")));
    }

    #[test]
    fn 够用判定_冷却期() {
        let mut 地图 = 心智地图::新();
        地图.写(维度::执行, "命令", "cargo build", 0.9, vec!["证".into()])
            .expect("写入应成功");
        地图.进入冷却期(维度::执行, "命令", 5).expect("进入冷却期应成功");
        let 信号 = 地图.够用(维度::执行, "命令");
        assert!(!信号.够用());
        assert!(信号.不达标项.iter().any(|项| 项.contains("S5")));
    }

    #[test]
    fn 够用判定_格位不存在() {
        let 地图 = 心智地图::新();
        let 信号 = 地图.够用(维度::内部, "不存在");
        assert!(!信号.够用());
        assert!(信号.不达标项.iter().any(|项| 项.contains("格位不存在")));
    }

    #[test]
    fn 上下文库_追加与硬上限() {
        let mut 库 = 上下文库::新_带上限(3);
        库.追加(消息角色::用户, "第一条");
        库.追加(消息角色::助手, "第二条");
        库.追加(消息角色::用户, "第三条");
        assert_eq!(库.长度(), 3);
        库.追加(消息角色::助手, "第四条");
        assert_eq!(库.长度(), 3, "超硬上限应丢最旧");
        assert_eq!(库.全部()[0].内容, "第二条");
        assert!(!库.全部().iter().any(|消息| 消息.内容 == "第一条"));
    }

    #[test]
    fn 上下文库_最近相关按关键词() {
        let mut 库 = 上下文库::新();
        库.追加(消息角色::助手, "修复了序列化错误");
        库.追加(消息角色::助手, "完成事件联动");
        let 命中 = 库.最近相关("序列化", 5);
        assert_eq!(命中.len(), 1);
        assert_eq!(命中[0].内容, "修复了序列化错误");
    }

    #[test]
    fn 压缩_超阈值保留窗口() {
        let mut 库 = 上下文库::新();
        let mut 地图 = 心智地图::默认三十六格位();
        for i in 0..上下文压缩阈值 + 5 {
            库.追加(消息角色::用户, format!("消息 {i} 完成"));
        }
        let mut 压缩器 = 压缩器::新(&mut 库, &mut 地图);
        let 晋升 = 压缩器.手动压缩();
        assert_eq!(库.长度(), 压缩滑动窗口 + 1, "滑动窗口 + 1 条压缩摘要");
        assert!(库.全部()[0].内容.starts_with("[压缩摘要]"));
        assert!(晋升.is_empty() || 晋升.iter().all(|记录| 记录.目标格位.1 == "教训"));
    }

    #[test]
    fn 压缩_阶段3重复模式晋升() {
        let mut 库 = 上下文库::新();
        let mut 地图 = 心智地图::默认三十六格位();
        for _ in 0..晋升重复阈值 {
            库.追加(消息角色::助手, "重复出现的错误模式 超时重试");
        }
        for i in 0..压缩滑动窗口 + 5 {
            库.追加(消息角色::助手, format!("正常步骤 {i}"));
        }
        let mut 压缩器 = 压缩器::新(&mut 库, &mut 地图);
        let 晋升 = 压缩器.手动压缩();
        assert!(!晋升.is_empty(), "重复模式应晋升");
        drop(压缩器);
        let 教训 = 地图.查询格位(维度::经历, "教训").expect("教训格位应被写入");
        assert!(教训.摘要.contains("反复出现的模式"));
        assert_eq!(教训.可信度, 0.7);
    }

    #[test]
    fn 压缩_空消息() {
        let mut 库 = 上下文库::新();
        let mut 地图 = 心智地图::默认三十六格位();
        let mut 压缩器 = 压缩器::新(&mut 库, &mut 地图);
        let 晋升 = 压缩器.手动压缩();
        assert!(晋升.is_empty());
    }

    #[test]
    fn 注入_推流拉三种模式() {
        let mut 图 = 图谱::新();
        图.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动".into() });
        图.添加符号(符号 {
            名称: "桥接运行".into(),
            种类: 符号种类::函数,
            所属模块: "hm-linkage".into(),
            签名: None,
        });
        let mut 地图 = 心智地图::默认三十六格位();
        地图.写(维度::外在, "结构", "项目共 1 个模块", 0.9, vec!["hm-linkage".into()])
            .expect("写入应成功");
        let mut 库 = 上下文库::新();
        库.追加(消息角色::助手, "最近的一条执行记录");

        let 注入器 = 注入器::新(&图, &地图, &库);
        let 推 = 注入器.推_格位摘要();
        assert!(!推.is_empty());
        assert!(推.iter().all(|片段| 片段.来源 == "格位"));
        assert_eq!(推[0].内容.contains("外在·结构"), true);

        let 流 = 注入器.流_最近消息();
        assert!(流.iter().all(|片段| 片段.来源 == "临时"));
        assert_eq!(流[0].内容, "最近的一条执行记录");

        let 拉 = 注入器.拉_图谱片段("hm-linkage", 5);
        assert!(!拉.is_empty());
        assert!(拉.iter().all(|片段| 片段.来源 == "图谱"));
    }

    #[test]
    fn 纠错_七步闭环() {
        let 图 = 图谱::新();
        let mut 地图 = 心智地图::默认三十六格位();
        地图.写(维度::执行, "命令", "cargo build", 0.9, vec!["旧证据".into()])
            .expect("写入应成功");
        let mut 库 = 上下文库::新();
        let mut 引擎 = 纠错引擎::新(&图, &mut 地图, &mut 库);
        let 事件 = 引擎.纠正(
            维度::执行,
            "命令",
            "cargo build --workspace",
            vec!["新证据".into()],
            "",
        );
        assert_eq!(事件.新可信度, 0.7, "置信度应降至 0.7");
        assert_eq!(引擎.事件序列.len(), 1);
        assert!(引擎.事件序列[0].检测到的差异.contains("摘要差异"));
        let 事件时间 = 事件.时间;
        drop(引擎);
        let 格位 = 地图.查询格位(维度::执行, "命令").expect("格位应存在");
        assert_eq!(格位.摘要, "cargo build --workspace");
        assert!(格位.处于冷却期(事件时间), "纠错后应进入冷却期");
        assert!(库.全部().iter().any(|消息| 消息.内容.contains("[纠错·执行·命令]")));
    }

    #[test]
    fn 纠错_同类触发教训() {
        let 图 = 图谱::新();
        let mut 地图 = 心智地图::默认三十六格位();
        let mut 库 = 上下文库::新();
        let mut 引擎 = 纠错引擎::新(&图, &mut 地图, &mut 库);
        for _ in 0..晋升重复阈值 {
            引擎.纠正(维度::执行, "命令", "cargo test --workspace", vec![], "");
        }
        drop(引擎);
        let 教训 = 地图.查询格位(维度::经历, "教训").expect("同类纠错应生成教训");
        assert!(教训.摘要.contains("同类纠错"));
    }

    #[test]
    fn 提炼_规则提炼器各格位() {
        let mut 图 = 图谱::新();
        图.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动".into() });
        图.添加模块(模块 { 名称: "hm-error".into(), 路径: "鸿蒙/契约".into() });
        图.添加依赖(依赖边 { 源: "hm-linkage".into(), 目标: "hm-error".into() });
        图.添加技术栈("serde");
        图.添加技术栈("tokio");
        let 提炼器 = 规则提炼器;
        let 候选 = 提炼器.提炼(&图);
        let 结构 = 候选.iter().find(|c| c.格位名 == "结构").expect("应有结构");
        assert!(结构.摘要.contains("2 个模块"));
        let 依赖 = 候选.iter().find(|c| c.格位名 == "依赖").expect("应有依赖");
        assert!(依赖.摘要.contains("hm-linkage→hm-error"));
        let 技术栈 = 候选.iter().find(|c| c.格位名 == "技术栈").expect("应有技术栈");
        assert!(技术栈.摘要.contains("tokio"));
        let 工具 = 候选.iter().find(|c| c.格位名 == "工具").expect("应有工具");
        assert!(工具.摘要.contains("serde"));
    }

    #[test]
    fn 提炼_空图谱() {
        let 图 = 图谱::新();
        let 提炼器 = 规则提炼器;
        let 候选 = 提炼器.提炼(&图);
        assert!(候选.is_empty(), "空图谱应无候选摘要");
    }

    #[test]
    fn 提炼器_写入心智格位成功() {
        let mut 图 = 图谱::新();
        图.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动".into() });
        图.添加技术栈("serde");
        let 提炼器 = 规则提炼器;
        let 候选 = 提炼器.提炼(&图);
        assert!(!候选.is_empty(), "建图后应有候选摘要");
        let mut 地图 = 心智地图::默认三十六格位();
        for 摘要 in &候选 {
            地图.写(摘要.维度.clone(), 摘要.格位名.as_str(), 摘要.摘要.clone(), 摘要.可信度, 摘要.证据引用.clone())
                .expect("提炼摘要应能写入心智格位");
        }
        let 格位 = 地图.查询格位(维度::外在, "结构").expect("结构格位应写入");
        assert!(格位.摘要.contains("hm-linkage"), "结构格位摘要应含模块名");
    }

    #[test]
    fn 纠错_联动下调引用同源格位() {
        let 图 = 图谱::新();
        let mut 地图 = 心智地图::默认三十六格位();
        地图.写(维度::执行, "命令", "cargo build", 0.9, vec!["共同证据".into()])
            .expect("写入命令");
        地图.写(维度::执行, "验证", "cargo test", 0.85, vec!["共同证据".into()])
            .expect("写入验证");
        地图.写(维度::执行, "步骤", "先编译", 0.8, vec!["独有证据".into()])
            .expect("写入步骤");
        let mut 库 = 上下文库::新();
        let mut 引擎 = 纠错引擎::新(&图, &mut 地图, &mut 库);
        let 事件 = 引擎.纠正(
            维度::执行,
            "命令",
            "cargo build --workspace",
            vec!["共同证据".into()],
            "",
        );
        assert!(
            事件.联动链.iter().any(|键| 键.contains("验证")),
            "联动链应含引用同源的验证格位: {:?}",
            事件.联动链
        );
        assert!(
            !事件.联动链.iter().any(|键| 键.contains("步骤")),
            "无交集的步骤不应被下调: {:?}",
            事件.联动链
        );
        drop(引擎);
        let 验证 = 地图.查询格位(维度::执行, "验证").expect("验证格位应存在");
        assert!(
            (验证.可信度 - 0.75).abs() < 1e-6,
            "引用同源格位应下调 0.1（0.85→0.75），实际 {}",
            验证.可信度
        );
    }

    #[test]
    fn 提炼_带历史覆盖目标经历维度() {
        let mut 图 = 图谱::新();
        图.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动".into() });
        图.添加模块(模块 { 名称: "hm-error".into(), 路径: "鸿蒙/契约".into() });
        图.添加依赖(依赖边 { 源: "hm-linkage".into(), 目标: "hm-error".into() });
        图.添加符号(符号 {
            名称: "运行".into(),
            种类: 符号种类::函数,
            所属模块: "hm-agent".into(),
            签名: Some("fn(任务)".into()),
        });
        图.添加技术栈("serde");
        let 迭代们 = vec![
            迭代线索 { 版本: "v1.54".into(), 状态: "已完成".into(), 变更说明: "Resume/Fork".into(), 时间: 100 },
            迭代线索 { 版本: "v1.55".into(), 状态: "已完成".into(), 变更说明: "治理量劫".into(), 时间: 200 },
            迭代线索 { 版本: "v1.56".into(), 状态: "已放弃".into(), 变更说明: "失败尝试".into(), 时间: 300 },
        ];
        let 提炼器 = 规则提炼器;
        let 候选 = 提炼器.提炼_带历史(&图, &迭代们);
        assert!(候选.len() >= 12, "带历史应提炼 ≥12 格位，实际 {}", 候选.len());
        let 现况 = 候选.iter().find(|c| c.格位名 == "现况").expect("应有目标·现况");
        assert!(现况.摘要.contains("v1.55"), "现况应为最新已完成 v1.55: {}", 现况.摘要);
        let 偏移 = 候选.iter().find(|c| c.格位名 == "偏移").expect("应有目标·偏移");
        assert!(偏移.摘要.contains("v1.56"), "偏移应含已放弃 v1.56: {}", 偏移.摘要);
        let 教训 = 候选.iter().find(|c| c.格位名 == "教训").expect("应有经历·教训");
        assert!(教训.摘要.contains("失败尝试"), "教训应含放弃结论: {}", 教训.摘要);
    }

    #[test]
    fn 提炼_带历史无历史不产出目标经历() {
        let mut 图 = 图谱::新();
        图.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动".into() });
        图.添加技术栈("serde");
        let 提炼器 = 规则提炼器;
        let 候选 = 提炼器.提炼_带历史(&图, &[]);
        assert!(
            候选.iter().all(|c| c.维度 != 维度::目标 && c.维度 != 维度::经历),
            "无历史时不应产出目标/经历维度: {:?}",
            候选.iter().map(|c| format!("{}·{}", c.维度.中文名(), c.格位名)).collect::<Vec<_>>()
        );
    }
}
