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
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "落地器文件锁失效"))?;
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
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "落地器文件锁失效"))?;
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

fn 打开文件(路径: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(路径)
}
