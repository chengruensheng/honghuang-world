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

/// 本地执行器：在工作区沙箱内读写文件、运行命令。
///
/// 所有文件路径强制限定在工作区内：拒绝绝对路径与 `..` 越界；
/// 命令在工作区目录下执行，带超时与输出大小限制，避免越权与资源失控。
pub struct 本地执行器 {
    工作区: PathBuf,
    命令超时秒: u64,
    最大输出字节: u64,
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
        本地执行器 { 工作区: 规范化工作区(路径), 命令超时秒: 默认命令超时秒, 最大输出字节: 默认最大输出字节 }
    }

    /// 以指定工作区根与显式上限构造本地执行器（从配置注入超时与输出上限，替代写死默认值）
    pub fn new_with_limits(工作区: impl Into<PathBuf>, 命令超时秒: u64, 最大输出字节: u64) -> Self {
        let 路径 = 工作区.into();
        if let Err(e) = std::fs::create_dir_all(&路径) {
            eprintln!("警告：无法创建工作区目录 {}: {}", 路径.display(), e);
        }
        本地执行器 { 工作区: 规范化工作区(路径), 命令超时秒, 最大输出字节 }
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

    /// 将相对路径解析到工作区内；拒绝绝对路径与 `..` 越界
    fn 解析路径(&self, 路径: &str) -> Result<PathBuf> {
        let 相对 = Path::new(路径);
        if 相对.is_absolute()
            || 相对.components().any(|c| matches!(c, 路径组件::ParentDir))
        {
            return Err(Error::Config(format!("非法路径（越出工作区）: {路径}")));
        }
        Ok(self.工作区.join(相对))
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

    /// 递归遍历目录，逐文件按行匹配关键词，命中写入「相对路径:行号:内容」
    fn 递归搜索(&self, 目录: &Path, 关键词: &str, 结果: &mut Vec<String>) -> Result<()> {
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
                    self.递归搜索(&路径, 关键词, 结果)?;
                }
            } else if 路径.is_file() {
                self.搜索单文件(&路径, 关键词, 结果);
            }
        }
        Ok(())
    }

    /// 读单个文本文件并按行匹配关键词；跳过超大文件与二进制（含 NUL 字节）
    fn 搜索单文件(&self, 路径: &Path, 关键词: &str, 结果: &mut Vec<String>) {
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
                if let Ok(相对) = 路径.strip_prefix(&self.工作区) {
                    结果.push(format!("{}:{}:{}", 相对.to_string_lossy(), 序号 + 1, 行));
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

impl Component for 本地执行器 {
    fn name(&self) -> &'static str { "本地执行器" }
}

impl 执行器 for 本地执行器 {
    fn 读文件(&self, 路径: &str) -> Result<String> {
        let 目标 = self.解析路径(路径)?;
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
        let 目标 = self.解析路径(路径)?;
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
        let 绝对模式 = self.工作区.join(模式);
        let 模式字符串 = 绝对模式.to_string_lossy().into_owned();
        let 匹配 = glob::glob(&模式字符串)
            .map_err(|e| Error::Config(format!("无效 glob 模式 {模式}: {e}")))?;
        let mut 结果: Vec<String> = Vec::new();
        for 项 in 匹配 {
            match 项 {
                Ok(路径) if 路径.is_file() => {
                    if let Ok(相对) = 路径.strip_prefix(&self.工作区) {
                        // 回收站内是已删除文件，不属于工作区残留：感知扫描排除，
                        // 否则已删除文件被当残留 → 清理核验门永远驳回 → 清理死循环
                        if super::删除防护::是回收站条目(相对) {
                            continue;
                        }
                        结果.push(相对.to_string_lossy().into_owned());
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
        let mut 结果: Vec<String> = Vec::new();
        self.递归搜索(&self.工作区.clone(), 关键词, &mut 结果)?;
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
    fn 运行命令_超时(&self, 命令: &str, 超时秒: u64) -> Result<String> {
        if 命令不在白名单(命令) {
            return Err(Error::危险命令(format!("命令不在白名单: {命令}")));
        }
        let mut 子进程 = Command::new("cmd")
            // raw_arg 原样传命令串：`args(["/C", 命令])` 会对含引号命令做 std 转义（内部 `"` → `\"`，
            // 整体加引号），cmd /C 遇多引号剥首尾后，cargo 经 CommandLineToArgvW 把 `\"` 解析成
            // 字面引号字符 → `--manifest-path "参数解析-府/Cargo.toml"` 变成带引号的路径 → 文件必不存在
            // （2026-09-13 实证：机器核验 100% 假失败）。raw_arg 让 cmd 收到的命令串与手敲一致。
            .raw_arg(format!("/C {命令}"))
            .current_dir(&self.工作区)
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
