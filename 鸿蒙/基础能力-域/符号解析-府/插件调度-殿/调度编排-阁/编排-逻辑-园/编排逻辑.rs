//! 编排逻辑：符号索引引擎主体
//!
//! 职责链（设计 §5.1）：
//! 读解析定义 → 读插件清单并校验 → 按后缀分发 → 校验产出
//! → 归一化（join 坐标）→ 合并输出
//!
//! **本文件不含任何具体语言名词**（设计决策 1）。

use crate::{
    边, 产出, 悬空, 插件清单, 插件状态, 索引元信息, 索引符号, 索引统计, 符号索引, 解析定义,
};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

/// 图谱产物文件名（数据契约）
const 解析定义文件: &str = "解析定义.json";
const 坐标索引文件: &str = "坐标索引.json";
const 符号索引文件: &str = "符号索引.json";
const 符号索引摘要文件: &str = "符号索引.md";
/// 插件交换产出后缀（引擎与插件进程的私有约定）
const 交换产出后缀: &str = ".json";
/// 坐标索引的节点类型字段与文件节点取值（外部数据契约）
const 节点类型键: &str = "类型";
const 文件节点值: &str = "文件";

/// 符号索引引擎。以仓库根构造，一次运行产出全量符号索引。
pub struct 编排器 {
    仓库根: PathBuf,
    图谱目录: PathBuf,
}

impl 编排器 {
    pub fn 新(仓库根: impl Into<PathBuf>) -> Self {
        let 仓库根 = 仓库根.into();
        let 图谱目录 = 仓库根.join(".传承").join("图谱").join("知识图谱");
        Self { 仓库根, 图谱目录 }
    }

    fn 定义路径(&self) -> PathBuf {
        self.图谱目录.join(解析定义文件)
    }

    fn 坐标索引路径(&self) -> PathBuf {
        self.图谱目录.join(坐标索引文件)
    }

    /// 执行一次全量索引，并把结果写入 `.传承/图谱/知识图谱/`
    pub fn 运行(&self) -> Result<符号索引, String> {
        let 定义 = self.读定义()?;
        let 坐标表 = self.读坐标表();
        let 限定: HashSet<String> = 坐标表.keys().cloned().collect();

        let mut 全部符号: Vec<索引符号> = Vec::new();
        let mut 全部边: Vec<边> = Vec::new();
        let mut 全部悬空: Vec<悬空> = Vec::new();
        let mut 插件状态们: Vec<插件状态> = Vec::new();
        let mut 后缀占用: BTreeMap<String, String> = BTreeMap::new();

        for 登记 in &定义.启用插件 {
            let 清单路径 = self.仓库根.join(&登记.清单);
            if !清单路径.is_file() {
                插件状态们.push(插件状态 {
                    语言: 登记.语言.clone(),
                    状态: "跳过".into(),
                    符号数: 0,
                    边数: 0,
                    信息: format!("清单未找到：{}", 登记.清单),
                });
                continue;
            }

            let 清单 = match self.读清单(&清单路径) {
                Ok(值) => 值,
                Err(原因) => {
                    插件状态们.push(插件状态 {
                        语言: 登记.语言.clone(),
                        状态: "失败".into(),
                        符号数: 0,
                        边数: 0,
                        信息: 原因,
                    });
                    continue;
                }
            };

            if let Err(原因) = crate::校验清单(&清单) {
                插件状态们.push(插件状态 {
                    语言: 清单.语言.clone(),
                    状态: "失败".into(),
                    符号数: 0,
                    边数: 0,
                    信息: 原因,
                });
                continue;
            }

            // 后缀冲突：拒绝启动而非静默择一（设计 §5.2）
            let mut 冲突 = None;
            for 后缀 in &清单.文件后缀 {
                if let Some(已有) = 后缀占用.get(后缀) {
                    if 已有 != &清单.语言 {
                        冲突 = Some(format!(
                            "后缀 {} 被 {} 与 {} 同时声明",
                            后缀, 已有, 清单.语言
                        ));
                        break;
                    }
                }
            }
            if let Some(原因) = 冲突 {
                return Err(format!("插件后缀冲突：{原因}"));
            }
            for 后缀 in &清单.文件后缀 {
                后缀占用.insert(后缀.clone(), 清单.语言.clone());
            }

            let 文件们 = 收集文件(&清单.文件后缀, &限定);
            if 文件们.is_empty() {
                插件状态们.push(插件状态 {
                    语言: 清单.语言.clone(),
                    状态: "成功".into(),
                    符号数: 0,
                    边数: 0,
                    信息: "无匹配文件".into(),
                });
                continue;
            }

            match self.调用插件(&清单, &文件们) {
                Ok(mut 包) => {
                    if let Err(原因) = crate::校验产出(&包, &清单.语言) {
                        插件状态们.push(插件状态 {
                            语言: 清单.语言.clone(),
                            状态: "失败".into(),
                            符号数: 0,
                            边数: 0,
                            信息: 原因,
                        });
                        continue;
                    }

                    let 去重 = crate::去重产出(&mut 包);
                    let 降级数 = crate::降级悬空边(&mut 包);
                    let 符号数 = 包.符号.len();
                    let 边数 = 包.边.len();
                    let 诊断数 = 包.诊断.len();

                    for 符 in 包.符号 {
                        let 坐标 = 坐标表.get(&符.文件).cloned().unwrap_or_default();
                        全部符号.push(索引符号 {
                            符号: 符,
                            来源语言: 清单.语言.clone(),
                            坐标,
                        });
                    }
                    全部边.extend(包.边);
                    全部悬空.extend(包.悬空);

                    let mut 信息 = format!("{} 个文件", 文件们.len());
                    if 去重.符号重复 > 0 || 去重.边重复 > 0 {
                        信息.push_str(&format!(
                            "；去重 符号 {} / 边 {}",
                            去重.符号重复, 去重.边重复
                        ));
                    }
                    if 去重.同名异种类 > 0 {
                        信息.push_str(&format!(
                            "；同 ID 异种类 {} 对（合法，未合并）：{}",
                            去重.同名异种类,
                            去重.同名样例.join("、")
                        ));
                    }
                    if 降级数 > 0 {
                        信息.push_str(&format!("；{降级数} 条边降级为悬空"));
                    }
                    if 诊断数 > 0 {
                        信息.push_str(&format!("；插件诊断 {诊断数} 条"));
                    }
                    插件状态们.push(插件状态 {
                        语言: 清单.语言.clone(),
                        状态: "成功".into(),
                        符号数,
                        边数,
                        信息,
                    });
                }
                Err(原因) => {
                    插件状态们.push(插件状态 {
                        语言: 清单.语言.clone(),
                        状态: "失败".into(),
                        符号数: 0,
                        边数: 0,
                        信息: 原因,
                    });
                }
            }
        }

        let 索引 = 符号索引 {
            元信息: 索引元信息 {
                图谱: "符号索引".into(),
                版本: 定义.版本.clone(),
                生成时间: 当前时间(),
                仓库根: self.仓库根.display().to_string(),
                定义文件: self.定义路径().display().to_string(),
                生成器: "hm-symext（鸿蒙/基础能力-域/符号解析-府）".into(),
            },
            统计: 统计(&全部符号, &全部边, &全部悬空, 插件状态们),
            符号: 全部符号,
            边: 全部边,
            悬空: 全部悬空,
        };

        self.写索引(&索引)?;
        Ok(索引)
    }

    // ---------- 读 ----------

    fn 读定义(&self) -> Result<解析定义, String> {
        let 路径 = self.定义路径();
        if !路径.is_file() {
            return Err(format!("解析定义未找到：{}", 路径.display()));
        }
        let 文本 = std::fs::read_to_string(&路径).map_err(|e| format!("读解析定义失败：{e}"))?;
        serde_json::from_str(&文本).map_err(|e| format!("解析定义 JSON 非法：{e}"))
    }

    fn 读清单(&self, 路径: &PathBuf) -> Result<插件清单, String> {
        let 文本 = std::fs::read_to_string(路径).map_err(|e| format!("读插件清单失败：{e}"))?;
        serde_json::from_str(&文本).map_err(|e| format!("插件清单 JSON 非法：{e}"))
    }

    /// 坐标表：仓库相对路径 → 坐标。由坐标索引 join 而来。
    fn 读坐标表(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        let mut 表 = BTreeMap::new();
        let 文本 = match std::fs::read_to_string(self.坐标索引路径()) {
            Ok(t) => t,
            Err(_) => return 表,
        };
        let 值: serde_json::Value = match serde_json::from_str(&文本) {
            Ok(v) => v,
            Err(_) => return 表,
        };
        let 节点们 = match 值.get("节点").and_then(|v| v.as_array()) {
            Some(a) => a,
            None => return 表,
        };
        for 节点 in 节点们 {
            if 节点.get(节点类型键).and_then(|v| v.as_str()) != Some(文件节点值) {
                continue;
            }
            let 路径 = match 节点.get("路径").and_then(|v| v.as_str()) {
                Some(p) => p.to_string(),
                None => continue,
            };
            let mut 坐标 = BTreeMap::new();
            if let Some(对象) = 节点.get("坐标").and_then(|v| v.as_object()) {
                for (键, 值) in 对象 {
                    if let Some(s) = 值.as_str() {
                        坐标.insert(键.clone(), s.to_string());
                    }
                }
            }
            表.insert(路径, 坐标);
        }
        表
    }

    // ---------- 调 ----------

    fn 调用插件(&self, 清单: &插件清单, 文件们: &[String]) -> Result<产出, String> {
        let 临时 = std::env::temp_dir();
        let 清单文件 = 临时.join(format!("hm-symext-{}-文件.txt", 清单.语言));
        let 产出文件 = 临时.join(format!("hm-symext-{}-产出{交换产出后缀}", 清单.语言));

        std::fs::write(&清单文件, 文件们.join("\n"))
            .map_err(|e| format!("写文件清单失败：{e}"))?;

        let 程序 = 解析程序路径(&清单.入口[0]);
        let mut 命令 = Command::new(&程序);
        命令.args(&清单.入口[1..]);
        命令.current_dir(&self.仓库根);
        命令.arg("--项目根").arg(&self.仓库根);
        命令.arg("--清单").arg(&清单文件);
        命令.arg("--输出").arg(&产出文件);

        let 结果 = 命令
            .output()
            .map_err(|e| format!("启动插件失败（{}）：{e}", 程序.display()))?;
        if !结果.status.success() {
            let 错误 = String::from_utf8_lossy(&结果.stderr);
            return Err(format!(
                "插件退出码 {:?}：{}",
                结果.status.code(),
                错误.trim()
            ));
        }
        if !产出文件.is_file() {
            return Err("插件未产出文件".into());
        }
        let 文本 = std::fs::read_to_string(&产出文件).map_err(|e| format!("读产出失败：{e}"))?;
        serde_json::from_str(&文本).map_err(|e| format!("产出 JSON 非法：{e}"))
    }

    // ---------- 写 ----------

    fn 写索引(&self, 索引: &符号索引) -> Result<(), String> {
        let json = serde_json::to_string_pretty(索引)
            .map_err(|e| format!("序列化符号索引失败：{e}"))?;
        std::fs::write(self.图谱目录.join(符号索引文件), json)
            .map_err(|e| format!("写符号索引失败：{e}"))?;
        std::fs::write(self.图谱目录.join(符号索引摘要文件), 摘要(索引))
            .map_err(|e| format!("写符号索引摘要失败：{e}"))?;
        Ok(())
    }
}

/// 按后缀从坐标索引限定的文件集中筛出待解析文件
fn 收集文件(后缀们: &[String], 限定: &HashSet<String>) -> Vec<String> {
    let mut 结果: Vec<String> = 限定
        .iter()
        .filter(|路径| 后缀们.iter().any(|后缀| 路径.ends_with(后缀.as_str())))
        .cloned()
        .collect();
    结果.sort();
    结果
}

/// 解析插件可执行文件：优先取与引擎同目录的同名文件，否则回退为 PATH 查找。
///
/// 之所以不直接用 `cargo run`：引擎若由 `cargo run` 启动，会持有 target 锁，
/// 子进程再 `cargo run` 将互等死锁。
fn 解析程序路径(名: &str) -> PathBuf {
    if let Ok(可执行) = std::env::current_exe() {
        if let Some(目录) = 可执行.parent() {
            for 尾巴 in ["", ".exe"] {
                let 候选 = 目录.join(format!("{名}{尾巴}"));
                if 候选.is_file() {
                    return 候选;
                }
            }
        }
    }
    PathBuf::from(名)
}

fn 统计(
    符号们: &[索引符号],
    边们: &[边],
    悬空们: &[悬空],
    插件们: Vec<插件状态>,
) -> 索引统计 {
    let mut 按种类: BTreeMap<String, usize> = BTreeMap::new();
    let mut 按语言: BTreeMap<String, usize> = BTreeMap::new();
    let mut 按坐标根: BTreeMap<String, usize> = BTreeMap::new();

    for 项 in 符号们 {
        *按语言.entry(项.来源语言.clone()).or_insert(0) += 1;
        let 种类名 = serde_json::to_value(项.符号.种类)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "未知".to_string());
        *按种类.entry(种类名).or_insert(0) += 1;
        let 根 = 项
            .坐标
            .get("根")
            .cloned()
            .unwrap_or_else(|| "(未归类)".to_string());
        *按坐标根.entry(根).or_insert(0) += 1;
    }

    索引统计 {
        符号总数: 符号们.len(),
        边总数: 边们.len(),
        悬空总数: 悬空们.len(),
        按种类,
        按语言,
        按坐标根,
        插件: 插件们,
    }
}

fn 摘要(索引: &符号索引) -> String {
    let mut 行: Vec<String> = Vec::new();
    行.push("# 洪荒·世界 · 知识图谱 · 符号索引".to_string());
    行.push(String::new());
    行.push("> **定位**：知识图谱第 2 阶段（符号层）——把源码解析为统一符号与事实边。".to_string());
    行.push("> **引擎**：`hm-symext`（语言无关）；**插件**：每语言一个独立进程（见 `解析定义.json`）。".to_string());
    行.push("> **数据本体**：`符号索引.json`（机器消费，本文件只是摘要）。".to_string());
    行.push(format!("> **生成时间**：{}", 索引.元信息.生成时间));
    行.push(String::new());
    行.push("---".to_string());
    行.push(String::new());

    行.push("## 一、总览".to_string());
    行.push(String::new());
    行.push("| 指标 | 数值 |".to_string());
    行.push("|---|---|".to_string());
    行.push(format!("| 符号总数 | {} |", 索引.统计.符号总数));
    行.push(format!("| 边总数 | {} |", 索引.统计.边总数));
    行.push(format!("| 悬空总数 | {} |", 索引.统计.悬空总数));
    行.push(String::new());

    行.push("## 二、插件状态".to_string());
    行.push(String::new());
    行.push("| 语言 | 状态 | 符号数 | 边数 | 信息 |".to_string());
    行.push("|---|---|---|---|---|".to_string());
    for 状态 in &索引.统计.插件 {
        行.push(format!(
            "| {} | {} | {} | {} | {} |",
            状态.语言, 状态.状态, 状态.符号数, 状态.边数, 状态.信息
        ));
    }
    行.push(String::new());

    行.push("## 三、按坐标根分布".to_string());
    行.push(String::new());
    行.push("| 根 | 符号数 |".to_string());
    行.push("|---|---|".to_string());
    for (根, 数) in &索引.统计.按坐标根 {
        行.push(format!("| {根} | {数} |"));
    }
    行.push(String::new());

    行.push("## 四、按符号种类分布".to_string());
    行.push(String::new());
    行.push("| 种类 | 数量 |".to_string());
    行.push("|---|---|".to_string());
    for (种类, 数) in &索引.统计.按种类 {
        行.push(format!("| {种类} | {数} |"));
    }
    行.push(String::new());

    行.push("## 五、悬空引用（前 20 条）".to_string());
    行.push(String::new());
    if 索引.悬空.is_empty() {
        行.push("（无）".to_string());
    }
    else {
        行.push("| 来源 | 目标文本 | 类型 | 置信度 |".to_string());
        行.push("|---|---|---|---|".to_string());
        for 项 in 索引.悬空.iter().take(20) {
            行.push(format!(
                "| {} | {} | {:?} | {:?} |",
                项.从, 项.目标文本, 项.类型, 项.置信度
            ));
        }
    }
    行.push(String::new());

    行.join("\n") + "\n"
}

/// UTC+8 的 `YYYY-MM-DD HH:MM`，避免引入日期库
fn 当前时间() -> String {
    let 秒 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + 8 * 3600;
    let 天 = (秒 / 86400) as i64;
    let 余 = 秒 % 86400;
    let (时, 分) = (余 / 3600, (余 % 3600) / 60);
    let (年, 月, 日) = 民用日期(天);
    format!("{年:04}-{月:02}-{日:02} {时:02}:{分:02}")
}

/// Howard Hinnant 的 civil_from_days 算法
fn 民用日期(天: i64) -> (i64, u32, u32) {
    let z = 天 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
