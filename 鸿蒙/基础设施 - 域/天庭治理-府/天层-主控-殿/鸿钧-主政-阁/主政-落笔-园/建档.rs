//! 建档：世界认识自己——机械扫描工作区 → 项目档案 → 写入世界状态。
//! 设计稿 §6.7：项目档案（规模/结构地图/构建状态/风格约定/成熟度/基线版本）。
//! 动机：世界状态.项目档案 长期为 null，世界对自己没有档案（鸿钧答不出「项目什么状态」）。

use crate::类型_定义_殿::{世界状态, 成熟度, 构建状态, 规模统计, 项目档案};
use rizhi_fu::info;
use shihai_fu::世界结果;

/// 扫描工作区生成项目档案（纯机械事实，不调 LLM）。
pub fn 生成项目档案() -> 项目档案 {
    let 根 = shihai_fu::工作区::定位();
    let 根路径 = 根.根路径();
    let 规模 = 统计规模(根路径);
    let 结构地图 = 生成结构地图(根路径);
    let 已知坑 = 扫描已知坑(根路径);
    let 成熟度 = if 规模.rs文件数 > 0 {
        成熟度::成熟完整
    } else {
        成熟度::半成品
    };
    项目档案 {
        来源: 根路径.display().to_string(),
        接手时间: shihai_fu::当前毫秒(),
        规模,
        结构地图,
        关键接口: vec!["跨府引用止步 lib 根（六层纪律）".to_string()],
        构建状态: 构建状态::可编译,
        风格约定: "全中文标识符 · 六层结构（维度/域/府/殿/阁/园）· 目录连字符/模块下划线 #[path] 桥接 · 技术日志走 rizhi_fu".to_string(),
        已知坑,
        成熟度,
        基线版本: 读最新版本号(),
        最近任务成功率: 读最近成功率(),
    }
}

/// 最近任务成功率：验收.jsonl 尾部 10 条的通过比例（生产化 2.3，世界自我认知关键指标）。
fn 读最近成功率() -> String {
    let 目录 = 状态目录();
    let 队列 = crate::落盘队列::<crate::终裁回执>::打开(目录.join("验收.jsonl"));
    let 验收们 = 队列.读全部().unwrap_or_default();
    let 尾部: Vec<&crate::终裁回执> = 验收们.iter().rev().take(10).collect();
    if 尾部.is_empty() {
        return "（暂无验收记录）".to_string();
    }
    let 通过数 = 尾部
        .iter()
        .filter(|回执| 回执.验收.结论 == crate::验收结论::通过)
        .count();
    format!(
        "通过 {通过数}/{} · {}%",
        尾部.len(),
        通过数 * 100 / 尾部.len()
    )
}

/// 规模统计：rs 文件数 / 总行数 / crate 数（读根 Cargo.toml members）。
fn 统计规模(根: &std::path::Path) -> 规模统计 {
    let mut rs文件数 = 0usize;
    let mut 总行数 = 0u64;
    递归统计(根, &mut rs文件数, &mut 总行数);
    // crate 数：读根 Cargo.toml members 段（含 `-府"` 的行即成员，适配单行/多行 members）。
    let crate数 = std::fs::read_to_string(根.join("Cargo.toml"))
        .map(|内容| 内容.lines().filter(|行| 行.contains("-府\"")).count())
        .unwrap_or(0) as u32;
    规模统计 {
        rs文件数: rs文件数 as u32,
        总行数,
        crate数,
    }
}

/// 扫描排除项：与识海承载-府 扫描排除一致（版本库/构建物/临时/工具目录不入世界认知）。
fn 应排除(路径: &std::path::Path) -> bool {
    const 排除名们: &[&str] = &[
        ".git",
        ".svn",
        ".hg",
        ".上下文",
        ".cargo",
        ".arts",
        ".codeartsdoer",
        "target",
        "node_modules",
        "vendor",
        "临时文件夹",
        "道果树",
    ];
    if let Some(名) = 路径.file_name().and_then(|名| 名.to_str()) {
        return 排除名们.contains(&名);
    }
    false
}

fn 递归统计(目录: &std::path::Path, rs文件数: &mut usize, 总行数: &mut u64) {
    if let Ok(条目们) = std::fs::read_dir(目录) {
        for 条目 in 条目们.flatten() {
            let 路径 = 条目.path();
            if 路径.is_dir() {
                if !应排除(&路径) {
                    递归统计(&路径, rs文件数, 总行数);
                }
            } else if 路径.extension().and_then(|e| e.to_str()) == Some("rs") {
                *rs文件数 += 1;
                if let Ok(内容) = std::fs::read_to_string(&路径) {
                    *总行数 += 内容.lines().count() as u64;
                }
            }
        }
    }
}

/// 结构地图：维度 → 域/府 概览（府 = 含 Cargo.toml 的目录，带域前缀）。
fn 生成结构地图(根: &std::path::Path) -> String {
    let mut 行们 = Vec::new();
    if let Ok(维度们) = std::fs::read_dir(根) {
        for 维度 in 维度们.flatten() {
            let 维度路径 = 维度.path();
            if !维度路径.is_dir() || 维度.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let 维度名 = 维度.file_name().to_string_lossy().to_string();
            let mut 府们 = Vec::new();
            if let Ok(域们) = std::fs::read_dir(&维度路径) {
                for 域 in 域们.flatten() {
                    let 域路径 = 域.path();
                    if !域路径.is_dir() {
                        continue;
                    }
                    let 域名 = 域.file_name().to_string_lossy().to_string();
                    if 域路径.join("Cargo.toml").exists() {
                        // 域直接是府
                        府们.push(域名);
                        continue;
                    }
                    if let Ok(项们) = std::fs::read_dir(&域路径) {
                        for 项 in 项们.flatten() {
                            if 项.path().is_dir() && 项.path().join("Cargo.toml").exists() {
                                府们.push(format!("{域名}/{}", 项.file_name().to_string_lossy()));
                            }
                        }
                    }
                }
            }
            if !府们.is_empty() {
                行们.push(format!("{维度名}/{}", 府们.join("、")));
            }
        }
    }
    if 行们.is_empty() {
        "（无）".to_string()
    } else {
        行们.join("\n")
    }
}

/// 已知坑：扫描出可机械判定的历史遗留（空目录园等）。
fn 扫描已知坑(根: &std::path::Path) -> Vec<String> {
    let mut 坑们 = Vec::new();
    递归找空园(根, &mut 坑们);
    坑们
}

fn 递归找空园(目录: &std::path::Path, 坑们: &mut Vec<String>) {
    if let Ok(条目们) = std::fs::read_dir(目录) {
        for 条目 in 条目们.flatten() {
            let 路径 = 条目.path();
            if 路径.is_dir() && !应排除(&路径) {
                递归找空园(&路径, 坑们);
            }
        }
    }
    // 园目录（名字以 -园 结尾）且无任何文件 → 残留。
    if 目录
        .file_name()
        .map(|名| 名.to_string_lossy().ends_with("-园"))
        .unwrap_or(false)
        && std::fs::read_dir(目录)
            .map(|mut 条目| 条目.next().is_none())
            .unwrap_or(false)
    {
        坑们.push(format!("空园残留：{}", 目录.display()));
    }
}

/// 状态目录：工作区根下的 .上下文/状态（与 世界运行.rs 同款，本文件复制以保持跨府引用只走 lib 根符号的边界）。
fn 状态目录() -> std::path::PathBuf {
    let 根 = std::env::var("WORLD_WORKSPACE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    根.join(".上下文").join("状态")
}

/// 最新版本号：读世界状态.版本历史 末条。
fn 读最新版本号() -> String {
    let 路径 = 状态目录().join("世界状态.jsonl");
    let 内容 = match std::fs::read_to_string(&路径) {
        Ok(内容) => 内容,
        Err(_) => return "（无版本历史）".to_string(),
    };
    let 末条 = 内容.lines().last().unwrap_or_default();
    match serde_json::from_str::<世界状态>(末条) {
        Ok(状态) => 状态
            .版本历史
            .last()
            .map(|记录| 记录.版本号.clone())
            .unwrap_or_else(|| "（无版本历史）".to_string()),
        Err(_) => "（无版本历史）".to_string(),
    }
}

/// 建档并写入世界状态.项目档案（幂等：重复建档覆盖旧档案）。
pub fn 建档落盘() -> 世界结果<String> {
    let 档案 = 生成项目档案();
    let 目录 = 状态目录();
    let mut 状态 = crate::确保世界状态初始化(&目录)?;
    状态.项目档案 = Some(档案.clone());
    crate::写世界状态(&目录, &状态)?;
    info!(规模 = ?档案.规模, 成熟度 = ?档案.成熟度, 坑数 = 档案.已知坑.len(), "项目档案已建档");
    let mut 报告 = format!(
        "项目档案已建档\n规模：{} 个 rs 文件 · {} 行 · {} 个 crate\n成熟度：{:?}\n基线版本：{}\n最近任务成功率：{}\n结构地图：\n{}",
        档案.规模.rs文件数, 档案.规模.总行数, 档案.规模.crate数, 档案.成熟度, 档案.基线版本, 档案.最近任务成功率, 档案.结构地图
    );
    if !档案.已知坑.is_empty() {
        报告.push_str(&format!("\n已知坑：\n{}", 档案.已知坑.join("\n")));
    }
    Ok(报告)
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 统计规模_空目录不崩() {
        let 根 = std::env::temp_dir().join(format!("建档测试-{}", std::process::id()));
        std::fs::create_dir_all(&根).unwrap();
        std::fs::write(根.join("a.rs"), "fn 甲() {}\nfn 乙() {}\n").unwrap();
        std::fs::write(
            根.join("Cargo.toml"),
            "[workspace]\nmembers = [\"某-府\"]\n",
        )
        .unwrap();
        let 规模 = 统计规模(&根);
        assert_eq!(规模.rs文件数, 1);
        assert_eq!(规模.总行数, 2);
        assert_eq!(规模.crate数, 1);
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 生成结构地图_识别维度内府() {
        let 根 = std::env::temp_dir().join(format!("建档地图-{}", std::process::id()));
        let 府 = 根.join("乾坤").join("呈现-域").join("命令操作-府");
        std::fs::create_dir_all(&府).unwrap();
        std::fs::write(府.join("Cargo.toml"), "").unwrap();
        let 地图 = 生成结构地图(&根);
        assert!(
            地图.contains("呈现-域/命令操作-府"),
            "应识别域内府，实际：{地图}"
        );
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 扫描已知坑_空园被检出() {
        let 根 = std::env::temp_dir().join(format!("建档坑-{}", std::process::id()));
        let 空园 = 根.join("某-府").join("某-殿").join("空-园");
        std::fs::create_dir_all(&空园).unwrap();
        let 坑们 = 扫描已知坑(&根);
        assert_eq!(坑们.len(), 1);
        assert!(坑们[0].contains("空园残留"));
        let _ = std::fs::remove_dir_all(&根);
    }
}
