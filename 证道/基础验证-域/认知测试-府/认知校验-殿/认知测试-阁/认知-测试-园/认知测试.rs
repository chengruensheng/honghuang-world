#[cfg(test)]
mod tests {
    use hm_cognition::{
        AgentRole, ContextManager, ToolCallRecord,
        依赖边, 图谱, 符号, 符号种类, 模块, 维度, 格位, 心智地图, 消息角色, 语境消息, 过程上下文,
        维度载荷, 目标载荷,
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
}
