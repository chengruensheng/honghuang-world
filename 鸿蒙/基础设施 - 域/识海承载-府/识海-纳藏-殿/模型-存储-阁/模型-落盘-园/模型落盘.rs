//! 模型 - 落盘 - 园：工作区定位 + 心智模型聚合 + 格位/记录落盘读写。

use crate::{会话记录, 全部格位, 工具清单, 记录};
use rizhi_fu::{debug, error, info, warn};
use std::collections::HashSet;
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
