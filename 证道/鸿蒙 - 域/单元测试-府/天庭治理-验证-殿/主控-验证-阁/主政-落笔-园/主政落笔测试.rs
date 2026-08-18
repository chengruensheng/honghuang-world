//! 主政 - 落笔 - 园 · 主政落笔测试：确认设计与验收裁决。

#[cfg(test)]
mod 测试 {
    use crate::道术施展_验证_殿::手脚_验证_阁::隔离_互斥_园::隔离设施::设施::环境变量锁;
    use tianting_fu::{产物条目, 拆解项, 设计方案, 确认设计, 验收结论, 验收裁决};

    #[test]
    fn 确认设计拒空拆解() {
        let 方案 = 设计方案 {
            要求id: "r1".to_string(),
            设计: "".to_string(),
            拆解: vec![],
            自评: "自评".to_string(),
        };
        assert_eq!(确认设计(&方案), 验收结论::打回);
    }

    #[test]
    fn 确认设计通过合法方案() {
        let 方案 = 设计方案 {
            要求id: "r1".to_string(),
            设计: "设计".to_string(),
            拆解: vec![拆解项 {
                目标: "目标".to_string(),
                执行层角色: vec![],
                工作流: "L2_script".to_string(),
            }],
            自评: "自评".to_string(),
        };
        assert_eq!(确认设计(&方案), 验收结论::通过);
    }

    #[test]
    fn 验收无产物打回() {
        let 回执 = 验收裁决("r1", &[], 0.0, &[], None);
        assert_eq!(回执.结论, 验收结论::打回);
    }

    #[test]
    fn 编译失败打回() {
        let 回执 = 验收裁决("r1", &[], 0.0, &[], Some("cargo build 失败"));
        assert_eq!(回执.结论, 验收结论::打回);
        assert_eq!(回执.验收意见.as_deref(), Some("cargo build 失败"));
    }

    #[test]
    fn 模块文件产物视为已接入() {
        // 造一个含完整模块声明链的临时 crate，产物包含各层模块.rs 与园实现文件。
        let 根 = std::env::temp_dir().join(format!("模块接入测试-{}", shihai_fu::当前毫秒()));
        let 园 = 根.join("观览-查询-殿/世界-观览-阁/流式-读取-园");
        std::fs::create_dir_all(&园).unwrap();
        std::fs::write(根.join("Cargo.toml"), "[package]\nname = \"测试-府\"\n").unwrap();
        std::fs::write(
            根.join("入口.rs"),
            "#[path = \"观览-查询-殿/模块.rs\"]\npub mod 观览_查询_殿;\n",
        )
        .unwrap();
        std::fs::write(
            根.join("观览-查询-殿/模块.rs"),
            "#[path = \"世界-观览-阁/模块.rs\"]\npub mod 世界_观览_阁;\npub use 世界_观览_阁::*;\n",
        )
        .unwrap();
        std::fs::write(
            根.join("观览-查询-殿/世界-观览-阁/模块.rs"),
            "#[path = \"流式-读取-园/模块.rs\"]\npub mod 流式_读取_园;\npub use 流式_读取_园::*;\n",
        )
        .unwrap();
        std::fs::write(
            园.join("模块.rs"),
            "#[path = \"流式读取.rs\"]\npub mod 流式读取;\npub use 流式读取::*;\n",
        )
        .unwrap();
        std::fs::write(
            园.join("流式读取.rs"),
            "pub fn 呈现世界时间() -> String {\"时间\".to_string()}\n",
        )
        .unwrap();

        let 锁 = 环境变量锁.lock().unwrap_or_else(|e| e.into_inner());
        let 旧根 = std::env::var("WORLD_WORKSPACE_ROOT").ok();
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 产物们 = vec![
            产物条目 {
                路径: "观览-查询-殿/世界-观览-阁/流式-读取-园/流式读取.rs".to_string(),
                类别: "代码".to_string(),
                字节数: 1,
                变化类型: "新增".to_string(),
            },
            产物条目 {
                路径: "观览-查询-殿/世界-观览-阁/流式-读取-园/模块.rs".to_string(),
                类别: "代码".to_string(),
                字节数: 1,
                变化类型: "新增".to_string(),
            },
            产物条目 {
                路径: "观览-查询-殿/世界-观览-阁/模块.rs".to_string(),
                类别: "代码".to_string(),
                字节数: 1,
                变化类型: "新增".to_string(),
            },
            产物条目 {
                路径: "观览-查询-殿/模块.rs".to_string(),
                类别: "代码".to_string(),
                字节数: 1,
                变化类型: "新增".to_string(),
            },
        ];
        let 回执 = 验收裁决("r1", &产物们, 0.0, &[], None);
        match 旧根 {
            Some(值) => std::env::set_var("WORLD_WORKSPACE_ROOT", 值),
            None => std::env::remove_var("WORLD_WORKSPACE_ROOT"),
        }
        drop(锁);
        let _ = std::fs::remove_dir_all(&根);

        assert_eq!(
            回执.结论,
            验收结论::通过,
            "模块.rs 自身也应视为已接入，验收意见：{:?}",
            回执.验收意见
        );
    }
}
