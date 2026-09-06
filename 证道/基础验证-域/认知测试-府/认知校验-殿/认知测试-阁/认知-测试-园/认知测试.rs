#[cfg(test)]
mod tests {
    use hm_cognition::{
        AgentRole, ContextManager, ToolCallRecord,
        依赖边, 图谱, 符号, 符号种类, 模块, 维度, 格位, 心智地图, 消息角色, 语境消息, 过程上下文,
        维度载荷, 目标载荷, 执行载荷, 规则载荷, 严重度, 外在载荷, 内部载荷, 经历载荷, 空载荷,
        默认三十六格位, 检索器, 检索源, 注入器, 压缩器, 上下文库, 上下文消息, 纠错引擎, 纠错事件,
        规则提炼器, 候选摘要, 三态存储,
        上下文压缩阈值, 压缩滑动窗口, 晋升重复阈值,
    };

    fn 造格位(维度: 维度, 格位名: &str, 摘要: &str) -> 格位 {
        格位 {
            维度,
            格位名: 格位名.into(),
            摘要: 摘要.into(),
            可信度: 0.9,
            证据引用: vec![],
            最后校验时间: 0,
            冷却期至: None,
            维度载荷: None,
        }
    }

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("hm_cognition_test_{名}.json"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn 图谱_查依赖返回被依赖模块() {
        let mut 图 = 图谱::新();
        图.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙".into() });
        图.添加模块(模块 { 名称: "hm-error".into(), 路径: "鸿蒙".into() });
        图.添加依赖(依赖边 { 源: "hm-linkage".into(), 目标: "hm-error".into() });

        assert_eq!(图.查依赖("hm-linkage"), vec!["hm-error".to_string()]);
        assert_eq!(图.被谁依赖("hm-error"), vec!["hm-linkage".to_string()]);
    }

    #[test]
    fn 图谱_查符号返回签名与所属模块() {
        let mut 图 = 图谱::新();
        图.添加符号(符号 {
            名称: "运行".into(),
            种类: 符号种类::函数,
            所属模块: "hm-agent".into(),
            签名: Some("fn(任务) -> 结果".into()),
        });

        let 符号 = 图.查符号("运行").expect("符号应存在");
        assert_eq!(符号.种类, 符号种类::函数);
        assert_eq!(符号.所属模块, "hm-agent");
        assert_eq!(符号.签名.as_deref(), Some("fn(任务) -> 结果"));
    }

    #[test]
    fn 图谱_序列化往返保持一致() {
        let mut 图 = 图谱::新();
        图.添加模块(模块 { 名称: "hm-agent".into(), 路径: "鸿蒙".into() });
        图.添加依赖(依赖边 { 源: "hm-agent".into(), 目标: "hm-execute".into() });

        let 文本 = toml::to_string(&图).expect("序列化应成功");
        let 还原: 图谱 = toml::from_str(&文本).expect("反序列化应成功");
        assert_eq!(还原.模块集.len(), 1);
        assert_eq!(还原.模块集[0].名称, "hm-agent");
        assert_eq!(还原.依赖集[0].目标, "hm-execute");
    }

    #[test]
    fn 心智地图_添加格位后可查询摘要() {
        let mut 地图 = 心智地图::空();
        地图.添加格位(造格位(维度::内部, "角色", "我是谁")).expect("添加应成功");

        let 格位 = 地图.查询格位(维度::内部, "角色").expect("格位应存在");
        assert_eq!(格位.摘要, "我是谁");
        assert_eq!(格位.格位名, "角色");
        assert_eq!(格位.可信度, 0.9);
    }

    #[test]
    fn 心智地图_同维度同名格位被拒绝() {
        let mut 地图 = 心智地图::空();
        地图.添加格位(造格位(维度::规则, "门禁", "一")).expect("首次添加应成功");
        let 结果 = 地图.添加格位(造格位(维度::规则, "门禁", "二"));
        assert!(结果.is_err());
        assert!(结果.expect_err("应报错").to_string().contains("格位已存在"));

        assert!(地图.添加格位(造格位(维度::执行, "门禁", "三")).is_ok());
    }

    #[test]
    fn 心智地图_更新格位单点修正不影响其他格位() {
        let mut 地图 = 心智地图::空();
        地图.添加格位(造格位(维度::目标, "当前", "旧摘要")).expect("添加应成功");
        地图.添加格位(造格位(维度::目标, "未来", "保持不动")).expect("添加应成功");

        地图.更新格位(造格位(维度::目标, "当前", "新摘要")).expect("更新应成功");
        assert_eq!(地图.查询格位(维度::目标, "当前").expect("存在").摘要, "新摘要");
        assert_eq!(地图.查询格位(维度::目标, "未来").expect("存在").摘要, "保持不动");
    }

    #[test]
    fn 心智地图_更新不存在格位报错() {
        let mut 地图 = 心智地图::空();
        let 结果 = 地图.更新格位(造格位(维度::经历, "不存在", "x"));
        assert!(结果.is_err());
        assert!(结果.expect_err("应报错").to_string().contains("格位不存在"));
    }

    #[test]
    fn 心智地图_序列化往返保持一致() {
        let mut 地图 = 心智地图::空();
        地图.添加格位(造格位(维度::内部, "角色", "我是谁")).expect("添加应成功");

        let 文本 = toml::to_string(&地图).expect("序列化应成功");
        let 还原: 心智地图 = toml::from_str(&文本).expect("反序列化应成功");
        assert_eq!(还原.格位集.len(), 1);
        assert_eq!(还原.格位集[0].维度, 维度::内部);
        assert_eq!(还原.格位集[0].格位名, "角色");
        assert_eq!(还原.格位集[0].摘要, "我是谁");
    }

    #[test]
    fn 维度_内外判定正确() {
        assert!(维度::内部.是否内向());
        assert!(维度::执行.是否内向());
        assert!(维度::目标.是否内向());
        assert!(维度::经历.是否内向());
        assert!(!维度::外在.是否内向());
        assert!(!维度::规则.是否内向());
    }

    #[test]
    fn 维度_正交对正确() {
        assert_eq!(维度::内部.正交对(), (维度::内部, 维度::外在));
        assert_eq!(维度::外在.正交对(), (维度::外在, 维度::内部));
        assert_eq!(维度::规则.正交对(), (维度::规则, 维度::执行));
        assert_eq!(维度::执行.正交对(), (维度::执行, 维度::规则));
        assert_eq!(维度::目标.正交对(), (维度::目标, 维度::经历));
        assert_eq!(维度::经历.正交对(), (维度::经历, 维度::目标));
    }

    #[test]
    fn 心智地图_目标维度格位持有目标载荷() {
        let mut 地图 = 心智地图::新();
        let mut 格位 = 造格位(维度::目标, "当前", "方向摘要");
        格位.维度载荷 = Some(维度载荷::目标(目标载荷 {
            初心: "让 AI 懂项目".into(),
            现况: "正在搭骨架".into(),
            愿景: "能自主开发".into(),
            偏移: None,
            度量: "三态落地".into(),
            依赖: vec!["图谱".into()],
        }));
        地图.添加格位(格位).expect("添加应成功");

        let 格位 = 地图.查询格位(维度::目标, "当前").expect("格位应存在");
        let 载荷 = 格位.维度载荷.as_ref().expect("目标维度应有目标载荷");
        match 载荷 {
            维度载荷::目标(内容) => {
                assert_eq!(内容.初心, "让 AI 懂项目");
                assert_eq!(内容.现况, "正在搭骨架");
                assert_eq!(内容.愿景, "能自主开发");
                assert_eq!(内容.度量, "三态落地");
            }
            _ => panic!("目标维度应承载目标载荷"),
        }
    }

    #[test]
    fn 心智地图_非目标维度格位无载荷() {
        let mut 地图 = 心智地图::空();
        地图.添加格位(造格位(维度::内部, "角色", "我是谁")).expect("添加应成功");

        let 格位 = 地图.查询格位(维度::内部, "角色").expect("格位应存在");
        assert!(格位.维度载荷.is_none(), "非目标维度格位不应持有载荷");
    }

    #[test]
    fn 过程上下文_追加消息后内容正确() {
        let mut 上下文 = 过程上下文::新();
        上下文.追加(语境消息 { 角色: 消息角色::用户, 内容: "修复bug".into() });
        上下文.追加(语境消息 { 角色: 消息角色::助手, 内容: "已完成".into() });

        assert_eq!(上下文.消息数(), 2);
        assert_eq!(上下文.全部()[0].角色, 消息角色::用户);
        assert_eq!(上下文.全部()[1].内容, "已完成");
    }

    #[test]
    fn 过程上下文_序列化往返保持一致() {
        let mut 上下文 = 过程上下文::新();
        上下文.追加(语境消息 { 角色: 消息角色::系统, 内容: "系统提示".into() });

        let 文本 = toml::to_string(&上下文).expect("序列化应成功");
        let 还原: 过程上下文 = toml::from_str(&文本).expect("反序列化应成功");
        assert_eq!(还原.消息数(), 1);
        assert_eq!(还原.全部()[0].角色, 消息角色::系统);
        assert_eq!(还原.全部()[0].内容, "系统提示");
    }

    #[test]
    fn 角色_默认为道祖() {
        let role = AgentRole::default();
        assert_eq!(role, AgentRole::道祖);
    }

    #[test]
    fn 角色_名称返回正确中文() {
        assert_eq!(AgentRole::道祖.名称(), "道祖");
        assert_eq!(AgentRole::圣人.名称(), "圣人");
        assert_eq!(AgentRole::大罗金仙.名称(), "大罗金仙");
        assert_eq!(AgentRole::准圣.名称(), "准圣");
    }

    #[test]
    fn 上下文管理器_创建后可查询() {
        let path = 临时路径("创建查询");
        let mut mgr = ContextManager::新(&path);
        let id = mgr.创建上下文(AgentRole::圣人, Some(42));

        let ctx = mgr.查询(&id).expect("上下文应存在");
        assert_eq!(ctx.层级角色, AgentRole::圣人);
        assert_eq!(ctx.task_id, Some(42));
        assert!(ctx.消息.is_empty());
        assert!(ctx.工具调用.is_empty());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_添加消息后内容正确() {
        let path = 临时路径("添加消息");
        let mut mgr = ContextManager::新(&path);
        let id = mgr.创建上下文(AgentRole::道祖, None);

        mgr.添加消息(&id, 语境消息 { 角色: 消息角色::用户, 内容: "设计需求".into() })
            .expect("添加消息应成功");
        mgr.添加消息(&id, 语境消息 { 角色: 消息角色::助手, 内容: "已理解".into() })
            .expect("添加消息应成功");

        let ctx = mgr.查询(&id).expect("上下文应存在");
        assert_eq!(ctx.消息.len(), 2);
        assert_eq!(ctx.消息[0].内容, "设计需求");
        assert_eq!(ctx.消息[1].角色, 消息角色::助手);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_记录工具调用后内容正确() {
        let path = 临时路径("工具调用");
        let mut mgr = ContextManager::新(&path);
        let id = mgr.创建上下文(AgentRole::大罗金仙, Some(7));

        mgr.记录工具调用(&id, ToolCallRecord {
            工具名: "read_file".into(),
            参数: "src/main.rs".into(),
            结果摘要: "读取成功".into(),
            时间戳: 100,
        }).expect("记录工具调用应成功");

        let ctx = mgr.查询(&id).expect("上下文应存在");
        assert_eq!(ctx.工具调用.len(), 1);
        assert_eq!(ctx.工具调用[0].工具名, "read_file");
        assert_eq!(ctx.工具调用[0].参数, "src/main.rs");
        assert_eq!(ctx.工具调用[0].结果摘要, "读取成功");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_清理后不可查询() {
        let path = 临时路径("清理");
        let mut mgr = ContextManager::新(&path);
        let id = mgr.创建上下文(AgentRole::准圣, None);

        mgr.清理上下文(&id).expect("清理应成功");
        assert!(mgr.查询(&id).is_none(), "清理后查询应返回 None");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_不存在上下文添加消息返回错误() {
        let path = 临时路径("不存在");
        let mut mgr = ContextManager::新(&path);
        let result = mgr.添加消息("ctx-nonexistent", 语境消息 {
            角色: 消息角色::用户,
            内容: "测试".into(),
        });
        assert!(result.is_err());
        let err = result.expect_err("应报错");
        assert!(err.to_string().contains("上下文不存在"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_不存在上下文记录工具调用返回错误() {
        let path = 临时路径("不存在工具");
        let mut mgr = ContextManager::新(&path);
        let result = mgr.记录工具调用("ctx-nonexistent", ToolCallRecord {
            工具名: "test".into(),
            参数: "".into(),
            结果摘要: "".into(),
            时间戳: 0,
        });
        assert!(result.is_err());
        assert!(result.expect_err("应报错").to_string().contains("上下文不存在"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_不存在上下文清理返回错误() {
        let path = 临时路径("不存在清理");
        let mut mgr = ContextManager::新(&path);
        let result = mgr.清理上下文("ctx-nonexistent");
        assert!(result.is_err());
        assert!(result.expect_err("应报错").to_string().contains("上下文不存在"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_不同角色上下文相互隔离() {
        let path = 临时路径("隔离");
        let mut mgr = ContextManager::新(&path);
        let id_sage = mgr.创建上下文(AgentRole::圣人, Some(1));
        let id_dev = mgr.创建上下文(AgentRole::大罗金仙, Some(1));

        mgr.添加消息(&id_sage, 语境消息 { 角色: 消息角色::助手, 内容: "圣人思考".into() })
            .expect("圣人添加消息");
        mgr.添加消息(&id_dev, 语境消息 { 角色: 消息角色::助手, 内容: "金仙实现".into() })
            .expect("金仙添加消息");

        let ctx_sage = mgr.查询(&id_sage).expect("圣人上下文应存在");
        let ctx_dev = mgr.查询(&id_dev).expect("金仙上下文应存在");

        assert_eq!(ctx_sage.层级角色, AgentRole::圣人);
        assert_eq!(ctx_sage.消息[0].内容, "圣人思考");
        assert_eq!(ctx_dev.层级角色, AgentRole::大罗金仙);
        assert_eq!(ctx_dev.消息[0].内容, "金仙实现");

        assert_eq!(ctx_sage.消息.len(), 1, "圣人上下文不应包含金仙消息");
        assert_eq!(ctx_dev.消息.len(), 1, "金仙上下文不应包含圣人消息");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_json持久化往返() {
        let path = 临时路径("持久化");
        let mut mgr = ContextManager::新(&path);
        let id = mgr.创建上下文(AgentRole::圣人, Some(10));
        mgr.添加消息(&id, 语境消息 { 角色: 消息角色::用户, 内容: "持久化测试".into() })
            .expect("添加消息");
        mgr.记录工具调用(&id, ToolCallRecord {
            工具名: "write_file".into(),
            参数: "output.rs".into(),
            结果摘要: "写入成功".into(),
            时间戳: 200,
        }).expect("记录工具调用");
        mgr.保存().expect("保存应成功");

        let loaded = ContextManager::加载(&path).expect("加载应成功");
        let ctx = loaded.查询(&id).expect("上下文应存在");
        assert_eq!(ctx.层级角色, AgentRole::圣人);
        assert_eq!(ctx.task_id, Some(10));
        assert_eq!(ctx.消息.len(), 1);
        assert_eq!(ctx.消息[0].内容, "持久化测试");
        assert_eq!(ctx.工具调用.len(), 1);
        assert_eq!(ctx.工具调用[0].工具名, "write_file");
        assert_eq!(ctx.工具调用[0].结果摘要, "写入成功");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn 上下文管理器_加载不存在的文件返回空管理器() {
        let path = 临时路径("不存在加载");
        let mgr = ContextManager::加载(&path).expect("加载应成功");
        assert_eq!(mgr.全部().len(), 0);
    }

    #[test]
    fn 上下文管理器_全部列出所有上下文() {
        let path = 临时路径("全部");
        let mut mgr = ContextManager::新(&path);
        mgr.创建上下文(AgentRole::道祖, None);
        mgr.创建上下文(AgentRole::圣人, None);
        mgr.创建上下文(AgentRole::大罗金仙, None);

        let 全部 = mgr.全部();
        assert_eq!(全部.len(), 3);
        let 角色 = 全部.iter().map(|c| c.层级角色).collect::<Vec<_>>();
        assert!(角色.contains(&AgentRole::道祖));
        assert!(角色.contains(&AgentRole::圣人));
        assert!(角色.contains(&AgentRole::大罗金仙));
        std::fs::remove_file(&path).ok();
    }

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
    fn 路由_关键词命中() {
        let mut 地图 = 心智地图::默认三十六格位();
        地图.写(维度::执行, "命令", "cargo build --workspace", 0.9, vec!["cargo".into()])
            .expect("写入应成功");
        地图.写(维度::外在, "依赖", "太初依赖量劫", 0.8, vec!["依赖".into()])
            .expect("写入应成功");
        let 候选 = 地图.路由("构建命令是什么", 6);
        assert!(候选.iter().any(|(维度, 名)| *维度 == 维度::执行 && 名 == "命令"));
        let 候选2 = 地图.路由("项目依赖关系", 6);
        assert!(候选2.iter().any(|(维度, 名)| *维度 == 维度::外在 && 名 == "依赖"));
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
    fn 检索链_格位命中() {
        let 图 = 图谱::新();
        let mut 地图 = 心智地图::默认三十六格位();
        地图.写(维度::外在, "依赖", "太初依赖量劫", 0.9, vec!["边".into()])
            .expect("写入应成功");
        let 库 = 上下文库::新();
        let mut 检索器 = 检索器::新(&图, &mut 地图, &库);
        let 决策 = 检索器.检索("项目依赖关系");
        assert_eq!(决策.最终来源, 检索源::格位);
        assert!(决策.答复.contains("太初依赖量劫"));
    }

    #[test]
    fn 检索链_下沉临时() {
        let 图 = 图谱::新();
        let mut 地图 = 心智地图::默认三十六格位();
        let mut 库 = 上下文库::新();
        库.追加(消息角色::助手, "刚才执行了序列化修复");
        let mut 检索器 = 检索器::新(&图, &mut 地图, &库);
        let 决策 = 检索器.检索("序列化");
        assert_eq!(决策.最终来源, 检索源::临时);
        assert!(决策.答复.contains("临时上下文"));
        assert!(决策.下沉路径.contains(&检索源::格位));
        assert!(决策.下沉路径.contains(&检索源::临时));
    }

    #[test]
    fn 检索链_下沉图谱校准() {
        let mut 图 = 图谱::新();
        图.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动".into() });
        let mut 地图 = 心智地图::默认三十六格位();
        地图.写(维度::外在, "定位", "hm-linkage 是联动模块", 0.6, vec!["旧".into()])
            .expect("写入应成功");
        let 库 = 上下文库::新();
        let mut 检索器 = 检索器::新(&图, &mut 地图, &库);
        let 决策 = 检索器.检索("linkage");
        assert_eq!(决策.最终来源, 检索源::图谱);
        let 格位 = 地图.查询格位(维度::外在, "定位").expect("格位应存在");
        assert!(格位.可信度 < 0.6, "图谱命中应下调候选格位置信度");
    }

    #[test]
    fn 检索链_兜底() {
        let 图 = 图谱::新();
        let mut 地图 = 心智地图::默认三十六格位();
        let 库 = 上下文库::新();
        let mut 检索器 = 检索器::新(&图, &mut 地图, &库);
        let 决策 = 检索器.检索("完全不存在的东西");
        assert_eq!(决策.最终来源, 检索源::空);
        assert!(决策.答复.contains("无答案"));
    }

    #[test]
    fn 检索链_空问题() {
        let 图 = 图谱::新();
        let mut 地图 = 心智地图::默认三十六格位();
        let 库 = 上下文库::新();
        let mut 检索器 = 检索器::新(&图, &mut 地图, &库);
        let 决策 = 检索器.检索("   ");
        assert_eq!(决策.最终来源, 检索源::空);
    }
    // ==================== 三态存储（v1.42 持久化） ====================

    fn 三态临时目录(名: &str) -> String {
        let 目录 = std::env::temp_dir().join("zd-cognition-三态").join(名);
        let _ = std::fs::remove_dir_all(&目录);
        目录.to_string_lossy().into_owned()
    }

    fn 造三态() -> (图谱, 心智地图, 上下文库) {
        let mut 图谱 = 图谱::新();
        图谱.添加模块(模块 { 名称: "hm-linkage".into(), 路径: "鸿蒙/联动装配".into() });
        图谱.添加符号(符号 { 名称: "运行".into(), 种类: 符号种类::函数, 所属模块: "hm-agent".into(), 签名: Some("fn(任务)".into()) });
        图谱.添加技术栈("serde");
        let mut 心智 = 心智地图::新();
        心智.写(维度::目标, "现况", "目标现况摘要", 0.85, vec!["证据A".into()]).expect("写格位");
        心智.写载荷(维度::目标, "初心", "初心摘要", 0.9, 维度载荷::目标(目标载荷 {
            初心: "自主演化".into(),
            现况: "三态就绪".into(),
            愿景: "无人干预".into(),
            偏移: Some("偏慢".into()),
            度量: "测试数".into(),
            依赖: vec!["Rust".into()],
        }), vec![]).expect("写载荷格位");
        let mut 上下文 = 上下文库::新();
        上下文.追加(消息角色::用户, "第一条");
        上下文.追加(消息角色::助手, "第二条");
        (图谱, 心智, 上下文)
    }

    #[test]
    fn 三态存储_保存加载往返() {
        let (图谱, 心智, 上下文) = 造三态();
        let 存储 = 三态存储::新(三态临时目录("往返")).expect("创建存储");
        存储.保存(&图谱, &心智, &上下文).expect("保存三态");

        let (图谱2, 心智2, 上下文2) = 存储.加载().expect("加载三态");
        assert_eq!(图谱2.模块集[0].名称, "hm-linkage");
        assert_eq!(图谱2.符号集[0].签名.as_deref(), Some("fn(任务)"));
        assert_eq!(图谱2.技术栈, vec!["serde".to_string()]);
        let 格位 = 心智2.查询格位(维度::目标, "现况").expect("格位应存在");
        assert_eq!(格位.摘要, "目标现况摘要");
        assert!((格位.可信度 - 0.85).abs() < 1e-6);
        assert_eq!(上下文2.长度(), 2);
        assert_eq!(上下文2.最近(2)[0].内容, "第一条");
        assert_eq!(上下文2.最近(2)[1].内容, "第二条");
    }

    #[test]
    fn 三态存储_空目录加载报错() {
        let 存储 = 三态存储::新(三态临时目录("空目录")).expect("创建存储");
        assert!(存储.加载().is_err(), "空目录加载应报错");
        assert!(!存储.已存在());
    }

    #[test]
    fn 三态存储_上下文库超限截断() {
        let 目录 = 三态临时目录("截断");
        let 存储 = 三态存储::新(&目录).expect("创建存储");
        let 消息流: Vec<上下文消息> = (1..=1005)
            .map(|id| 上下文消息 { id, 角色: 消息角色::用户, 内容: format!("消息{id}"), 时间戳: id })
            .collect();
        let 上下文 = 上下文库::导入(消息流, 1000);
        存储.保存上下文(&上下文).expect("保存上下文");

        let 加载 = 存储.加载上下文().expect("加载上下文");
        assert_eq!(加载.长度(), 1000, "超限应截断到硬上限");
        assert_eq!(加载.最近(1)[0].内容, "消息1005", "最新消息保留");
        assert_eq!(加载.全部()[0].内容, "消息6", "最旧 5 条被丢弃");
    }

    #[test]
    fn 三态存储_心智地图格位载荷保留() {
        let (图谱, 心智, 上下文) = 造三态();
        let 存储 = 三态存储::新(三态临时目录("载荷")).expect("创建存储");
        存储.保存心智(&心智).expect("保存心智");

        let 加载 = 存储.加载心智().expect("加载心智");
        let 格位 = 加载.查询格位(维度::目标, "初心").expect("格位应存在");
        match &格位.维度载荷 {
            Some(维度载荷::目标(载荷)) => {
                assert_eq!(载荷.初心, "自主演化");
                assert_eq!(载荷.愿景, "无人干预");
                assert_eq!(载荷.依赖, vec!["Rust".to_string()]);
            }
            _ => panic!("目标格位应持有目标载荷"),
        }
    }

    #[test]
    fn 三态存储_纠错事件追加与加载() {
        let 存储 = 三态存储::新(三态临时目录("纠错")).expect("创建存储");
        let 事件1 = 纠错事件 { 时间: 1, 检测到的差异: "摘要过时".into(), 受影响格位: vec!["目标·现况".into()], 新摘要: "新摘要".into(), 新可信度: 0.6, 教训触发: None };
        let 事件2 = 纠错事件 { 时间: 2, 检测到的差异: "规则冲突".into(), 受影响格位: vec!["规则·红线".into()], 新摘要: "规则修正".into(), 新可信度: 0.5, 教训触发: Some("同类纠错≥2，应提炼教训".into()) };
        存储.追加纠错事件(&事件1).expect("追加");
        存储.追加纠错事件(&事件2).expect("追加");

        let 事件集 = 存储.加载纠错事件().expect("加载");
        assert_eq!(事件集.len(), 2);
        assert_eq!(事件集[0].新摘要, "新摘要");
        assert_eq!(事件集[1].新可信度, 0.5);
        assert_eq!(事件集[1].教训触发.as_deref(), Some("同类纠错≥2，应提炼教训"));
    }

    #[test]
    fn 三态存储_损坏文件加载报错() {
        let 目录 = 三态临时目录("损坏");
        let 存储 = 三态存储::新(&目录).expect("创建存储");
        std::fs::write(存储.目录().join("心智地图.json"), "这不是合法JSON{").expect("写入坏文件");

        assert!(存储.加载心智().is_err(), "损坏心智文件应报错");
        assert!(存储.已存在(), "存在文件应判定为已存在");
    }

    #[test]
    fn 三态存储_上下文库坏行容忍() {
        let 目录 = 三态临时目录("坏行");
        let 存储 = 三态存储::新(&目录).expect("创建存储");
        let 好消息 = 上下文消息 { id: 2, 角色: 消息角色::助手, 内容: "完好".into(), 时间戳: 2 };
        let 内容 = format!("坏行不是JSON\n{}\n", serde_json::to_string(&好消息).expect("序列化"));
        std::fs::write(存储.目录().join("上下文库.jsonl"), 内容).expect("写坏行文件");

        let 加载 = 存储.加载上下文().expect("坏行应被跳过");
        assert_eq!(加载.长度(), 1, "坏行跳过，好行保留");
        assert_eq!(加载.最近(1)[0].内容, "完好");
    }

}