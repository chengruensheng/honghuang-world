use std::io::Read;
use std::path::{Component as 路径组件, Path, PathBuf};
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
// Windows 专属：raw_arg 让 cmd /C 收到未经 std 转义的原样命令串（见 运行命令_超时 内注释）
use std::os::windows::process::CommandExt;
use std::time::{Duration, Instant};
use hm_contract::Component;
use hm_error::{Error, Result};
use hm_execute_contract::执行器;

/// 默认命令超时（秒）：仅在未显式设置时生效，便于单元测试与最小化构造
pub const 默认命令超时秒: u64 = 30;
/// 默认单次命令最大输出字节（64 KiB，防止输出过大撑爆内存）：仅在未显式设置时生效
pub const 默认最大输出字节: u64 = 64 * 1024;
/// 感知工具（Glob/Grep）单次返回结果条数上限，防止结果过多撑爆上下文
pub const 条目上限: usize = 200;
/// 搜索内容时单个文件的最大字节（1 MiB）：超大文件跳过，避免读入内存
const 单文件最大字节: u64 = 1024 * 1024;
/// 允许执行的命令白名单（自主开发场景所需的安全命令集；白名单外命令一律拒绝）
const 允许命令白名单: &[&str] = &[
    "cargo", "rustc", "rustup", "dir", "type", "echo", "cd",
    "where", "findstr", "set", "cls", "chcp", "ping",
];

/// 脚本扩展名黑名单（拒绝执行脚本文件，防止白名单外命令通过脚本间接执行）
const 脚本扩展名: &[&str] = &[".bat", ".cmd", ".ps1", ".vbs", ".js", ".wsf", ".msi"];

/// 本地执行器：读写文件、运行命令。**读写分离**——
///
/// - **写**（写文件/精确编辑/删除文件）与**工作区命令**严格锁定工作区：拒绝绝对路径与 `..` 越界；
/// - **读**（读文件/列目录/按名找文件/搜索内容）可越出工作区探索本体：`~/` 前缀指向项目根（只读根首项），
///   绝对路径须落在只读根集合内，且命中读黑名单（`.git`/`.env`）时拒绝。
/// - 命令 `~ ` 前缀则在项目根 cwd 下执行，且仅允许只读白名单（cargo build/test 等被拒）。
///
/// 命令带超时与输出大小限制，避免越权与资源失控。
pub struct 本地执行器 {
    工作区: PathBuf,
    命令超时秒: u64,
    最大输出字节: u64,
    /// 只读根集合：仅影响读操作，写操作仍锁工作区。默认空 = 只有工作区可读。
    只读根: Vec<PathBuf>,
}

/// 工作区规范化：相对路径（如配置 `./工作区`）锚定进程 cwd 转绝对，再经 components 重组剥掉 `.` 冗余组件。
///
/// 为什么必须：`按名找文件` 用 glob 匹配后以 `strip_prefix(工作区)` 截相对路径；
/// glob 返回的路径不含 `./`（被 glob 规范化），而根含 `CurDir` 组件时组件逐一比对失配，
/// 匹配项被静默丢弃 → 感知工具在生产配置下永远返回「（无匹配）」。
pub fn 规范化工作区(路径: PathBuf) -> PathBuf {
    let 绝对 = if 路径.is_absolute() {
        路径
    } else {
        match std::env::current_dir() {
            Ok(当前) => 当前.join(路径),
            Err(_) => 路径,
        }
    };
    绝对.components().collect()
}

impl 本地执行器 {
    /// 以指定工作区根构造本地执行器（采用写死默认值：超时 30 秒、最大输出 64 KiB）。
    /// 配置化场景请改用 `new_with_limits` 或链式 `设置命令超时` / `设置最大输出`。
    /// 构造时自动创建工作区目录，确保命令执行与文件操作有有效目录（P0 修复：目录不存在导致全部工具调用失败）。
    ///
    /// 工作区规范化（P1 修复）：相对路径（如配置 `./工作区`）一律转绝对路径并剥掉 `.` 组件。
    /// 否则 `按名找文件` 中 glob 返回的规范化路径与相对根做 `strip_prefix` 时组件失配，
    /// 匹配项被静默丢弃，感知工具在生产环境永远返回「（无匹配）」。
    pub fn new(工作区: impl Into<PathBuf>) -> Self {
        let 路径 = 工作区.into();
        if let Err(e) = std::fs::create_dir_all(&路径) {
            eprintln!("警告：无法创建工作区目录 {}: {}", 路径.display(), e);
        }
        本地执行器 { 工作区: 规范化工作区(路径), 命令超时秒: 默认命令超时秒, 最大输出字节: 默认最大输出字节, 只读根: Vec::new() }
    }

    /// 以指定工作区根与显式上限构造本地执行器（从配置注入超时与输出上限，替代写死默认值）
    pub fn new_with_limits(工作区: impl Into<PathBuf>, 命令超时秒: u64, 最大输出字节: u64) -> Self {
        let 路径 = 工作区.into();
        if let Err(e) = std::fs::create_dir_all(&路径) {
            eprintln!("警告：无法创建工作区目录 {}: {}", 路径.display(), e);
        }
        本地执行器 { 工作区: 规范化工作区(路径), 命令超时秒, 最大输出字节, 只读根: Vec::new() }
    }

    /// 设置命令超时（秒），链式构造
    pub fn 设置命令超时(mut self, 秒: u64) -> Self {
        self.命令超时秒 = 秒;
        self
    }

    /// 设置单次命令最大输出字节数，链式构造
    pub fn 设置最大输出(mut self, 字节: u64) -> Self {
        self.最大输出字节 = 字节;
        self
    }

    /// 设置只读根集合（项目根 + 白名单子目录）：供读操作越出工作区探索本体用。
    /// 每个根均规范化；默认空 = 仅工作区可读。写操作与工作区命令不受影响。
    pub fn 设置只读根(mut self, 根们: Vec<PathBuf>) -> Self {
        self.只读根 = 根们.into_iter().map(规范化工作区).collect();
        self
    }

    /// 将相对路径解析到工作区内；拒绝绝对路径、`~/` 前缀与 `..` 越界。
    /// 写操作专用：`~/` 只在读路径有意义，写路径必须显式拒绝（否则会被当普通相对路径
    /// 在工作区内建出 `~` 目录，形成读写边界不一致的漏洞）。
    fn 解析路径(&self, 路径: &str) -> Result<PathBuf> {
        let 相对 = Path::new(路径);
        if 路径.starts_with("~/")
            || 相对.is_absolute()
            || 相对.components().any(|c| matches!(c, 路径组件::ParentDir))
        {
            return Err(Error::Config(format!("非法路径（越出工作区）: {路径}")));
        }
        Ok(self.工作区.join(相对))
    }

    /// 解析读路径：支持三种形态——
    ///   1) `~/` 开头 → 相对项目根（只读根集合首项）
    ///   2) 绝对路径 → 仅当落在只读根集合内才允许
    ///   3) 普通相对路径 → 相对工作区（现状，拒绝 `..`）
    /// 最终规范化后必须落在「工作区 ∪ 只读根」内且不命中读黑名单，否则拒绝。
    fn 解析读路径(&self, 路径: &str) -> Result<PathBuf> {
        let 候选 = if let Some(余) = 路径.strip_prefix("~/") {
            let 项目根 = self
                .只读根
                .first()
                .ok_or_else(|| Error::Config("未配置只读根（~/ 前缀不可用）".into()))?;
            项目根.join(余)
        } else if Path::new(路径).is_absolute() {
            PathBuf::from(路径)
        } else {
            let 相对 = Path::new(路径);
            if 相对.components().any(|c| matches!(c, 路径组件::ParentDir)) {
                return Err(Error::Config(format!("非法路径（越出工作区）: {路径}")));
            }
            self.工作区.join(相对)
        };
        let 规范化 = 词法归一化(&候选)
            .ok_or_else(|| Error::Config(format!("非法路径（`..` 越出根）: {路径}")))?;
        self.校验可读(&规范化)?;
        Ok(规范化)
    }

    /// 读边界校验：落在「工作区 ∪ 只读根」内，且不命中读黑名单。
    fn 校验可读(&self, 规范化: &Path) -> Result<()> {
        let 在工作区内 = 规范化.starts_with(&self.工作区);
        let 在只读根内 = self.只读根.iter().any(|根| 规范化.starts_with(根));
        if !在工作区内 && !在只读根内 {
            return Err(Error::Config(format!(
                "非法路径（越出只读范围）: {}",
                规范化.display()
            )));
        }
        if 命中读黑名单(规范化) {
            return Err(Error::Config(format!(
                "无权限读取（命中读黑名单）: {}",
                规范化.display()
            )));
        }
        Ok(())
    }

    /// 解析 glob 模式的根与展示前缀：`~/` 开头 → 项目根（返回路径带 `~/` 前缀）；
    /// 否则工作区（返回相对工作区路径，现状）。
    fn 解析glob根(&self, 模式: &str) -> Result<(PathBuf, String, String)> {
        if let Some(余) = 模式.strip_prefix("~/") {
            let 项目根 = self
                .只读根
                .first()
                .ok_or_else(|| Error::Config("未配置只读根（~/ 前缀不可用）".into()))?;
            Ok((项目根.clone(), 余.to_string(), "~/".to_string()))
        } else {
            Ok((self.工作区.clone(), 模式.to_string(), String::new()))
        }
    }

    /// 轮询等待子进程退出；超时则终止进程并报错
    fn 等待退出(&self, 子进程: &mut std::process::Child, 命令: &str, 超时秒: u64) -> Result<std::process::ExitStatus> {
        let 开始 = Instant::now();
        loop {
            match 子进程.try_wait().map_err(Error::Io)? {
                Some(状态) => return Ok(状态),
                None => {
                    if 开始.elapsed() >= Duration::from_secs(超时秒) {
                        // 终止整个进程树（cmd 及其子进程），避免残留孤儿进程
                        let 进程号 = 子进程.id();
                        if let Err(失败) = Command::new("taskkill")
                            .args(["/F", "/T", "/PID", &进程号.to_string()])
                            .output()
                        {
                            tracing::warn!("终止超时进程树失败: {失败}");
                        }
                        if let Err(失败) = 子进程.wait() {
                            tracing::warn!("等待超时进程退出失败: {失败}");
                        }
                        return Err(Error::命令超时(命令.to_string()));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }

    /// 递归遍历目录，逐文件按行匹配关键词，命中写入「展示前缀 + 相对路径:行号:内容」
    fn 递归搜索(&self, 根: &Path, 前缀: &str, 目录: &Path, 关键词: &str, 结果: &mut Vec<String>) -> Result<()> {
        let 条目 = std::fs::read_dir(目录).map_err(Error::Io)?;
        let mut 项集: Vec<PathBuf> = Vec::new();
        for 项 in 条目 {
            项集.push(项.map_err(Error::Io)?.path());
        }
        项集.sort();
        for 路径 in 项集 {
            if 结果.len() >= 条目上限 {
                break;
            }
            if 路径.is_dir() {
                if !应跳过目录(&路径) {
                    self.递归搜索(根, 前缀, &路径, 关键词, 结果)?;
                }
            } else if 路径.is_file() {
                // 读黑名单对内容检索同样生效（.env 密钥 / .git 内部）：防经搜索绕过
                if !命中读黑名单(&路径) {
                    self.搜索单文件(根, 前缀, &路径, 关键词, 结果);
                }
            }
        }
        Ok(())
    }

    /// 读单个文本文件并按行匹配关键词；跳过超大文件与二进制（含 NUL 字节）
    fn 搜索单文件(&self, 根: &Path, 前缀: &str, 路径: &Path, 关键词: &str, 结果: &mut Vec<String>) {
        let 元数据 = match std::fs::metadata(路径) {
            Ok(m) => m,
            Err(_) => return,
        };
        if 元数据.len() > 单文件最大字节 {
            return;
        }
        let 字节 = match std::fs::read(路径) {
            Ok(b) => b,
            Err(_) => return,
        };
        if 字节.contains(&0) {
            return;
        }
        let 文本 = String::from_utf8_lossy(&字节);
        for (序号, 行) in 文本.lines().enumerate() {
            if 行.contains(关键词) {
                if let Ok(相对) = 路径.strip_prefix(根) {
                    结果.push(format!("{前缀}{}:{}:{}", 相对.to_string_lossy(), 序号 + 1, 行));
                }
            }
        }
    }
}

/// 判断目录是否应跳过（构建产物、仓库元数据、依赖目录，避免误搜噪声）
fn 应跳过目录(路径: &Path) -> bool {
    let 名 = 路径.file_name().and_then(|n| n.to_str());
    matches!(
        名,
        Some("target") | Some(".git") | Some("node_modules")
    ) || 名.is_some_and(|n| n.eq_ignore_ascii_case(super::删除防护::回收站名))
}

/// 词法归一化绝对路径：解析 `.` 与 `..`，不触碰文件系统。
/// `..` 试图弹出前缀/根（越出盘根）时返回 None——读路径的 `~/../../` 逃逸防护关键。
fn 词法归一化(路径: &Path) -> Option<PathBuf> {
    let mut 栈: Vec<路径组件> = Vec::new();
    for 组件 in 路径.components() {
        match 组件 {
            路径组件::Prefix(_) | 路径组件::RootDir => 栈.push(组件),
            路径组件::CurDir => {}
            路径组件::ParentDir => match 栈.last() {
                Some(路径组件::Normal(_)) => {
                    栈.pop();
                }
                _ => return None,
            },
            路径组件::Normal(_) => 栈.push(组件),
        }
    }
    Some(栈.iter().collect())
}

/// 读黑名单命中判定：路径任一组件为 `.git`，或文件名以 `.env` 开头。
/// 防智能体放开读项目根后泄密钥（.env）或读到仓库内部对象（.git）。
fn 命中读黑名单(路径: &Path) -> bool {
    if 路径.components().any(|c| c.as_os_str().to_string_lossy() == ".git") {
        return true;
    }
    if let Some(名) = 路径.file_name().and_then(|n| n.to_str()) {
        if 名 == ".env" || 名.starts_with(".env.") {
            return true;
        }
    }
    false
}

impl Component for 本地执行器 {
    fn name(&self) -> &'static str { "本地执行器" }
}

impl 执行器 for 本地执行器 {
    fn 读文件(&self, 路径: &str) -> Result<String> {
        let 目标 = self.解析读路径(路径)?;
        std::fs::read_to_string(目标).map_err(Error::Io)
    }

    fn 写文件(&self, 路径: &str, 内容: &str) -> Result<()> {
        let 目标 = self.解析路径(路径)?;
        if let Some(父目录) = 目标.parent() {
            std::fs::create_dir_all(父目录).map_err(Error::Io)?;
        }
        // 覆盖前先备份原文件，防止误覆盖丢失内容
        if 目标.exists() {
            std::fs::copy(&目标, 备份路径(&目标)).map_err(Error::Io)?;
        }
        std::fs::write(目标, 内容).map_err(Error::Io)
    }

    fn 运行命令(&self, 命令: &str) -> Result<String> {
        self.运行命令_超时(命令, self.命令超时秒)
    }

    fn 运行命令_限时(&self, 命令: &str, 超时秒: u64) -> Result<String> {
        self.运行命令_超时(命令, 超时秒)
    }

    fn 列目录(&self, 路径: &str) -> Result<String> {
        let 目标 = self.解析读路径(路径)?;
        let 条目 = std::fs::read_dir(&目标).map_err(Error::Io)?;
        let mut 结果: Vec<String> = Vec::new();
        for 项 in 条目 {
            let 项 = 项.map_err(Error::Io)?;
            let 名 = 项.file_name().to_string_lossy().into_owned();
            let 类型 = if 项.path().is_dir() { "[目录]" } else { "[文件]" };
            结果.push(format!("{类型} {名}"));
        }
        结果.sort();
        if 结果.is_empty() {
            Ok("（空目录）".to_string())
        } else {
            Ok(结果.join("\n"))
        }
    }

    fn 按名找文件(&self, 模式: &str) -> Result<String> {
        let (根, 相对模式, 前缀) = self.解析glob根(模式)?;
        let 绝对模式 = 根.join(相对模式);
        let 模式字符串 = 绝对模式.to_string_lossy().into_owned();
        let 匹配 = glob::glob(&模式字符串)
            .map_err(|e| Error::Config(format!("无效 glob 模式 {模式}: {e}")))?;
        let mut 结果: Vec<String> = Vec::new();
        for 项 in 匹配 {
            match 项 {
                Ok(路径) if 路径.is_file() => {
                    // 读黑名单（.env 密钥 / .git 内部）对感知类读同样生效，防经 glob 绕过
                    if 命中读黑名单(&路径) {
                        continue;
                    }
                    if let Ok(相对) = 路径.strip_prefix(&根) {
                        // 回收站内是已删除文件，不属于工作区残留：感知扫描排除，
                        // 否则已删除文件被当残留 → 清理核验门永远驳回 → 清理死循环
                        // （仅工作区模式存在回收站；项目根模式下前缀非空，不适用回收站语义）
                        if 前缀.is_empty() && super::删除防护::是回收站条目(相对) {
                            continue;
                        }
                        结果.push(format!("{前缀}{}", 相对.to_string_lossy()));
                    }
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("glob 匹配错误: {e}"),
            }
        }
        if 结果.len() > 条目上限 {
            结果.truncate(条目上限);
        }
        if 结果.is_empty() {
            Ok("（无匹配）".to_string())
        } else {
            Ok(结果.join("\n"))
        }
    }

    fn 搜索内容(&self, 关键词: &str) -> Result<String> {
        // `~/` 前缀 → 搜项目根（返回路径带 `~/` 前缀）；否则搜工作区（现状）
        let (根, 前缀, 实际关键词) = if let Some(余) = 关键词.strip_prefix("~/") {
            let 项目根 = self
                .只读根
                .first()
                .ok_or_else(|| Error::Config("未配置只读根（~/ 前缀不可用）".into()))?;
            (项目根.clone(), "~/", 余.to_string())
        } else {
            (self.工作区.clone(), "", 关键词.to_string())
        };
        let mut 结果: Vec<String> = Vec::new();
        self.递归搜索(&根, 前缀, &根, &实际关键词, &mut 结果)?;
        if 结果.is_empty() {
            Ok("（无匹配）".to_string())
        } else {
            Ok(结果.join("\n"))
        }
    }

    fn 精确编辑(&self, 路径: &str, 旧: &str, 新: &str) -> Result<String> {
        if 旧.is_empty() {
            return Err(Error::Config("要替换的文本不能为空".into()));
        }
        let 目标 = self.解析路径(路径)?;
        let 原文 = std::fs::read_to_string(&目标).map_err(Error::Io)?;
        let 匹配次数 = 原文.matches(旧).count();
        if 匹配次数 == 0 {
            return Err(Error::Config(format!("未找到要替换的文本: {旧}")));
        }
        if 匹配次数 > 1 {
            return Err(Error::Config(format!(
                "要替换的文本出现 {匹配次数} 处，请提供更精确的上下文"
            )));
        }
        std::fs::copy(&目标, 备份路径(&目标)).map_err(Error::Io)?;
        let 新内容 = 原文.replacen(旧, 新, 1);
        std::fs::write(&目标, 新内容).map_err(Error::Io)?;
        Ok("替换成功（1 处）".to_string())
    }

    fn 删除文件(&self, 路径: &str) -> Result<String> {
        // 安全删除四步闭环（预审→名册→移入回收站→复核）在删除防护能力文件中内聚实现
        super::删除防护::安全删除(&self.工作区, 路径)
    }

}

impl 本地执行器 {
    /// 运行命令的实际实现（可指定超时）：白名单校验 → 工作区执行 → 收集输出 → 非零退出码报错。
    /// `运行命令` 用执行器默认超时，`运行命令_限时` 用调用方超时（编译/测试核验等长耗时场景）。
    /// 解析命令上下文：`~ ` 开头 → 项目根 cwd + 只读白名单；否则工作区 cwd + 现有白名单。
    fn 解析命令上下文(&self, 命令: &str) -> Result<(PathBuf, String, bool)> {
        let 修剪 = 命令.trim_start();
        if let Some(余) = 修剪.strip_prefix("~ ") {
            let 项目根 = self
                .只读根
                .first()
                .ok_or_else(|| Error::Config("未配置只读根（~ 命令前缀不可用）".into()))?;
            Ok((项目根.clone(), 余.to_string(), true))
        } else {
            Ok((self.工作区.clone(), 命令.to_string(), false))
        }
    }

    fn 运行命令_超时(&self, 命令: &str, 超时秒: u64) -> Result<String> {
        let (执行目录, 实际命令, 只读模式) = self.解析命令上下文(命令)?;
        if 命令不在白名单(&实际命令) {
            return Err(Error::危险命令(format!("命令不在白名单: {实际命令}")));
        }
        if 只读模式 && 命令不在只读白名单(&实际命令) {
            return Err(Error::危险命令(format!("命令不在只读白名单: {实际命令}")));
        }
        let mut 子进程 = Command::new("cmd")
            // raw_arg 原样传命令串：`args(["/C", 命令])` 会对含引号命令做 std 转义（内部 `"` → `\"`，
            // 整体加引号），cmd /C 遇多引号剥首尾后，cargo 经 CommandLineToArgvW 把 `\"` 解析成
            // 字面引号字符 → `--manifest-path "参数解析-府/Cargo.toml"` 变成带引号的路径 → 文件必不存在
            // （2026-09-13 实证：机器核验 100% 假失败）。raw_arg 让 cmd 收到的命令串与手敲一致。
            .raw_arg(format!("/C {实际命令}"))
            .current_dir(&执行目录)
            // 从源头关闭子进程彩色/光标输出（cargo 认 CARGO_TERM_COLOR，通用 CLI 认 NO_COLOR/CLICOLOR），
            // 避免 ANSI 转义序列进入天机流形成乱码；输出边界另有 剥终端转义 兜底（见 读流）。
            .env("CARGO_TERM_COLOR", "never")
            .env("NO_COLOR", "1")
            .env("CLICOLOR", "0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(Error::Io)?;

        let 标准输出流 = 子进程.stdout.take();
        let 标准错误流 = 子进程.stderr.take();
        let 上限 = self.最大输出字节;
        let 收集线程 = std::thread::spawn(move || 收集输出(标准输出流, 标准错误流, 上限));

        let 状态结果 = self.等待退出(&mut 子进程, 命令, 超时秒);
        let (标准输出, 标准错误) = 收集线程
            .join()
            .map_err(|_| Error::Other("输出收集线程异常".into()))?;
        let 状态 = 状态结果?;
        if !状态.success() {
            return Err(Error::Other(format!(
                "命令失败（退出码 {}）: {标准错误}{标准输出}",
                状态
            )));
        }
        Ok(标准输出)
    }
}

/// 生成备份路径：原文件名追加 ".bak" 后缀
fn 备份路径(目标: &Path) -> PathBuf {
    let mut 名 = 目标.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    名.push(".bak");
    目标.with_file_name(名)
}

/// 判断命令是否不在白名单（含转义符、脚本扩展名、白名单外命令名）
fn 命令不在白名单(命令: &str) -> bool {
    if 命令.contains('^') {
        return true;
    }
    for 子命令 in 拆命令段(命令) {
        let 命令名 = match 子命令.trim().split_whitespace().next() {
            Some(s) => s.trim_matches('"'),
            None => continue,
        };
        if 脚本扩展名.iter().any(|ext| 命令名.to_lowercase().ends_with(ext)) {
            return true;
        }
        let 基名 = 去路径去扩展名(命令名);
        if !允许命令白名单.contains(&基名.as_str()) {
            return true;
        }
    }
    false
}

/// 项目根 cwd 下的只读白名单判定：仅纯只读命令与 cargo 只读子命令；拒绝 build/test/run/add、
/// rustc、rustup、set 等一切写倾向命令。
fn 命令不在只读白名单(命令: &str) -> bool {
    if 命令.contains('^') {
        return true;
    }
    for 子命令 in 拆命令段(命令) {
        let 命令名 = match 子命令.trim().split_whitespace().next() {
            Some(s) => s.trim_matches('"'),
            None => continue,
        };
        if 脚本扩展名.iter().any(|ext| 命令名.to_lowercase().ends_with(ext)) {
            return true;
        }
        let 基名 = 去路径去扩展名(命令名);
        match 基名.as_str() {
            "cargo" => {
                if !cargo子命令只读(&子命令) {
                    return true;
                }
            }
            "cd" => {
                // cwd 已在项目根：禁止 .. 与绝对路径切换，防止逃逸到只读根之外
                let 参数们: Vec<&str> = 子命令.split_whitespace().skip(1).collect();
                if 子命令.contains("..")
                    || 参数们.iter().any(|p| Path::new(p.trim_matches('"')).is_absolute())
                {
                    return true;
                }
            }
            "dir" | "type" | "findstr" | "where" | "echo" | "cls" | "chcp" | "ping" => {}
            _ => return true,
        }
    }
    false
}

/// cargo 只读子命令：仅 metadata / tree / 版本与帮助旗标；其余（build/test/run/add/install…）拒绝。
fn cargo子命令只读(子命令: &str) -> bool {
    let 首词 = 子命令
        .split_whitespace()
        .nth(1)
        .map(|s| s.trim_matches('"'))
        .unwrap_or("");
    matches!(首词, "metadata" | "tree" | "-V" | "-h" | "--help")
        || 首词.starts_with("--version")
        || 首词 == "--list"
}

/// 按 cmd 语义把命令拆为独立子命令段。
///
/// 只认真正的命令分隔符：`&`、`&&`、`|`、`||`、`;`。
/// 重定向（`2>&1`、`1>&2`、`>file`、`>>file`、`<file` 等）不是分隔，
/// 其内部的 `&`（如 `2>&1`）不得触发拆分——否则会把 `1` 误判成一条独立命令而误拦。
fn 拆命令段(命令: &str) -> Vec<String> {
    let 字符集: Vec<char> = 命令.chars().collect();
    let mut 结果: Vec<String> = Vec::new();
    let mut 段 = String::new();
    let mut i = 0;
    while i < 字符集.len() {
        match 字符集[i] {
            // 引号内的分隔符/重定向均按字面处理
            '"' => {
                段.push('"');
                i += 1;
                while i < 字符集.len() && 字符集[i] != '"' {
                    段.push(字符集[i]);
                    i += 1;
                }
                if i < 字符集.len() {
                    段.push('"');
                    i += 1;
                }
            }
            '>' | '<' => {
                // 重定向开始：跳过其目标（文件 / & 数字合并符 / 引号文件名），整体不作为命令
                i += 1;
                while i < 字符集.len() {
                    let d = 字符集[i];
                    if d == '>' || d == '<' {
                        i += 1;
                        continue;
                    }
                    if d == '"' {
                        i += 1;
                        while i < 字符集.len() && 字符集[i] != '"' {
                            i += 1;
                        }
                        if i < 字符集.len() {
                            i += 1;
                        }
                        continue;
                    }
                    if d == '&' {
                        // 仅当 & 后紧跟数字才是 2>&1 式合并重定向；否则视为命令分隔，交给外层处理
                        if i + 1 < 字符集.len() && 字符集[i + 1].is_ascii_digit() {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    if d.is_whitespace() || d == '|' || d == ';' {
                        break;
                    }
                    i += 1;
                }
            }
            '&' | '|' | ';' => {
                if !段.trim().is_empty() {
                    结果.push(std::mem::take(&mut 段));
                } else {
                    段.clear();
                }
                // `&&` / `||` 双字符分隔符一次跳过
                if 字符集[i] != ';' && i + 1 < 字符集.len() && 字符集[i + 1] == 字符集[i] {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                段.push(字符集[i]);
                i += 1;
            }
        }
    }
    if !段.trim().is_empty() {
        结果.push(段);
    }
    结果
}

/// 从命令名中提取文件名部分并去掉 .exe 扩展名（如 `cargo.exe` → `cargo`）
fn 去路径去扩展名(命令名: &str) -> String {
    let 文件名 = match Path::new(命令名).file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => 命令名.to_string(),
    };
    if 文件名.to_lowercase().ends_with(".exe") {
        文件名[..文件名.len() - 4].to_string()
    } else {
        文件名
    }
}

/// 后台收集标准输出与标准错误（各自限长，避免 pipe 满阻塞子进程）
fn 收集输出(标准输出: Option<ChildStdout>, 标准错误: Option<ChildStderr>, 上限: u64) -> (String, String) {
    let 输出 = 标准输出.map(|流| 读流(流, 上限)).unwrap_or_default();
    let 错误 = 标准错误.map(|流| 读流(流, 上限)).unwrap_or_default();
    (输出, 错误)
}

/// 读取流内容并截断到上限字节，再剥离 ANSI 转义序列（尽力读取，读取失败仅返回已读部分）
fn 读流<R: Read>(流: R, 上限: u64) -> String {
    let mut 缓冲 = Vec::new();
    if let Err(失败) = 流.take(上限).read_to_end(&mut 缓冲) {
        tracing::warn!("读取子进程输出失败: {失败}");
    }
    let 原文 = String::from_utf8_lossy(&缓冲);
    super::输出净化::剥终端转义(&原文)
}
