//! 世界观览：世界状态 / 版本历史 / 版本详情
//!
//! 数据源：
//! - 世界状态 → `.上下文/状态/世界状态.jsonl`（最新一行）
//! - 版本历史 → `.上下文/状态/版本.jsonl`（全部行）
//! - 版本详情 → `.上下文/状态/版本.jsonl` 按版本号查询

use crate::工作区根;
use crate::状态目录;
use rizhi_fu::{debug, info, warn};

pub fn 呈现世界状态() -> String {
    let 状态目录 = 状态目录();
    match tianting_fu::读世界状态(&状态目录) {
        Ok(Some(状态)) => {
            debug!(阶段 = ?状态.阶段, v1已存档 = 状态.v1已存档, "世界状态已呈现");
            format!(
                "世界状态\n阶段：{:?}\nv1 已存档：{}\n进入路径：{:?}\n长期记忆长度：{} 字符\n界主想法池：{} 条\n在途要求：{} 条\n验收历史：{} 条\n版本历史：{} 条\n巡世候选池：{} 条\n天道报告库：{} 条",
                状态.阶段,
                状态.v1已存档,
                状态.进入路径,
                状态.长期记忆.len(),
                状态.界主想法池.len(),
                状态.在途要求.len(),
                状态.验收历史.len(),
                状态.版本历史.len(),
                状态.巡世候选池.len(),
                状态.天道报告库.len()
            )
        }
        Ok(None) => {
            warn!("世界状态文件不存在");
            "世界状态\n（未初始化，执行「版本 存档」自动写入：阶段=甲、v1已存档=false）".to_string()
        }
        Err(错误) => {
            warn!(错误 = %错误, "读世界状态失败");
            format!("读世界状态失败：{错误}")
        }
    }
}

pub fn 呈现队列水位() -> String {
    info!("队列水位已呈现");
    "队列水位\n在途要求：0（队列调度殿已建，端到端全流程由「想法 投递」直接驱动）".to_string()
}

pub fn 呈现版本历史() -> String {
    let 状态目录 = 状态目录();
    match tianting_fu::读版本历史(&状态目录) {
        Ok(记录们) => {
            if 记录们.is_empty() {
                warn!("版本历史为空");
                return "版本历史\n（暂无，用「版本 存档」创建）".to_string();
            }
            let mut 行 = format!("版本历史（{} 条）\n", 记录们.len());
            for 记录 in 记录们.iter().rev() {
                行.push_str(&format!(
                    "{} 阶段={:?} 时间={} 改了什么={}\n  源码快照：{}\n",
                    记录.版本号,
                    记录.阶段,
                    记录.时间,
                    记录.改了什么,
                    记录.源码快照路径
                ));
            }
            行
        }
        Err(错误) => {
            warn!(错误 = %错误, "读版本历史失败");
            format!("读版本历史失败：{错误}")
        }
    }
}

/// 版本详情：查指定版本的源码快照路径与完整字段。
pub fn 版本详情(版本号: &str) -> String {
    let 状态目录 = 状态目录();
    match tianting_fu::读版本历史(&状态目录) {
        Ok(记录们) => {
            for 记录 in &记录们 {
                if 记录.版本号 == 版本号 {
                    return format!(
                        "版本 {版本号}\n时间：{}\n阶段：{:?}\n改了什么：{}\n源码快照：{}\n构建产物：{}\n验收结论：{} 条\n对比上一版：{}\n",
                        记录.时间,
                        记录.阶段,
                        记录.改了什么,
                        记录.源码快照路径,
                        记录.构建产物路径,
                        记录.验收结论.len(),
                        记录.对比上一版
                    );
                }
            }
            format!("版本 {版本号} 未找到")
        }
        Err(错误) => {
            warn!(版本号, 错误 = %错误, "读版本历史失败");
            format!("读版本历史失败：{错误}")
        }
    }
}

/// 版本库根路径显示（兜底用，工作区根）。
pub fn _显示工作区() -> String {
    let 根 = 工作区根();
    根.display().to_string()
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use std::path::PathBuf;

    /// 本 crate 测试进程级 env 互斥锁：并行测试下 WORLD_WORKSPACE_ROOT 串行使用
    ///（照 终裁.rs 同款模式）。
    static 测试环境锁: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 造临时工作区：建 状态 目录并写指定 jsonl 内容，返回工作区根。
    fn 建临时工作区(名: &str, 文件们: &[(&str, &str)]) -> PathBuf {
        let 根 = std::env::temp_dir().join(format!("缓存读取测试-{名}-{}", std::process::id()));
        let 状态目录 = 根.join(".上下文").join("状态");
        std::fs::create_dir_all(&状态目录).unwrap();
        for (文件, 内容) in 文件们 {
            std::fs::write(状态目录.join(文件), 内容).unwrap();
        }
        根
    }

    fn 世界状态样例() -> String {
        let 状态 = serde_json::json!({
            "阶段": "乙",
            "v1已存档": true,
            "进入路径": "半路接手",
            "长期记忆": "",
            "界主想法池": [],
            "在途要求": [],
            "验收历史": [],
            "失败模式": [],
            "版本历史": [],
            "巡世候选池": [],
            "项目档案": null,
            "天道报告库": []
        });
        format!("{}\n", 状态)
    }

    fn 版本记录行(版本号: &str, 改了什么: &str) -> String {
        let 记录 = serde_json::json!({
            "版本号": 版本号,
            "时间": 1700000000000u64,
            "阶段": "乙",
            "改了什么": 改了什么,
            "源码快照路径": format!("版本-库/版本-{}/源码-快照", 版本号.replace('v', "")),
            "构建产物路径": "",
            "验收结论": [],
            "对比上一版": "增量 2 件"
        });
        format!("{}\n", 记录)
    }

    #[test]
    fn 呈现世界状态_读取存在状态() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 根 = 建临时工作区("存在状态", &[("世界状态.jsonl", &世界状态样例())]);
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 输出 = 呈现世界状态();
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = std::fs::remove_dir_all(&根);
        assert!(输出.contains("世界状态"), "应含标题：{输出}");
        assert!(输出.contains("乙"), "应含阶段：{输出}");
        assert!(输出.contains("true"), "应含 v1已存档：{输出}");
        assert!(输出.contains("半路接手"), "应含进入路径：{输出}");
    }

    #[test]
    fn 呈现世界状态_文件缺失提示未初始化() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 根 = 建临时工作区("无状态", &[]);
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 输出 = 呈现世界状态();
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = std::fs::remove_dir_all(&根);
        assert!(输出.contains("未初始化"), "应提示未初始化：{输出}");
    }

    #[test]
    fn 呈现版本历史_多记录倒序展示() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 内容 = format!("{}{}", 版本记录行("v2", "第二版"), 版本记录行("v3", "第三版"));
        let 根 = 建临时工作区("多版本", &[("版本.jsonl", &内容)]);
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 输出 = 呈现版本历史();
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = std::fs::remove_dir_all(&根);
        assert!(输出.contains("版本历史"), "应含标题：{输出}");
        assert!(输出.contains("v2"), "应含 v2：{输出}");
        assert!(输出.contains("v3"), "应含 v3：{输出}");
        assert!(输出.contains("第三版"), "应含改了什么：{输出}");
    }

    #[test]
    fn 呈现版本历史_空文件提示暂无() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 根 = 建临时工作区("空版本", &[]);
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 输出 = 呈现版本历史();
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = std::fs::remove_dir_all(&根);
        assert!(输出.contains("暂无"), "应提示暂无：{输出}");
    }

    #[test]
    fn 版本详情_命中返回完整字段() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 内容 = 版本记录行("v1", "初始版");
        let 根 = 建临时工作区("版本命中", &[("版本.jsonl", &内容)]);
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 输出 = 版本详情("v1");
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = std::fs::remove_dir_all(&根);
        assert!(输出.contains("版本 v1"), "应含版本号：{输出}");
        assert!(输出.contains("初始版"), "应含改了什么：{输出}");
        assert!(输出.contains("源码快照"), "应含源码快照：{输出}");
    }

    #[test]
    fn 版本详情_未命中提示找不到() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 内容 = 版本记录行("v1", "初始版");
        let 根 = 建临时工作区("版本未命中", &[("版本.jsonl", &内容)]);
        std::env::set_var("WORLD_WORKSPACE_ROOT", &根);
        let 输出 = 版本详情("v99");
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = std::fs::remove_dir_all(&根);
        assert!(输出.contains("未找到"), "应提示未找到：{输出}");
    }
}