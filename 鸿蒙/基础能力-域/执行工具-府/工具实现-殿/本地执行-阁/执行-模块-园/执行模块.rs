use std::io::Read;
use std::path::{Component as 路径组件, Path, PathBuf};
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};
use hm_contract::Component;
use hm_error::{Error, Result};
use hm_execute_contract::执行器;

/// 默认命令超时（秒）
const 默认命令超时秒: u64 = 30;
/// 默认单次命令最大输出字节（64 KB，防止输出过大撑爆内存）
const 默认最大输出字节: u64 = 64 * 1024;
/// 危险命令关键字：命中即拒绝执行（防自主智能体误删、移动、杀进程、外泄或逃逸）
const 危险命令关键字: &[&str] = &[
    "del", "erase", "rd", "rmdir", "move", "ren", "rename",
    "format", "diskpart", "shutdown", "taskkill",
    "curl", "wget", "bitsadmin", "certutil", "powershell", "pwsh",
];

/// 本地执行器：在工作区沙箱内读写文件、运行命令。
///
/// 所有文件路径强制限定在工作区内：拒绝绝对路径与 `..` 越界；
/// 命令在工作区目录下执行，带超时与输出大小限制，避免越权与资源失控。
pub struct 本地执行器 {
    工作区: PathBuf,
    命令超时秒: u64,
    最大输出字节: u64,
}

impl 本地执行器 {
    /// 以指定工作区根构造本地执行器（命令超时 30 秒、最大输出 64 KB）
    pub fn new(工作区: impl Into<PathBuf>) -> Self {
        本地执行器 { 工作区: 工作区.into(), 命令超时秒: 默认命令超时秒, 最大输出字节: 默认最大输出字节 }
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
    fn 等待退出(&self, 子进程: &mut std::process::Child, 命令: &str) -> Result<std::process::ExitStatus> {
        let 开始 = Instant::now();
        loop {
            match 子进程.try_wait().map_err(Error::Io)? {
                Some(状态) => return Ok(状态),
                None => {
                    if 开始.elapsed() >= Duration::from_secs(self.命令超时秒) {
                        // 终止整个进程树（cmd 及其子进程），避免残留孤儿进程
                        let 进程号 = 子进程.id();
                        let _ = Command::new("taskkill")
                            .args(["/F", "/T", "/PID", &进程号.to_string()])
                            .output();
                        let _ = 子进程.wait();
                        return Err(Error::命令超时(命令.to_string()));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
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
        if 命中危险命令(命令) {
            return Err(Error::危险命令(命令.to_string()));
        }
        let mut 子进程 = Command::new("cmd")
            .args(["/C", 命令])
            .current_dir(&self.工作区)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(Error::Io)?;

        let 标准输出流 = 子进程.stdout.take();
        let 标准错误流 = 子进程.stderr.take();
        let 上限 = self.最大输出字节;
        let 收集线程 = std::thread::spawn(move || 收集输出(标准输出流, 标准错误流, 上限));

        let 状态结果 = self.等待退出(&mut 子进程, 命令);
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

/// 判断命令是否命中危险命令黑名单（按词边界匹配，避免 "model" 误伤 "del"）
fn 命中危险命令(命令: &str) -> bool {
    let 小写 = 命令.to_lowercase();
    危险命令关键字.iter().any(|词| {
        小写.split(|c: char| !c.is_alphanumeric()).any(|token| token == *词)
    })
}

/// 后台收集标准输出与标准错误（各自限长，避免 pipe 满阻塞子进程）
fn 收集输出(标准输出: Option<ChildStdout>, 标准错误: Option<ChildStderr>, 上限: u64) -> (String, String) {
    let 输出 = 标准输出.map(|流| 读流(流, 上限)).unwrap_or_default();
    let 错误 = 标准错误.map(|流| 读流(流, 上限)).unwrap_or_default();
    (输出, 错误)
}

/// 读取流内容并截断到上限字节（尽力读取，读取失败仅返回已读部分）
fn 读流<R: Read>(流: R, 上限: u64) -> String {
    let mut 缓冲 = Vec::new();
    let _ = 流.take(上限).read_to_end(&mut 缓冲);
    String::from_utf8_lossy(&缓冲).to_string()
}
