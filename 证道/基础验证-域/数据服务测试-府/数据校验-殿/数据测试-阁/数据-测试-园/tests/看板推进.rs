use super::*;

    /// 构造完整装配状态（五行相生桥接已串联），用于看板驱动五行闭环测试
    fn 构造装配状态() -> 数据服务状态 {
        let 装配 = hm_linkage::五行装配::装配();
        let 序号 = 看板序号.fetch_add(1, Ordering::SeqCst);
        let 任务看板 = Arc::new(Mutex::new(tc_task::TaskBoard::新建(
            std::env::temp_dir().join(format!("洪荒测试看板_装配_{序号}.jsonl")).to_string_lossy().to_string(),
        )));
        {
            let mut 看板 = 任务看板.lock().expect("看板锁中毒");
            看板.设置信号总线(装配.信号总线.clone());
        }
        数据服务状态::新(
            装配.任务仓库.clone(),
            装配.迭代日志.clone(),
            装配.记忆库.clone(),
            装配.规则库.clone(),
            装配.事件总线.clone(),
            装配.图谱.clone(),
            装配.心智地图.clone(),
            装配.语境.clone(),
            任务看板,
            装配.日志记录器.clone(),
            Arc::new(开发执行台::新()),
            Arc::new(hm_http::看板驱动台::新()),
            None,
            None,
        )
    }

    #[tokio::test]
    async fn 看板_发布任务触发任务推进信号() {
        let 状态 = 构造装配状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "信号测试".into(),
            description: "验证发布信号".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
            panic!("查询应成功");
        };
        assert_eq!(任务.title, "信号测试");
    }

    #[tokio::test]
    async fn 看板_任务完成触发木生火开启迭代() {
        let 状态 = 构造装配状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "闭环测试".into(),
            description: "验证木生火".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "圣人".into(),
        })).await.expect("承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "圣人".into(),
            next_status: "待大罗金仙实现".into(),
        })).await.expect("提交应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "大罗金仙".into(),
        })).await.expect("大罗金仙承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "大罗金仙".into(),
            next_status: "待准圣验收".into(),
        })).await.expect("提交应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "准圣".into(),
        })).await.expect("准圣承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "准圣".into(),
            next_status: "待道祖终审".into(),
        })).await.expect("提交应成功");

        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "道祖".into(),
        })).await.expect("道祖承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "道祖".into(),
            next_status: "待清理".into(),
        })).await.expect("道祖提交应成功");

        // 六层流转：终审通过后由太乙金仙清理收尾，清理完成触发 木生火
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "太乙金仙".into(),
        })).await.expect("太乙金仙承接应成功");

        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "太乙金仙".into(),
            next_status: "清理完成".into(),
        })).await.expect("太乙金仙提交应成功");

        let 迭代日志 = 状态.迭代日志.lock().expect("迭代日志锁中毒");
        let 迭代列表 = 迭代日志.全部();
        assert!(!迭代列表.is_empty(), "木生火应已开启迭代");
        assert!(迭代列表.iter().any(|it| it.变更说明.contains("闭环测试")),
            "迭代变更说明应包含任务标题");
    }

    #[tokio::test]
    async fn 看板_清理一键接口完成待清理任务() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "清理测试".into(),
            description: "验证一键清理".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        // 推进到待清理状态
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "圣人".into(),
        })).await.expect("圣人承接");
        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "圣人".into(),
            next_status: "待大罗金仙实现".into(),
        })).await.expect("圣人提交");
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "大罗金仙".into(),
        })).await.expect("大罗金仙承接");
        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "大罗金仙".into(),
            next_status: "待准圣验收".into(),
        })).await.expect("大罗金仙提交");
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "准圣".into(),
        })).await.expect("准圣承接");
        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "准圣".into(),
            next_status: "待道祖终审".into(),
        })).await.expect("准圣提交");
        看板承接(State(状态.clone()), Path(id), Json(承接任务请求 {
            role: "道祖".into(),
        })).await.expect("道祖承接");
        看板提交(State(状态.clone()), Path(id), Json(提交任务请求 {
            role: "道祖".into(),
            next_status: "待清理".into(),
        })).await.expect("道祖提交到待清理");

        // 一键清理
        let _ = 看板清理(State(状态.clone()), Path(id)).await.expect("清理应成功");

        let Ok(Json(任务)) = 看板查询(State(状态), Path(id)).await else {
            panic!("查询应成功");
        };
        assert_eq!(任务.status, TaskStatus::清理完成);
        assert_eq!(任务.当前承接人, tc_task::AgentRole::太乙金仙);
    }

    #[tokio::test]
    async fn 看板_清理接口对非待清理状态返回错误() {
        let 状态 = 构造状态();
        let Json(id) = 看板发布(State(状态.clone()), Json(发布任务请求 {
            title: "不可清理".into(),
            description: "验证错误清理".into(),
            scene: None,
            priority: None,
        })).await.expect("发布应成功");

        let 结果 = 看板清理(State(状态), Path(id)).await;
        assert!(结果.is_err(), "对待圣人设计状态的任务清理应失败");
    }
