// 并行落地 —— 控制台与文件并行落地
#![allow(non_snake_case)]

use crate::日志去向;

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::writer::MakeWriter;

/// 落地器：按去向写控制台、写文件，或双写（互不阻塞）
#[derive(Clone)]
pub struct 落地器 {
    pub 写控制台: bool,
    pub 文件: Option<Arc<Mutex<File>>>,
}

impl 落地器 {
    /// 仅控制台（无文件，不落盘）
    pub fn 仅控制台() -> Self {
        落地器 {
            写控制台: true,
            文件: None,
        }
    }

    /// 按去向新建落地器；文件打不开时返回错误，由调用方兜底
    pub fn 新建(去向: &日志去向) -> io::Result<Self> {
        match 去向 {
            日志去向::仅控制台 => Ok(落地器::仅控制台()),
            日志去向::仅文件(路径) => Ok(落地器 {
                写控制台: false,
                文件: Some(Arc::new(Mutex::new(打开文件(路径)?))),
            }),
            日志去向::双写(路径) => Ok(落地器 {
                写控制台: true,
                文件: Some(Arc::new(Mutex::new(打开文件(路径)?))),
            }),
        }
    }
}

impl Write for 落地器 {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.写控制台 {
            io::stdout().write(buf)?;
        }
        if let Some(文件) = &self.文件 {
            let mut 锁 = 文件
                .lock()
                .map_err(|_| io::Error::other("落地器文件锁失效"))?;
            锁.write(buf)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.写控制台 {
            io::stdout().flush()?;
        }
        if let Some(文件) = &self.文件 {
            let mut 锁 = 文件
                .lock()
                .map_err(|_| io::Error::other("落地器文件锁失效"))?;
            锁.flush()?;
        }
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for 落地器 {
    type Writer = 落地器;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// 日志轮转大小：单文件超过 5MB 触发轮转（生产化 3.2）。
const 日志轮转大小: u64 = 5 * 1024 * 1024;
/// 日志保留份数：轮转后保留 当前 + .1 ~ .N。
const 日志保留份数: u32 = 5;

fn 打开文件(路径: &Path) -> io::Result<File> {
    轮转日志(路径)?;
    OpenOptions::new().create(true).append(true).open(路径)
}

/// 按大小轮转：现有文件超阈值 → 顺延为 .1/.2/…（最旧删除），再开新文件。
/// 只在句柄打开时检查（重启/重建落地器时生效）；运行中的句柄由下次打开轮转。
fn 轮转日志(路径: &Path) -> io::Result<()> {
    let 元 = match std::fs::metadata(路径) {
        Ok(元) => 元,
        Err(_) => return Ok(()),
    };
    if 元.len() < 日志轮转大小 {
        return Ok(());
    }
    let 名 = 路径.display();
    let _ = std::fs::remove_file(format!("{名}.{日志保留份数}"));
    for 序 in (1..日志保留份数).rev() {
        let 旧 = format!("{名}.{序}");
        let 新 = format!("{名}.{}", 序 + 1);
        let _ = std::fs::rename(&旧, &新);
    }
    let _ = std::fs::rename(路径, format!("{名}.1"));
    Ok(())
}
