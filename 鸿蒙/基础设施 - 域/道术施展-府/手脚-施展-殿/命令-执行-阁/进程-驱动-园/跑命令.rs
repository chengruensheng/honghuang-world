//! 进程 - 驱动 - 园：驱动外部进程执行命令，支持超时强杀。
//!
//! 背景：原版同步 wait 等待子进程退出，无超时上限；命令可无限挂起整轮执行。
//! 现在「运行命令超时」用共享句柄槽 + 后台轮询线程实现：
//! 1) 子进程 stdout/stderr 走管道，主线程外另起线程读取，避免管道缓冲占满阻塞；
//! 2) Child 句柄放入共享槽，超时分支一次性 take + kill + wait 强杀；
//! 3) 后台等待线程轮询 try_wait（50ms 一次），完成后发回 ExitStatus；
//! 4) 主线程 recv_timeout 监听完成信号，超时则走强杀分支。

use rizhi_fu::{debug, error, warn};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// 默认超时上限（毫秒）：10 分钟。模型未指定时兜底，防任务无限挂起。
pub const 默认超时毫秒: u64 = 600_000;

/// 等待线程轮询 try_wait 的间隔：50ms，平衡响应速度与 CPU 占用。
const 轮询间隔: Duration = Duration::from_millis(50);

/// 命令执行结果。
#[derive(Clone, Debug, PartialEq)]
pub struct 命令结果 {
    pub 退出码: Option<i32>,
    pub 标准输出: String,
    pub 标准错误: String,
    pub 错误码: String,
}

/// 运行外部命令，使用默认超时上限（10 分钟）。
pub fn 运行命令(命令: &str, 参数们: &[&str], 工作目录: Option<&str>) -> Result<命令结果, String> {
    运行命令超时(命令, 参数们, 工作目录, 默认超时毫秒)
}

/// 运行外部命令，可选工作目录，指定超时（毫秒）。超时后强杀子进程并返回超时错误。
pub fn 运行命令超时(
    命令: &str,
    参数们: &[&str],
    工作目录: Option<&str>,
    超时毫秒: u64,
) -> Result<命令结果, String> {
    let mut 进程 = Command::new(命令);
    进程.args(参数们);
    if let Some(目录) = 工作目录 {
        进程.current_dir(目录);
    }
    进程.stdout(Stdio::piped());
    进程.stderr(Stdio::piped());

    let mut 子进程 = match 进程.spawn() {
        Ok(子) => 子,
        Err(错误) => {
            error!(命令, "运行命令失败：{错误}");
            return Err(format!("运行命令失败：{命令}：{错误}"));
        }
    };

    let stdout_handle = 子进程.stdout.take();
    let stderr_handle = 子进程.stderr.take();

    // 共享 Child 句柄槽：超时分支可一次性取出并 kill。
    let 句柄槽: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(Some(子进程)));
    let 句柄槽_w = Arc::clone(&句柄槽);

    // 后台等待线程：轮询 try_wait，完成后发回 ExitStatus。
    let (tx, rx) = channel::<Result<std::process::ExitStatus, String>>();
    let _等待线程 = thread::spawn(move || loop {
        let 取出 = match 句柄槽_w.lock() {
            Ok(mut 守卫) => 守卫.take(),
            Err(_) => {
                let _ = tx.send(Err("句柄槽中毒".to_string()));
                return;
            }
        };
        let mut 子进程 = match 取出 {
            Some(c) => c,
            // 已被超时分支取走（kill），等待循环退出。
            None => return,
        };
        match 子进程.try_wait() {
            Ok(Some(状态)) => {
                let _ = tx.send(Ok(状态));
                return;
            }
            Ok(None) => {
                let mut 守卫 = 句柄槽_w.lock().expect("句柄槽中毒");
                *守卫 = Some(子进程);
                drop(守卫);
                thread::sleep(轮询间隔);
            }
            Err(错误) => {
                let _ = tx.send(Err(format!("try_wait 失败：{错误}")));
                return;
            }
        }
    });

    // 后台线程读取 stdout/stderr（管道随子进程退出而关闭）。
    let stdout_线程 = stdout_handle.map(|mut out| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = out.read_to_end(&mut buf);
            buf
        })
    });
    let stderr_线程 = stderr_handle.map(|mut err| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = err.read_to_end(&mut buf);
            buf
        })
    });

    let 超时 = Duration::from_millis(超时毫秒);
    let 退出状态 = match rx.recv_timeout(超时) {
        Ok(Ok(状态)) => 状态,
        Ok(Err(错误)) => {
            error!(命令, "运行命令失败：{错误}");
            return Err(错误);
        }
        Err(RecvTimeoutError::Timeout) => {
            // 超时：一次性取出 child，kill 并 wait。
            let 取出 = 句柄槽.lock().ok().and_then(|mut 槽| 槽.take());
            if let Some(mut 子进程) = 取出 {
                let _ = 子进程.kill();
                let _ = 子进程.wait();
            }
            warn!(命令, 超时毫秒, "命令执行超时，已强杀子进程");
            return Err(format!(
                "命令执行超时被杀：{命令}（超时 {超时毫秒} 毫秒，子进程已强杀）"
            ));
        }
        Err(RecvTimeoutError::Disconnected) => {
            error!(命令, "等待线程失联");
            return Err(format!("命令执行线程失联：{命令}"));
        }
    };

    let stdout_bytes = stdout_线程
        .and_then(|t| t.join().ok())
        .unwrap_or_default();
    let stderr_bytes = stderr_线程
        .and_then(|t| t.join().ok())
        .unwrap_or_default();

    let 码 = 退出状态.code();
    if 码 != Some(0) {
        warn!(命令, 退出码 = ?码, "命令返回非零");
    } else {
        debug!(命令, "命令执行完成");
    }
    Ok(命令结果 {
        退出码: 码,
        标准输出: String::from_utf8_lossy(&stdout_bytes).to_string(),
        标准错误: String::from_utf8_lossy(&stderr_bytes).to_string(),
        错误码: match 码 {
            Some(0) | None => "OK".to_string(),
            Some(码) => format!("EXIT_{码}"),
        },
    })
}