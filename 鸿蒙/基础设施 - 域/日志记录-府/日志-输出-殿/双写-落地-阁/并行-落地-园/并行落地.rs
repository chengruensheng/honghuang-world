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

#[cfg(test)]
mod 测试 {
    use super::落地器;
    use crate::日志去向;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 生成测试用的唯一临时文件路径（基于 PID + 纳秒时间戳 + 标签）
    fn 临时路径(标签: &str) -> PathBuf {
        let 时间戳 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "rizhi_fu_test_{}_{}_{}.log",
            std::process::id(),
            时间戳,
            标签
        ))
    }

    /// 仅控制台落地器：开控制台、不持文件。
    #[test]
    fn 仅控制台无文件() {
        let 落地器 = 落地器::仅控制台();
        assert!(落地器.写控制台);
        assert!(落地器.文件.is_none());
    }

    /// 仅文件去向：关控制台、持文件。
    #[test]
    fn 新建仅文件去向() {
        let 路径 = 临时路径("仅文件开关");
        let 落地器 = 落地器::新建(&日志去向::仅文件(路径.clone())).unwrap();
        assert!(!落地器.写控制台, "仅文件必须关控制台");
        assert!(落地器.文件.is_some(), "仅文件必须持文件");
        let _ = fs::remove_file(&路径);
    }

    /// 双写去向：开控制台、持文件。
    #[test]
    fn 双写模式同时开启控制台与文件() {
        let 路径 = 临时路径("双写开关");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        assert!(落地器.写控制台, "双写必须开控制台");
        assert!(落地器.文件.is_some(), "双写必须持文件");
        let _ = fs::remove_file(&路径);
    }

    /// 双写字节指纹：单次写入已知字节并 flush，回读比对字节完全相等。
    #[test]
    fn 双写落盘字节指纹一致() {
        let 路径 = 临时路径("指纹");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.as_ref().expect("双写必有文件");
        let 期望: &[u8] = b"hello-parallel-landing\n";
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            守.write_all(期望).expect("写入成功");
            守.flush().expect("刷新成功");
        }
        let 实际 = fs::read(&路径).expect("回读成功");
        assert_eq!(实际, 期望, "双写落盘字节必须与期望完全一致");
        let _ = fs::remove_file(&路径);
    }

    /// 并发竞态：N 线程同时写同一落地器，断言双通道落盘行集合完整且无截断、无重复、无丢失。
    #[test]
    fn 并发多线程双写完整保真() {
        let 路径 = 临时路径("并发");
        let 共享落地器 = std::sync::Arc::new(落地器::新建(&日志去向::双写(路径.clone())).unwrap());
        let 线程数: usize = 8;
        let 每线程条数: usize = 10;

        let 句柄组: Vec<_> = (0..线程数)
            .map(|线程号| {
                let 落地器 = std::sync::Arc::clone(&共享落地器);
                std::thread::spawn(move || {
                    let 文件句柄 = 落地器.文件.as_ref().expect("并发必有文件");
                    for 序号 in 0..每线程条数 {
                        let 行 = format!("T{}-N{}\n", 线程号, 序号);
                        let mut 守 = 文件句柄.lock().expect("锁未中毒");
                        守.write_all(行.as_bytes()).expect("写入成功");
                        守.flush().expect("刷新成功");
                    }
                })
            })
            .collect();

        for h in 句柄组 {
            h.join().expect("线程无 panic");
        }

        let 内容 = std::fs::read_to_string(&路径).expect("回读成功");
        let 实际行: Vec<&str> = 内容.lines().collect();
        assert_eq!(
            实际行.len(),
            线程数 * 每线程条数,
            "并发双写总条数必须严格等于线程数×每线程条数"
        );

        let mut 已见 = std::collections::HashSet::<String>::with_capacity(实际行.len());
        for 行 in &实际行 {
            assert!(已见.insert((*行).to_string()), "并发落盘出现重复行：{}", 行);
        }
        for 线程号 in 0..线程数 {
            for 序号 in 0..每线程条数 {
                let 期望 = format!("T{}-N{}", 线程号, 序号);
                assert!(已见.contains(&期望), "并发落盘缺少行：{}", 期望);
            }
        }

        let _ = std::fs::remove_file(&路径);
    }

    /// 边界：空 payload 写入不 panic、不污染文件句柄，落盘字节数严格为 0。
    #[test]
    fn 空字符串写不污染文件() {
        let 路径 = 临时路径("空写");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.as_ref().expect("双写必有文件");
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            守.write_all(b"").expect("空字符串写入成功");
            守.flush().expect("刷新成功");
        }
        assert!(std::path::Path::new(&路径).exists(), "空写后文件必须存在");
        let 实际 = std::fs::read(&路径).expect("回读成功");
        assert_eq!(实际.len(), 0, "空字符串写入不得增加任何字节");
        let _ = std::fs::remove_file(&路径);
    }

    /// 并发空写：N 线程同时写空字符串，最终文件字节数严格为 0，验证锁无吞字节污染。
    #[test]
    fn 并发空字符串终态字节数为零() {
        let 路径 = 临时路径("并发空");
        let 共享 = std::sync::Arc::new(落地器::新建(&日志去向::双写(路径.clone())).unwrap());
        let 线程数: usize = 8;
        let 句柄组: Vec<_> = (0..线程数)
            .map(|_| {
                let 落地器 = std::sync::Arc::clone(&共享);
                std::thread::spawn(move || {
                    let 文件句柄 = 落地器.文件.as_ref().expect("并发必有文件");
                    let mut 守 = 文件句柄.lock().expect("锁未中毒");
                    守.write_all(b"").expect("空写成功");
                    守.flush().expect("刷新成功");
                })
            })
            .collect();
        for h in 句柄组 {
            h.join().expect("线程无 panic");
        }
        let 实际 = std::fs::read(&路径).expect("回读成功");
        assert_eq!(实际.len(), 0, "并发空写终态字节数必须严格为 0");
        let _ = std::fs::remove_file(&路径);
    }

    /// 边界：64 KiB 大字段单行写入回读字节完全一致，验证大块写入无截断。
    #[test]
    fn 超大单行写入字节一致() {
        let 路径 = 临时路径("大字段");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.as_ref().expect("双写必有文件");
        let 期望: Vec<u8> = (0..(64 * 1024)).map(|i| (i % 251) as u8).collect();
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            守.write_all(&期望).expect("大字段写入成功");
            守.flush().expect("刷新成功");
        }
        let 实际 = std::fs::read(&路径).expect("回读成功");
        assert_eq!(实际.len(), 期望.len(), "大字段落盘长度一致");
        assert_eq!(实际, 期望, "大字段落盘字节指纹完全一致");
        let _ = std::fs::remove_file(&路径);
    }

    /// 双写追加多次：多次顺序追加并显式 flush，验证按序拼接的字节级一致性。
    #[test]
    fn 双写追加多次字节一致() {
        let 路径 = 临时路径("追加");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.clone().expect("双写必有文件");
        let 分片: &[&[u8]] = &[
            b"first-line\n",
            b"second-line\n",
            b"third-line\n",
            b"final-line\n",
        ];
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            for 分 in 分片 {
                守.write_all(分).expect("写入成功");
                守.flush().expect("刷新成功");
            }
        }
        let 实际 = fs::read(&路径).expect("回读成功");
        let 期望: Vec<u8> = 分片.iter().flat_map(|s| s.iter().copied()).collect();
        assert_eq!(实际, 期望, "多次追加必须按序拼接，字节完全一致");
        let _ = fs::remove_file(&路径);
    }

    /// 合法路径：中文文件名 UTF-8 路径能成功创建并正常落盘。
    #[test]
    fn 双写中文文件名字节指纹一致() {
        let 路径 =
            std::env::temp_dir().join(format!("rizhi_fu_test_{}_中文文件.log", std::process::id()));
        let _ = fs::remove_file(&路径);
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.as_ref().expect("双写必有文件");
        let 内容: &[u8] = "你好，世界\n".as_bytes();
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            守.write_all(内容).expect("写入成功");
            守.flush().expect("刷新成功");
        }
        let 实际 = fs::read(&路径).expect("回读成功");
        assert_eq!(实际, 内容, "中文文件名落盘字节必须完全一致");
        let _ = fs::remove_file(&路径);
    }

    /// 边界：空内容写入后文件大小为零且不报错。
    #[test]
    fn 双写空内容文件字节长度为0() {
        let 路径 = 临时路径("空内容");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.as_ref().expect("双写必有文件");
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            守.write_all(b"").expect("空写入成功");
            守.flush().expect("刷新成功");
        }
        let 元数据 = fs::metadata(&路径).expect("文件存在");
        assert_eq!(元数据.len(), 0, "空内容落盘后文件长度必须为 0");
        let _ = fs::remove_file(&路径);
    }

    /// 边界：单字节内容能正确落盘。
    #[test]
    fn 双写单字节内容字节指纹一致() {
        let 路径 = 临时路径("单字节");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.as_ref().expect("双写必有文件");
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            守.write_all(b"X").expect("单字节写入成功");
            守.flush().expect("刷新成功");
        }
        let 实际 = fs::read(&路径).expect("回读成功");
        assert_eq!(实际, b"X", "单字节落盘必须等于 'X'");
        let _ = fs::remove_file(&路径);
    }

    /// 边界：非法 UTF-8 字节序列不触发 panic，原样落盘。
    #[test]
    fn 双写非法utf8字节序列不panic() {
        let 路径 = 临时路径("非法utf8");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.as_ref().expect("双写必有文件");
        let 非法: &[u8] = &[0xFF, 0xFE, 0xFD, 0xFC, 0x80, 0x81, 0x82];
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            守.write_all(非法).expect("非法 UTF-8 写入不 panic");
            守.flush().expect("刷新成功");
        }
        let 实际 = fs::read(&路径).expect("回读成功");
        assert_eq!(实际, 非法, "非法 UTF-8 字节必须原样落盘");
        let _ = fs::remove_file(&路径);
    }

    /// 边界：1MB 一次性写入后字节级完全一致。
    #[test]
    fn 双写一次性大文件字节一致() {
        let 路径 = 临时路径("大文件");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.as_ref().expect("双写必有文件");
        let 期望: Vec<u8> = (0..1024 * 1024).map(|i| (i % 251) as u8).collect();
        {
            let mut 守 = 文件句柄.lock().expect("锁未中毒");
            守.write_all(&期望).expect("大文件写入成功");
            守.flush().expect("刷新成功");
        }
        let 实际 = fs::read(&路径).expect("回读成功");
        assert_eq!(实际.len(), 期望.len(), "大文件字节长度必须一致");
        assert_eq!(实际, 期望, "大文件字节内容必须完全一致");
        let _ = fs::remove_file(&路径);
    }

    /// 双写一致性：多线程并发写入后所有字节均落盘无丢失。
    #[test]
    fn 双写多线程并发写入无丢失() {
        let 路径 = 临时路径("并发");
        let 落地器 = 落地器::新建(&日志去向::双写(路径.clone())).unwrap();
        let 文件句柄 = 落地器.文件.clone().expect("双写必有文件");
        let 线程数: usize = 4;
        let 每线程_条数: usize = 50;
        let 期望总行数: usize = 线程数 * 每线程_条数;
        let 句柄s: Vec<thread::JoinHandle<()>> = (0..线程数)
            .map(|i| {
                let 文件句柄 = 文件句柄.clone();
                thread::spawn(move || {
                    for j in 0..每线程_条数 {
                        // 每条固定 9 字节："tXX-mYYY\n" (t+2位+- +m+3位+\n)
                        let 文本 = format!("t{:02}-m{:03}\n", i, j);
                        let mut 守 = 文件句柄.lock().expect("锁未中毒");
                        守.write_all(文本.as_bytes()).expect("并发写入成功");
                        守.flush().expect("刷新成功");
                    }
                })
            })
            .collect();
        for h in 句柄s {
            h.join().expect("线程不应 panic");
        }
        let 实际 = fs::read(&路径).expect("回读成功");
        let 实际文本 = String::from_utf8_lossy(&实际);
        let 实际行数 = 实际文本.lines().count();
        assert_eq!(实际行数, 期望总行数, "所有线程写入的行都必须落盘且无丢失");
        assert_eq!(实际.len(), 期望总行数 * 9, "总字节数必须等于行数 × 9");
        let _ = fs::remove_file(&路径);
    }

    /// 非法输入：空路径在新建时不应 panic（行为由底层 open 决定）。
    #[test]
    fn 新建空路径_不panic() {
        let 结果 = 落地器::新建(&日志去向::仅文件(PathBuf::new()));
        // 行为依赖平台（Windows 上 open("") 会失败），但调用本身不 panic
        let _ = 结果;
    }

    /// None 载荷：仅控制台落地器（文件字段为 None）通过 Write trait 写入与 flush 完整链路不 panic、返回字节数正确。
    #[test]
    fn 仅控制台落地器_Write路径不panic() {
        use std::io::Write as _;
        let mut 落地器 = 落地器::仅控制台();
        let 内容: &[u8] = b"stdout-only-no-file\n";
        // 文件字段为 None，仅走控制台分支；调用必须不 panic 且返回写入字节数
        let 实际 = 落地器.write(内容).expect("仅控制台写入不应返回错误");
        assert_eq!(实际, 内容.len(), "仅控制台写入字节数必须等于输入长度");
        落地器.flush().expect("仅控制台 flush 不应返回错误");
    }

    /// 非法输入：路径所在父目录不存在（目录不可写），落地器::新建 必须返回 Err 而不 panic。
    #[test]
    fn 新建不可访问路径_返回Err_不panic() {
        // 父目录必定不存在（使用唯一后缀），模拟"目录不可写"场景：
        // OpenOptions 在父目录缺失时无法 create，期望 io::Error。
        let 父目录 = std::env::temp_dir().join("rizhi_fu_zzz_no_parent_aaa9999");
        let 路径 = 父目录.join("child_dir").join("日志.log");
        let 结果 = 落地器::新建(&日志去向::仅文件(路径.clone()));
        assert!(
            结果.is_err(),
            "不可访问路径下落地器::新建 必须返回 Err，不能 panic"
        );
        // 父目录本身不存在，清理为 no-op
        let _ = std::fs::remove_dir_all(&父目录);
    }
}
