//! 模型 - 落盘 - 园：工作区定位 + 心智模型聚合 + 格位/记录落盘读写。

use crate::{会话记录, 全部格位, 工具清单, 记录};
use rizhi_fu::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// 上下文目录名（工作区内的记忆数据目录，源码快照时排除）。
pub const 上下文目录名: &str = ".上下文";

/// 工作区：目标项目根 + 记忆数据目录（.上下文）。
#[derive(Clone, Debug, PartialEq)]
pub struct 工作区 {
    根路径: PathBuf,
}

impl 工作区 {
    /// 构造工作区（不创建目录）。
    pub fn 新(根路径: impl AsRef<Path>) -> 工作区 {
        工作区 {
            根路径: 根路径.as_ref().to_path_buf(),
        }
    }

    /// 定位工作区根：环境变量 → 向上探测锚点 → 当前目录。
    pub fn 定位() -> 工作区 {
        if let Ok(根) = std::env::var("WORLD_WORKSPACE_ROOT") {
            if !根.is_empty() {
                return 工作区::新(根);
            }
        }
        if let Ok(当前) = std::env::current_dir() {
            for 目录 in 当前.ancestors() {
                if 目录.join(上下文目录名).is_dir()
                    || 目录.join("Cargo.toml").exists()
                    || 目录.join("AGENTS.md").exists()
                {
                    return 工作区::新(目录);
                }
            }
        }
        工作区::新(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// 初始化记忆目录结构（模板落盘）。
    pub fn 初始化(&self) -> Result<(), String> {
        fs::create_dir_all(self.格位目录()).map_err(|错误| format!("建格位目录失败: {错误}"))?;
        fs::create_dir_all(self.会话目录()).map_err(|错误| format!("建会话目录失败: {错误}"))?;
        debug!(根 = %self.根路径.display(), "记忆目录已就绪");
        Ok(())
    }

    /// 上下文目录：根路径/.上下文。
    pub fn 上下文目录(&self) -> PathBuf {
        self.根路径.join(上下文目录名)
    }

    /// 工作区根路径。
    pub fn 根路径(&self) -> &Path {
        &self.根路径
    }

    /// 格位目录：.上下文/格位。
    pub fn 格位目录(&self) -> PathBuf {
        self.上下文目录().join("格位")
    }

    /// 会话目录：.上下文/会话。
    pub fn 会话目录(&self) -> PathBuf {
        self.上下文目录().join("会话")
    }

    /// 依赖图路径：.上下文/依赖图.json。
    pub fn 依赖图路径(&self) -> PathBuf {
        self.上下文目录().join("依赖图.json")
    }
}

/// 构建产物目录的本质：读 .cargo/config.toml 的 target-dir 首段路径。
/// 「道果树」被排除不是因为名字，而是因为它是构建产物的落点。
pub fn 构建产物目录(根: &Path) -> Option<String> {
    let 配置 = 根.join(".cargo").join("config.toml");
    let 内容 = std::fs::read_to_string(配置).ok()?;
    for 行 in 内容.lines() {
        let 行 = 行.trim();
        if let Some(值) = 行.strip_prefix("target-dir") {
            let 值 = 值.trim_start();
            if !值.starts_with('=') {
                continue;
            }
            let 值 = 值.trim_start_matches('=').trim().trim_matches('"');
            if let Some(首段) = 值
                .split(|字符: char| ['/', '\\'].contains(&字符))
                .find(|段| !段.is_empty())
            {
                return Some(首段.to_string());
            }
        }
    }
    None
}

/// 扫描排除项：版本库 / 依赖 / 构建产物 / 临时文件夹（默认名）+ 本质识别的构建产物目录。
pub fn 扫描排除项(根: &Path) -> Vec<String> {
    let mut 项 = vec![
        ".git".to_string(),
        ".svn".to_string(),
        ".hg".to_string(),
        ".上下文".to_string(),
        "node_modules".to_string(),
        "vendor".to_string(),
        "target".to_string(),
        "临时文件夹".to_string(),
    ];
    if let Some(构建产物) = 构建产物目录(根) {
        项.push(构建产物);
    }
    项
}

/// 心智模型整体：工作区 + 工具清单 + 格位们 + 会话记录（内存聚合，不落盘）。
#[derive(Clone, Debug, PartialEq)]
pub struct 心智模型 {
    pub 工作区: 工作区,
    pub 工具清单: 工具清单,
    pub 格位们: Vec<crate::格位>,
    pub 会话记录: Vec<会话记录>,
}

impl 心智模型 {
    /// 新建心智模型：默认装载 36 格位。
    pub fn 新(工作区: 工作区) -> 心智模型 {
        心智模型 {
            工作区,
            工具清单: 工具清单::全部(),
            格位们: 全部格位(),
            会话记录: Vec::new(),
        }
    }
}

/// 模型存储：格位目录下的格位文件落盘读写。
#[derive(Clone)]
pub struct 模型存储 {
    格位目录: PathBuf,
}

impl 模型存储 {
    /// 打开指定格位目录（目录不存在则创建）。
    pub fn 打开(格位目录: impl AsRef<Path>) -> 模型存储 {
        let 格位目录 = 格位目录.as_ref().to_path_buf();
        let _ = fs::create_dir_all(&格位目录);
        模型存储 { 格位目录 }
    }

    /// 在工作区根下打开（格位落 .上下文/格位/）。
    pub fn 在工作区(工作区: &工作区) -> 模型存储 {
        模型存储::打开(工作区.格位目录())
    }

    /// 追加写入一条记录（jsonl 一行）。
    pub fn 写记录(&self, 记录: &记录) -> Result<(), String> {
        let 路径 = self.格位文件路径(&记录.格位名);
        let 行 = serde_json::to_string(记录).map_err(|错误| format!("序列化记录失败: {错误}"))?;
        use std::io::Write;
        let mut 文件 = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&路径)
            .map_err(|错误| {
                error!(格位 = %记录.格位名, 路径 = %路径.display(), "打开格位文件失败：{错误}");
                format!("打开格位文件失败 {}: {错误}", 路径.display())
            })?;
        writeln!(文件, "{行}").map_err(|错误| {
            error!(格位 = %记录.格位名, "写记录失败：{错误}");
            format!("写记录失败: {错误}")
        })?;
        debug!(格位 = %记录.格位名, "记录已写入");
        Ok(())
    }

    /// 读某个格位的全部记录（按写入顺序）。
    pub fn 读格位(&self, 格位名: &str) -> Result<Vec<记录>, String> {
        let 路径 = self.格位文件路径(格位名);
        if !路径.exists() {
            return Ok(Vec::new());
        }
        let 内容 = fs::read_to_string(&路径).map_err(|错误| {
            error!(格位 = %格位名, 路径 = %路径.display(), "读格位文件失败：{错误}");
            format!("读格位文件失败 {}: {错误}", 路径.display())
        })?;
        let 记录们 = 内容
            .lines()
            .filter(|行| !行.trim().is_empty())
            .map(|行| {
                serde_json::from_str::<记录>(行).map_err(|错误| format!("解析记录失败: {错误}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        info!(格位 = %格位名, 条数 = 记录们.len(), "格位已读出");
        Ok(记录们)
    }

    /// 格位文件路径：格位目录 / 格位名.jsonl。
    fn 格位文件路径(&self, 格位名: &str) -> PathBuf {
        self.格位目录.join(format!("{格位名}.jsonl"))
    }

    /// 读某个格位的链头集：按实体键分组，每组取时间戳最新一条（默认拉最新）。
    /// 实体键为空的旧记录按内容兜底分组。
    pub fn 读链头集(&self, 格位名: &str) -> Result<Vec<记录>, String> {
        let 全部 = self.读格位(格位名)?;
        let mut 链头 = Vec::new();
        let mut 已见 = HashSet::new();
        for 记录 in 全部.into_iter().rev() {
            let 键 = if 记录.实体键.is_empty() {
                记录.内容.clone()
            } else {
                记录.实体键.clone()
            };
            if 已见.insert(键) {
                链头.push(记录);
            }
        }
        链头.reverse();
        Ok(链头)
    }

    /// 清洗格位：reducer 四步（去重 + 剔失效 + 分组留链头 + 标矛盾）→ 重写 jsonl 只含链头。
    /// 纯代码 reducer（设计稿 §14.20）：不调 LLM、零 token，机械判定归代码。
    /// 幂等：对已清洗的 jsonl 再跑一次，去重数/剔除失效数/矛盾清单均为空。
    pub fn 清洗格位(&self, 格位名: &str) -> Result<清洗报告, String> {
        let 全部 = self.读格位(格位名)?;
        let 原条数 = 全部.len();

        // 步1 去重：同内容指纹（内容+证据+分组键）只留时间戳最新。
        let mut 指纹表: HashMap<(String, String, String), 记录> = HashMap::new();
        for 记录 in 全部 {
            let 键 = 内容指纹(&记录);
            指纹表
                .entry(键)
                .and_modify(|旧| {
                    if 记录.时间戳 > 旧.时间戳 {
                        *旧 = 记录.clone();
                    }
                })
                .or_insert(记录);
        }
        let 去重后: Vec<记录> = 指纹表.into_values().collect();
        let 去重数 = 原条数.saturating_sub(去重后.len());

        // 步2 剔失效：物理清理失效记录。
        let 剔除前 = 去重后.len();
        let 有效: Vec<记录> = 去重后.into_iter().filter(|记录| !记录.失效).collect();
        let 剔除失效数 = 剔除前.saturating_sub(有效.len());

        // 步4 标矛盾：同分组键不同内容指纹的记录对标（在有效记录里，供上层聚焦，不剔除）。
        let mut 按分组键: HashMap<String, Vec<记录>> = HashMap::new();
        for 记录 in &有效 {
            按分组键.entry(分组键(记录)).or_default().push(记录.clone());
        }
        let mut 矛盾清单: Vec<矛盾> = 按分组键
            .into_iter()
            .filter(|(_, 组)| 组.len() > 1)
            .map(|(键, mut 组)| {
                组.sort_by_key(|记录| 记录.时间戳);
                矛盾 {
                    实体键: 键,
                    冲突记录们: 组,
                }
            })
            .collect();
        矛盾清单.sort_by(|甲, 乙| 甲.实体键.cmp(&乙.实体键));

        // 步3 分组留链头：按分组键分组，每组留时间戳最新一条（链头 = 实体当前状态）。
        let mut 链头表: HashMap<String, 记录> = HashMap::new();
        for 记录 in 有效 {
            链头表
                .entry(分组键(&记录))
                .and_modify(|旧| {
                    if 记录.时间戳 > 旧.时间戳 {
                        *旧 = 记录.clone();
                    }
                })
                .or_insert(记录);
        }
        let mut 链头们: Vec<记录> = 链头表.into_values().collect();
        链头们.sort_by_key(|记录| 记录.时间戳);
        let 分组留链头数 = 链头们.len();

        // 重写 jsonl 只含链头（物理清理失效 + 重复 + 旧版本）。
        self.重写格位(格位名, &链头们)?;

        info!(
            格位 = %格位名,
            原条数,
            剔除失效数,
            去重数,
            分组留链头数,
            矛盾数 = 矛盾清单.len(),
            "格位已清洗"
        );
        Ok(清洗报告 {
            原条数,
            剔除失效数,
            去重数,
            分组留链头数,
            矛盾清单,
        })
    }

    /// 重写格位 jsonl：覆盖写入给定记录（按顺序一行一条）。
    fn 重写格位(&self, 格位名: &str, 记录们: &[记录]) -> Result<(), String> {
        let 路径 = self.格位文件路径(格位名);
        let mut 文本 = String::new();
        for 记录 in 记录们 {
            let 行 =
                serde_json::to_string(记录).map_err(|错误| format!("序列化记录失败: {错误}"))?;
            文本.push_str(&行);
            文本.push('\n');
        }
        fs::write(&路径, 文本).map_err(|错误| {
            error!(格位 = %格位名, 路径 = %路径.display(), "重写格位失败：{错误}");
            format!("重写格位失败 {}: {错误}", 路径.display())
        })?;
        debug!(格位 = %格位名, 条数 = 记录们.len(), "格位已重写");
        Ok(())
    }
}

/// 分组键：实体键为空时按内容兜底（与 读链头集 同策略，兼容旧式无键记录）。
fn 分组键(记录: &记录) -> String {
    if 记录.实体键.is_empty() {
        记录.内容.clone()
    } else {
        记录.实体键.clone()
    }
}

/// 内容指纹：(内容, 证据, 分组键) ——同分组键下同内容同证据视为重复。
fn 内容指纹(记录: &记录) -> (String, String, String) {
    (记录.内容.clone(), 记录.证据.clone(), 分组键(记录))
}

/// 清洗报告：reducer 四步后的统计 + 矛盾清单（设计稿 §14.20.6）。
#[derive(Clone, Debug, PartialEq)]
pub struct 清洗报告 {
    /// 清洗前 jsonl 总条数。
    pub 原条数: usize,
    /// `失效=true` 物理剔除条数。
    pub 剔除失效数: usize,
    /// 同内容指纹只留最新后剔除的重复条数。
    pub 去重数: usize,
    /// 按分组键留链头后写入 jsonl 的最终条数。
    pub 分组留链头数: usize,
    /// 同分组键不同内容指纹的记录对标，供上层聚焦裁决。
    pub 矛盾清单: Vec<矛盾>,
}

/// 矛盾：同分组键下多条不同内容指纹的记录，供上层聚焦裁决（不剔除，只标记）。
#[derive(Clone, Debug, PartialEq)]
pub struct 矛盾 {
    /// 触发矛盾的分组键（实体键，或空实体键时为内容兜底）。
    pub 实体键: String,
    /// 同键下不同内容指纹的冲突记录（按时间戳升序）。
    pub 冲突记录们: Vec<记录>,
}

/// 代码变更后，证据指向变更路径的记录标记失效，返回失效条数（防幻觉）。
pub fn 标记证据失效(记录们: &mut [记录], 变更路径: &str) -> usize {
    let mut 计数 = 0;
    for 记录 in 记录们.iter_mut() {
        if !记录.失效 && 记录.证据.contains(变更路径) {
            记录.失效 = true;
            计数 += 1;
        }
    }
    if 计数 > 0 {
        warn!(变更路径, 失效条数 = 计数, "证据失效标记");
    }
    计数
}

#[cfg(test)]
mod 测试 {
    //! 模型 - 落盘 - 园 · 清洗格位 reducer 测试（设计稿 §14.20）：
    //! 去重 + 剔失效 + 分组留链头 + 标矛盾 + 幂等 + 空实体键兜底。

    use super::*;
    use crate::来源;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 临时格位目录：pid + 计数 + 标签 隔离并行测试。
    fn 临时格位目录(标签: &str) -> PathBuf {
        static 计数: AtomicU64 = AtomicU64::new(0);
        let n = 计数.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let 目录 = std::env::temp_dir().join(format!("shihai_清洗格位_{pid}_{n}_{标签}"));
        let _ = std::fs::remove_dir_all(&目录);
        std::fs::create_dir_all(&目录).unwrap();
        目录
    }

    /// 造一条记录（手动设时间戳，避免 当前毫秒 抖动）。
    fn 造记录(
        实体键: &str, 内容: &str, 证据: &str, 时间戳: u64, 失效: bool
    ) -> 记录 {
        记录 {
            格位名: "测试格位".to_string(),
            内容: 内容.to_string(),
            证据: 证据.to_string(),
            时间戳,
            来源: 来源::代码,
            前记录: None,
            失效,
            实体键: 实体键.to_string(),
            规则级别: None,
        }
    }

    #[test]
    fn 清洗_去重剔失效分组留链头标矛盾() {
        let 目录 = 临时格位目录("四步");
        let 存储 = 模型存储::打开(&目录);
        let 记录1 = 造记录("甲", "甲内容", "甲证据", 100, false);
        let 记录2 = 造记录("甲", "甲内容", "甲证据", 200, false); // 与记录1同指纹，去重留此
        let 记录3 = 造记录("甲", "甲内容新", "甲证据", 300, false); // 同实体键不同内容，矛盾
        let 记录4 = 造记录("乙", "乙内容", "乙证据", 400, true); // 失效且指纹唯一，剔失效
        let 记录5 = 造记录("乙", "乙内容新", "乙证据", 500, false); // 乙链头（与记录4不同指纹）
        for 记录 in [&记录1, &记录2, &记录3, &记录4, &记录5] {
            存储.写记录(记录).unwrap();
        }

        let 报告 = 存储.清洗格位("测试格位").unwrap();
        assert_eq!(报告.原条数, 5);
        assert_eq!(报告.去重数, 1, "记录1与记录2同指纹应去重1");
        assert_eq!(报告.剔除失效数, 1, "记录4失效应剔除1");
        assert_eq!(报告.分组留链头数, 2, "甲留链头+乙留链头=2");
        assert_eq!(报告.矛盾清单.len(), 1, "甲实体键有两条不同内容指纹应标矛盾");
        let 矛盾 = &报告.矛盾清单[0];
        assert_eq!(矛盾.实体键, "甲");
        assert_eq!(矛盾.冲突记录们.len(), 2);
        assert_eq!(矛盾.冲突记录们[0].时间戳, 200, "冲突记录按时间戳升序");
        assert_eq!(矛盾.冲突记录们[1].时间戳, 300);

        let 读回 = 存储.读格位("测试格位").unwrap();
        assert_eq!(读回.len(), 2, "jsonl 应只含2条链头");
        assert_eq!(读回[0].时间戳, 300, "甲链头=记录3（最新）");
        assert_eq!(读回[1].时间戳, 500, "乙链头=记录5");
        let _ = std::fs::remove_dir_all(&目录);
    }

    #[test]
    fn 清洗_幂等() {
        let 目录 = 临时格位目录("幂等");
        let 存储 = 模型存储::打开(&目录);
        let 记录1 = 造记录("甲", "甲内容", "甲证据", 100, false);
        let 记录2 = 造记录("甲", "甲内容", "甲证据", 200, false);
        存储.写记录(&记录1).unwrap();
        存储.写记录(&记录2).unwrap();
        存储.清洗格位("测试格位").unwrap();

        let 报告 = 存储.清洗格位("测试格位").unwrap();
        assert_eq!(报告.去重数, 0, "已清洗应无重复");
        assert_eq!(报告.剔除失效数, 0, "已清洗应无失效");
        assert!(报告.矛盾清单.is_empty(), "已清洗应无矛盾");
        assert_eq!(报告.分组留链头数, 1);
        let _ = std::fs::remove_dir_all(&目录);
    }

    #[test]
    fn 清洗_空实体键按内容兜底不误判矛盾() {
        let 目录 = 临时格位目录("空键");
        let 存储 = 模型存储::打开(&目录);
        let 记录1 = 造记录("", "同内容", "同证据", 100, false);
        let 记录2 = 造记录("", "同内容", "同证据", 200, false); // 同指纹，去重
        let 记录3 = 造记录("", "异内容", "同证据", 300, false); // 空键按内容兜底，不同组
        存储.写记录(&记录1).unwrap();
        存储.写记录(&记录2).unwrap();
        存储.写记录(&记录3).unwrap();

        let 报告 = 存储.清洗格位("测试格位").unwrap();
        assert_eq!(报告.去重数, 1, "记录1与记录2同指纹应去重1");
        assert_eq!(报告.分组留链头数, 2, "兜底分组键：同内容+异内容=2组");
        assert!(
            报告.矛盾清单.is_empty(),
            "空键按内容兜底，不同内容归不同组，不矛盾"
        );
        let _ = std::fs::remove_dir_all(&目录);
    }

    #[test]
    fn 清洗_空格位返回空报告() {
        let 目录 = 临时格位目录("空格位");
        let 存储 = 模型存储::打开(&目录);
        let 报告 = 存储.清洗格位("不存在").unwrap();
        assert_eq!(报告.原条数, 0);
        assert_eq!(报告.分组留链头数, 0);
        let _ = std::fs::remove_dir_all(&目录);
    }
}
