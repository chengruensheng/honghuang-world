//! 命令 - 沙箱 - 园：命令在隔离视图内运行，越界写入自动回滚。
//!
//! 目标：模型跑 cargo build / test 验证时，不能把真实工作区改坏、不能把构建物写进真实盘面。
//! 手段：
//! 1. 物化：工作区源码 → 硬链接镜像视图（零拷贝；增量复用，指纹一致不动；跨卷/权限失败降级复制）；
//! 2. 备份：命令前把源码内容快照到备份区（增量复用），供越界后恢复原内容；
//! 3. 运行：命令 cwd 落在视图根，构建物（道果树/target）随 .cargo 相对配置落在视图内，真实盘面零污染；
//! 4. 越界检测：命令前后指纹快照对比，源码区（视图或真实）任何新增/修改/删除都判越界；
//! 5. 回滚：变化文件用备份恢复（视图与真实同 inode，穿透修改需双侧都恢复），新增的删除；
//! 6. 并发锁：同一沙箱串行「物化+备份+快照+命令+检测回滚」原子事务，防多线程互扰。
//!
//! 沙箱目录：`.上下文/命令沙箱/{任务id}/视图` 与 `.../备份`；清理时整箱删除。

use crate::{运行命令超时, 默认超时毫秒};
use rizhi_fu::{debug, info, warn};
use shihai_fu::当前任务 as 取当前任务;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// 源码区排除目录：记忆数据、版本库、构建物、依赖树、临时目录不进视图、不进快照。
/// 与 识海承载-府 扫描排除项 对齐（含 临时文件夹——模型观测日志等临时文件不进视图，
/// 防命令误碰/误判越界，2026-08-17 轮5 体检对齐）。
const 排除目录们: [&str; 6] = [
    ".上下文",
    ".git",
    "道果树",
    "target",
    "node_modules",
    "临时文件夹",
];

/// sccache 可用性缓存：首次检查后复用，避免每次物化都 spawn 子进程探测。
/// sccache 不在 PATH 时返回 false，沙箱运行不注入 RUSTC_WRAPPER，不影响现有功能。
/// 探测带 5 秒超时（安全报告 L11）：超时视为不可用，防 sccache --version 挂死物化流程。
fn sccache可用() -> bool {
    static 缓存: OnceLock<bool> = OnceLock::new();
    *缓存.get_or_init(|| {
        // 用 运行命令超时 带 5 秒超时探测，超时或失败均视为 sccache 不可用。
        运行命令超时("sccache", &["--version"], None, 5_000, &[]).is_ok()
    })
}

/// 文件指纹：大小 + 修改纳秒（Windows NTFS 100ns 精度，足够捕捉命令改写）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct 指纹 {
    大小: u64,
    纳秒: u128,
}

/// 沙箱视图：命令隔离执行区。
pub struct 沙箱视图 {
    工作区根: PathBuf,
    视图根: PathBuf,
    备份根: PathBuf,
    锁: Mutex<()>,
}

/// 沙箱运行回执：命令结果 + 越界统计与详情。
#[derive(Clone, Debug, PartialEq)]
pub struct 沙箱结果 {
    pub 结果: crate::命令结果,
    pub 越界数: u32,
    pub 越界详情: String,
}

impl 沙箱视图 {
    /// 打开沙箱：目录落在 `.上下文/命令沙箱/{任务id}/`（空任务id 归「全局」分组）。
    pub fn 打开(工作区根: impl AsRef<Path>, 任务id: &str) -> 沙箱视图 {
        let 工作区根 = 工作区根.as_ref().to_path_buf();
        let 任务id = if 任务id.is_empty() {
            "全局"
        } else {
            任务id
        };
        let 箱 = 工作区根.join(".上下文").join("命令沙箱").join(任务id);
        info!(任务id, "沙箱已打开：{}", 箱.display());
        沙箱视图 {
            工作区根,
            视图根: 箱.join("视图"),
            备份根: 箱.join("备份"),
            锁: Mutex::new(()),
        }
    }

    /// 在当前任务下打开沙箱（任务id 取线程本地，未进入任务则归「全局」分组）。
    pub fn 打开当前(工作区根: impl AsRef<Path>) -> 沙箱视图 {
        沙箱视图::打开(工作区根, &取当前任务())
    }

    /// 物化 + 备份 + 前真实快照合并遍历（C3 优化，2026-08-22 入稿）：
    /// 原物化、备份、前真实快照各遍历一次工作区源码树（3 次），合并为 1 次遍历同时完成——
    /// 对每个源文件取一次指纹存前真实表、硬链接到视图、复制到备份。
    /// 命令前遍历从 4 次（物化+备份+前视图+前真实）减到 2 次（本合并遍历+前视图快照）。
    /// 前视图快照必须独立遍历视图（视图可能有真实区已删除的残留文件，物化增量只新增不清理）。
    fn 物化备份快照(&self) -> Result<HashMap<String, 指纹>, String> {
        fs::create_dir_all(&self.视图根)
            .map_err(|错误| format!("物化创建视图根失败：{}：{错误}", self.视图根.display()))?;
        let 来源们 = 遍历源码(&self.工作区根)?;
        let mut 新建 = 0u32;
        let mut 备份数 = 0u32;
        let mut 前真实 = HashMap::with_capacity(来源们.len());
        for 相对 in 来源们 {
            let 源 = self.工作区根.join(&相对);
            // 取源指纹存前真实表（与原快照逻辑一致：非文件不插入）。
            let 源指纹 = 取指纹(&源);
            if let Some(指纹) = 源指纹 {
                前真实.insert(相对.to_string_lossy().into_owned(), 指纹);
            }
            // 物化：硬链接到视图（增量：同指纹跳过，跨卷降级复制）。
            let 视图目标 = self.视图根.join(&相对);
            if !同指纹源(源指纹, &视图目标) {
                if let Some(父) = 视图目标.parent() {
                    fs::create_dir_all(父)
                        .map_err(|错误| format!("物化创建目录失败：{}：{错误}", 父.display()))?;
                }
                if 视图目标.exists() {
                    let _ = fs::remove_file(&视图目标);
                }
                if fs::hard_link(&源, &视图目标).is_err() {
                    // 跨卷/权限不支持硬链接：降级为真实复制（内容一致，仅不再共享 inode）。
                    fs::copy(&源, &视图目标)
                        .map_err(|错误| format!("物化复制失败：{}：{错误}", 源.display()))?;
                }
                新建 += 1;
            }
            // 备份：复制到备份区（增量：同指纹跳过）。
            let 备份目标 = self.备份根.join(&相对);
            if !同指纹源(源指纹, &备份目标) {
                if let Some(父) = 备份目标.parent() {
                    fs::create_dir_all(父)
                        .map_err(|错误| format!("备份创建目录失败：{}：{错误}", 父.display()))?;
                }
                if 备份目标.exists() {
                    let _ = fs::remove_file(&备份目标);
                }
                if let Err(错误) = fs::copy(&源, &备份目标) {
                    // 设计稿 §4.3 规则 4：单文件被占用/不可达 → 跳过，不阻断整条命令（防运行时锁文件卡死循环）。
                    warn!(相对 = %相对.display(), "备份跳过（文件被占用或不可达）：{错误}");
                    continue;
                }
                备份数 += 1;
            }
        }
        debug!(新建, 备份数, "物化+备份+前真实快照合并完成");
        Ok(前真实)
    }

    /// 源码区指纹快照：路径(相对) → 指纹。
    fn 快照(&self, 根: &Path) -> HashMap<String, 指纹> {
        let mut 表 = HashMap::new();
        if let Ok(相对们) = 遍历源码(根) {
            for 相对 in 相对们 {
                if let Some(指纹) = 取指纹(&根.join(&相对)) {
                    表.insert(相对.to_string_lossy().into_owned(), 指纹);
                }
            }
        }
        表
    }

    /// 运行命令：原子事务（物化备份快照 → 前视图快照 → 命令 → 越界检测回滚），全程加锁串行。
    /// `超时毫秒` 可选：None 走 默认超时毫秒（10 分钟）；超时后子进程被强杀并返回超时错误。
    pub fn 运行(
        &self,
        命令: &str,
        参数们: &[&str],
        工作目录: Option<&str>,
        超时毫秒: Option<u64>,
    ) -> Result<沙箱结果, String> {
        let _锁 = self.锁.lock().map_err(|_| "沙箱锁中毒".to_string())?;
        // 物化+备份+前真实快照合并遍历（C3）：一次遍历工作区同时完成三事，返回前真实指纹表。
        let 前真实 = self.物化备份快照()?;
        // 前视图快照独立遍历视图（视图可能有残留文件，必须独立扫）。
        let 前视图 = self.快照(&self.视图根);
        let 视图cwd = match 工作目录 {
            Some(相对) => self.视图根.join(相对).to_string_lossy().into_owned(),
            None => self.视图根.to_string_lossy().into_owned(),
        };
        let 超时 = 超时毫秒.unwrap_or(默认超时毫秒);
        info!(命令, cwd = %视图cwd, 超时, "沙箱内执行命令");
        // sccache 加速（设计稿 §11.2 规则 18）：沙箱视图排除 target/道果树，每次全量编译无缓存。
        // 修法：① 共享 CARGO_TARGET_DIR（.上下文/命令沙箱/共享构建缓存/）让 sccache 在同目录下
        // 缓存命中（sccache 缓存键含 --out-dir 路径，跨目录不命中）；② RUSTC_WRAPPER=sccache
        // 走全局缓存；③ CARGO_INCREMENTAL=0 让 sccache 缓存生效（sccache 不缓存增量编译单元）。
        // 注：cargo 的 rustc-wrapper 配置只在 $CARGO_HOME 生效、不在 workspace 根 .cargo 生效，
        // 故走环境变量注入。sccache 不可用时仅共享 target 目录（cargo 增量编译复用产物）。
        let 共享构建目录 = self
            .工作区根
            .join(".上下文")
            .join("命令沙箱")
            .join("共享构建缓存");
        let _ = fs::create_dir_all(&共享构建目录);
        let 共享构建路径 = 共享构建目录.to_string_lossy().into_owned();
        let sccache就绪 = sccache可用();
        let mut 环境: Vec<(&str, &str)> = vec![("CARGO_TARGET_DIR", 共享构建路径.as_str())];
        if sccache就绪 {
            环境.push(("RUSTC_WRAPPER", "sccache"));
            环境.push(("CARGO_INCREMENTAL", "0"));
        }
        let 结果 = 运行命令超时(命令, 参数们, Some(&视图cwd), 超时, &环境)?;
        let (越界数, 越界详情) = self.检测回滚(&前视图, &前真实)?;
        Ok(沙箱结果 {
            结果,
            越界数,
            越界详情,
        })
    }

    /// 越界检测与回滚：对比命令前后指纹，源码区变化一律判越界并恢复。
    fn 检测回滚(
        &self,
        前视图: &HashMap<String, 指纹>,
        前真实: &HashMap<String, 指纹>,
    ) -> Result<(u32, String), String> {
        let 后视图 = self.快照(&self.视图根);
        let 后真实 = self.快照(&self.工作区根);
        let mut 越界们: Vec<String> = Vec::new();
        // 已处理路径：视图区与真实区同 inode（穿透），记一次即可，真实区跳过避免重复计数。
        let mut 已处理: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 视图区变化（硬链接同 inode，命令改写视图会穿透真实，双侧都恢复）。
        for (路径, 后指纹) in &后视图 {
            match 前视图.get(路径) {
                None => {
                    self.删除视图与真实(路径)?;
                    已处理.insert(路径.clone());
                    越界们.push(format!("新增 {路径}（已删除）"));
                }
                Some(前) if 前 != 后指纹 => {
                    self.恢复视图与真实(路径)?;
                    已处理.insert(路径.clone());
                    越界们.push(format!("修改 {路径}（已恢复）"));
                }
                _ => {}
            }
        }
        for 路径 in 前视图.keys() {
            if !后视图.contains_key(路径) {
                self.恢复视图与真实(路径)?;
                已处理.insert(路径.clone());
                越界们.push(format!("删除 {路径}（已恢复）"));
            }
        }

        // 真实区变化（命令用绝对路径直接写真实盘面；穿透导致的真实变化已在视图区处理，跳过）。
        for (路径, 后指纹) in &后真实 {
            if 已处理.contains(路径) {
                continue;
            }
            match 前真实.get(路径) {
                None => {
                    self.删除真实(路径)?;
                    越界们.push(format!("真实区新增 {路径}（已删除）"));
                }
                Some(前) if 前 != 后指纹 => {
                    self.恢复视图与真实(路径)?;
                    越界们.push(format!("真实区修改 {路径}（已恢复）"));
                }
                _ => {}
            }
        }
        for 路径 in 前真实.keys() {
            if 已处理.contains(路径) {
                continue;
            }
            if !后真实.contains_key(路径) {
                self.恢复真实(路径)?;
                越界们.push(format!("真实区删除 {路径}（已恢复）"));
            }
        }

        if 越界们.is_empty() {
            info!("沙箱命令执行干净，零越界");
            Ok((0, String::new()))
        } else {
            let 详情 = 越界们.join("\n");
            warn!(
                越界数 = 越界们.len(),
                "沙箱拦截并回滚 {} 处越界写入",
                越界们.len()
            );
            Ok((越界们.len() as u32, 详情))
        }
    }

    /// 恢复：用备份内容覆盖 视图 与 真实（视图改为独立副本，断链防后续穿透）。
    fn 恢复视图与真实(&self, 相对: &str) -> Result<(), String> {
        let 备份 = self.备份根.join(相对);
        self.覆盖(相对, &self.视图根)?;
        self.覆盖(相对, &self.工作区根)?;
        if !备份.exists() {
            warn!(相对, "备份缺失：已按删除处理");
        }
        Ok(())
    }

    /// 覆盖单个区：备份存在 → 复制覆盖；备份缺失 → 删除目标（当作新增前不存在）。
    fn 覆盖(&self, 相对: &str, 区根: &Path) -> Result<(), String> {
        let 备份 = self.备份根.join(相对);
        let 目标 = 区根.join(相对);
        if 目标.exists() {
            if let Err(错误) = fs::remove_file(&目标) {
                // 设计稿 §4.3 规则 4：删除目标失败（被占用）→ 跳过，不阻断恢复流程。
                warn!(相对, "删除目标失败（文件被占用？）：{错误}");
                return Ok(());
            }
        }
        if 备份.exists() {
            if let Some(父) = 目标.parent() {
                fs::create_dir_all(父)
                    .map_err(|错误| format!("恢复创建目录失败：{}：{错误}", 父.display()))?;
            }
            if let Err(错误) = fs::copy(&备份, &目标) {
                warn!(相对, "恢复复制失败（文件被占用？）：{错误}");
            }
        }
        Ok(())
    }

    /// 恢复真实区：备份存在 → 覆盖；缺失 → 删除（当作新增前不存在）。
    fn 恢复真实(&self, 相对: &str) -> Result<(), String> {
        self.覆盖(相对, &self.工作区根)
    }

    fn 删除视图与真实(&self, 相对: &str) -> Result<(), String> {
        self.删除(相对, &self.视图根)?;
        self.删除(相对, &self.工作区根)?;
        Ok(())
    }

    fn 删除真实(&self, 相对: &str) -> Result<(), String> {
        self.删除(相对, &self.工作区根)
    }

    fn 删除(&self, 相对: &str, 区根: &Path) -> Result<(), String> {
        let 目标 = 区根.join(相对);
        if 目标.is_dir() {
            fs::remove_dir_all(&目标)
                .map_err(|错误| format!("删除目录失败：{}：{错误}", 目标.display()))?;
        } else if 目标.exists() {
            if let Err(错误) = fs::remove_file(&目标) {
                // 设计稿 §4.3 规则 4：删除失败（被占用）→ 跳过，不阻断回滚流程。
                warn!(相对, "删除文件失败（文件被占用？）：{错误}");
            }
        }
        Ok(())
    }

    /// 清理：任务结束后整箱删除（视图 + 备份 + 快照）。
    pub fn 清理(&self) -> Result<(), String> {
        let 箱 = self.视图根.parent().unwrap_or(&self.视图根);
        if 箱.is_dir() {
            fs::remove_dir_all(箱)
                .map_err(|错误| format!("沙箱清理失败：{}：{错误}", 箱.display()))?;
            debug!("沙箱已清理：{}", 箱.display());
        }
        Ok(())
    }
}

/// 遍历源码区文件（相对路径列表）：跳过排除目录。
fn 遍历源码(根: &Path) -> Result<Vec<PathBuf>, String> {
    let mut 文件们 = Vec::new();
    let mut 栈 = vec![根.to_path_buf()];
    while let Some(目录) = 栈.pop() {
        let 条目们 = fs::read_dir(&目录)
            .map_err(|错误| format!("读目录失败：{}：{错误}", 目录.display()))?;
        for 条目 in 条目们 {
            let 路径 = 条目.map_err(|错误| format!("读目录项失败：{错误}"))?.path();
            let 名 = 路径.file_name().and_then(|名| 名.to_str()).unwrap_or("");
            if 排除目录们.contains(&名) {
                continue;
            }
            if 路径.is_dir() {
                栈.push(路径);
            } else if 路径.is_file() {
                if let Ok(相对) = 路径.strip_prefix(根) {
                    文件们.push(相对.to_path_buf());
                }
            }
        }
    }
    文件们.sort();
    Ok(文件们)
}

/// 取文件指纹（非文件返回 None）。
fn 取指纹(路径: &Path) -> Option<指纹> {
    let 元 = fs::metadata(路径).ok()?;
    if !元.is_file() {
        return None;
    }
    let 纳秒 = 元
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(指纹 {
        大小: 元.len(),
        纳秒,
    })
}

/// 同指纹的源指纹已知变体（C3 合并遍历用）：源指纹已取，只取目标指纹比对，省一次源 metadata。
fn 同指纹源(源指纹: Option<指纹>, 目标: &Path) -> bool {
    match (源指纹, 取指纹(目标)) {
        (Some(甲), Some(乙)) => 甲 == 乙,
        _ => false,
    }
}
