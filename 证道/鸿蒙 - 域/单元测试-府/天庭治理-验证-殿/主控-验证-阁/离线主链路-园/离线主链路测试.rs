//! 离线主链路-园：主链路机械段零 token 回归（生产化 4.1）。
//! 覆盖 任务线全生命周期 / 状态机机械链 / 读时聚合——不调 LLM、不碰网络、不写真实工作区。

#[cfg(test)]
mod 测试 {
    use crate::道术施展_验证_殿::手脚_验证_阁::隔离_互斥_园::隔离设施::设施::临时工作区;
    use std::fs;
    use tianting_fu::{
        中止任务线, 回填任务线结果, 读任务线们, 读世界状态, 推进想法状态, 确保世界状态初始化,
        登记任务线, 领取待执行任务线, 落盘队列, 写世界状态, 想法, 想法状态, 任务线状态,
    };

    /// 任务线全生命周期：登记(待执行) → 领取(执行中) → 回填(已完成)。
    #[test]
    fn 任务线全生命周期机械链() {
        let (根, _锁) = 临时工作区("离线主链路", "任务线链");
        let 想法 = 想法 {
            id: "想法-1".to_string(),
            内容: "给 流式时刻园 补闰年测试，涉及路径：乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-时刻-园/流式时刻.rs".to_string(),
            时间: 1,
            状态: 想法状态::未处理,
        };
        let 任务线 = 登记任务线(&想法).unwrap();
        assert_eq!(任务线.状态, 任务线状态::待执行);

        let 领取 = 领取待执行任务线().unwrap().expect("应领取到一条");
        assert_eq!(领取.id, 任务线.id);
        let 状态们 = 读任务线们().unwrap();
        assert_eq!(状态们[0].状态, 任务线状态::执行中, "领取后应置执行中");

        // 未领取前再次领取应无（已被领取）。
        assert!(领取待执行任务线().unwrap().is_none(), "单条任务线不应被双跑");

        回填任务线结果(&任务线.id, "要求-1", "通过", "测试汇报").unwrap();
        let 状态们 = 读任务线们().unwrap();
        assert_eq!(状态们[0].状态, 任务线状态::已完成);
        assert_eq!(状态们[0].结论.as_deref(), Some("通过"));
        fs::create_dir_all(根.join(".上下文").join("状态")).unwrap();
        let _ = fs::remove_dir_all(&根);
    }

    /// 中止任务线：待执行 → 已中止，领取不到。
    #[test]
    fn 中止任务线后不再被领取() {
        let (根, _锁) = 临时工作区("离线主链路", "中止");
        let 想法 = 想法 {
            id: "想法-2".to_string(),
            内容: "新增 世界 昼夜 命令，涉及路径：乾坤/呈现-域/命令操作-府".to_string(),
            时间: 1,
            状态: 想法状态::未处理,
        };
        let 任务线 = 登记任务线(&想法).unwrap();
        中止任务线(&任务线.id).unwrap();
        assert!(领取待执行任务线().unwrap().is_none(), "已中止任务线不得被领取");
        let _ = fs::remove_dir_all(&根);
    }

    /// 状态机机械链：想法入池 → 推进状态 → 读世界状态（读时聚合 生产化 2.1）。
    #[test]
    fn 状态机与读时聚合() {
        let (根, _锁) = 临时工作区("离线主链路", "状态机");
        fs::create_dir_all(根.join(".上下文").join("状态")).unwrap();
        let 状态 = 确保世界状态初始化(&根.join(".上下文").join("状态")).unwrap();
        写世界状态(&根.join(".上下文").join("状态"), &状态).unwrap();

        let 想法 = 想法 {
            id: "想法-3".to_string(),
            内容: "审验 各园测试覆盖，涉及路径：无（审验类）".to_string(),
            时间: 1,
            状态: 想法状态::未处理,
        };
        let 想法池 = 落盘队列::<想法>::打开(根.join(".上下文").join("状态").join("想法.jsonl"));
        想法池.入队(&想法).unwrap();
        推进想法状态(&想法.id, 想法状态::已化为要求).unwrap();

        // 读时聚合：世界状态内嵌 想法池 应从 想法.jsonl 聚合出 1 条，且状态已推进。
        let 读回 = 读世界状态(&根.join(".上下文").join("状态")).unwrap().expect("应有状态");
        assert_eq!(读回.界主想法池.len(), 1);
        assert_eq!(读回.界主想法池[0].状态, 想法状态::已化为要求, "读时聚合应看到最新状态");
        let _ = fs::remove_dir_all(&根);
    }

    /// 并发压力（生产化 4.4）：多线程同时领取，单条任务线只被领到一次（锁文件互斥，无双跑）。
    #[test]
    fn 并发领取无双跑() {
        let (根, _锁) = 临时工作区("离线主链路", "并发");
        for 序 in 0..3 {
            let 想法 = 想法 {
                id: format!("想法-并发-{序}"),
                内容: format!("并发任务 {序}，涉及路径：乾坤/呈现-域/命令操作-府/观览-查询-殿"),
                时间: 1,
                状态: 想法状态::未处理,
            };
            登记任务线(&想法).unwrap();
        }
        let 成功数 = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let 线程们: Vec<_> = (0..4)
            .map(|_| {
                let 成功数 = std::sync::Arc::clone(&成功数);
                std::thread::spawn(move || {
                    for _ in 0..60 {
                        if 领取待执行任务线().unwrap().is_some() {
                            成功数.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                })
            })
            .collect();
        for 线程 in 线程们 {
            线程.join().unwrap();
        }
        assert_eq!(成功数.load(std::sync::atomic::Ordering::Relaxed), 3, "3 条任务线应恰好被领 3 次（无双跑）");
        let 状态们 = 读任务线们().unwrap();
        assert_eq!(状态们.iter().filter(|线| 线.状态 == 任务线状态::执行中).count(), 3, "全部进入执行中");
        let _ = fs::remove_dir_all(&根);
    }

    /// 并发登记与回填不丢失（2026-08-17 轮8 体检：复合读改写与入队并发会互相覆盖——
    /// 回填重写时登记 append 的新行被覆盖丢失）。排他锁统一后：并发后行数不减、无损坏行。
    #[test]
    fn 并发登记与回填不丢失() {
        let (根, _锁) = 临时工作区("离线主链路", "并发写");
        // 线程 A：登记 30 条；线程 B：并发领取并回填（处理到无可领为止）。
        let 甲 = std::thread::spawn(move || {
            for 序 in 0..30 {
                let 想法 = 想法 {
                    id: format!("想法-并发写-{序}"),
                    内容: format!("并发任务 {序}，涉及路径：乾坤/呈现-域/命令操作-府/观览-查询-殿"),
                    时间: 1,
                    状态: 想法状态::未处理,
                };
                登记任务线(&想法).unwrap();
            }
        });
        let 乙 = std::thread::spawn(move || {
            for _ in 0..200 {
                if let Some(线) = 领取待执行任务线().unwrap() {
                    回填任务线结果(&线.id, "要求-并发", "通过", "并发测试").unwrap();
                }
            }
        });
        甲.join().unwrap();
        乙.join().unwrap();
        // 并发后：30 条登记全部保留（无覆盖丢失），且每条可解析（无行交错损坏）。
        let 内容 = fs::read_to_string(根.join(".上下文").join("状态").join("任务线.jsonl")).unwrap();
        let 行们: Vec<&str> = 内容.lines().filter(|行| !行.trim().is_empty()).collect();
        assert_eq!(行们.len(), 30, "30 条登记应全部保留（无覆盖丢失），实际 {}", 行们.len());
        for 行 in 行们 {
            let 线: tianting_fu::任务线 = serde_json::from_str(行).unwrap_or_else(|错误| panic!("损坏行：{错误}：{行}"));
            assert!(
                线.状态 == 任务线状态::待执行 || 线.状态 == 任务线状态::已完成,
                "状态应为待执行或已完成：{:?}",
                线.状态
            );
        }
        let _ = fs::remove_dir_all(&根);
    }
}
