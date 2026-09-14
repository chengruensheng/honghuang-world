//! 网络看门狗：跨线程限时执行（同步整体时限 + 流式块间隔时限）。
//!
//! 背景（实证 2026-09-14）：Windows 下 ureq 的 `.timeout()` 只可靠覆盖连接与响应头阶段，
//! 读响应体 / 读流阶段挂死时超时**不触发**——服务器建立连接后不发数据，
//! `into_string()` / `read_line()` 无限阻塞（实测挂死 36 分钟，配置的 300 秒超时形同虚设）。
//! 修法：把「请求+读」整体丢进子线程，主线程 `recv_timeout` 限时等待——
//! 超时立即判败转移下一供应商；孤儿线程自生自灭（进程退出即回收），绝不拖死主流程。
//!
//! 两条时限口径：
//! - 同步请求（非流式）：整体时限 = 供应商配置超时（原语义，仅让失效的机制重新生效）；
//! - 流式请求：块间隔时限 = 配置超时（与 60s 取大者，同原超时口径），
//!   首块等待与中流停顿共用同一时限——正常流式块间隔为秒级，只会斩「绝对挂死」，不误杀长生成。

use std::time::Duration;

use hm_content_contract::模型响应;
use hm_error::{Error, Result};

/// 限时执行同步任务：任务在子线程运行，主线程限时等待结果。
/// 超时或子线程异常退出 → 就地判败（错误消息带「看门狗」特征，便于日志甄别挂死类故障）。
pub fn 限时执行<T, 任务>(时限: Duration, 任务: 任务) -> Result<T>
where
    T: Send + 'static,
    任务: FnOnce() -> Result<T> + Send + 'static,
{
    let (发送, 接收) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // 接收端被放弃后 send 必然失败——孤儿线程的正常归宿，无需处理
        let _ = 发送.send(任务());
    });
    match 接收.recv_timeout(时限) {
        Ok(结果) => 结果,
        Err(_) => Err(Error::模型(format!(
            "模型响应超过限时 {} 秒（疑似网络挂死），看门狗强制判败",
            时限.as_secs().max(1)
        ))),
    }
}

/// 流式通道信令：增量块 / 最终结果
enum 流式信令 {
    块(String),
    完成(Result<模型响应>),
}

/// 流式限时执行：子线程跑「请求+读流」，块经 channel 转发、主线程回调（保持实时推送）；
/// 首块等待与块间隔超过时限 → 判败。已推块语义经 已发块 标记传回调用方（中流不重试保持不变）。
pub fn 流式限时执行<任务>(
    间隔时限: Duration,
    任务: 任务,
    已发块: &mut bool,
    on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), Error>,
) -> Result<模型响应>
where
    任务: FnOnce(
            &mut dyn FnMut(String) -> std::result::Result<(), Error>,
        ) -> Result<模型响应>
        + Send
        + 'static,
{
    let (发送, 接收) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let 转发 = &mut |块: String| -> std::result::Result<(), Error> {
            发送
                .send(流式信令::块(块))
                .map_err(|_| Error::模型("流式看门狗管道已断，中止读取".into()))
        };
        let 结果 = 任务(转发);
        let _ = 发送.send(流式信令::完成(结果));
    });
    loop {
        match 接收.recv_timeout(间隔时限) {
            Ok(流式信令::块(块)) => {
                on_chunk(块)?;
                *已发块 = true;
            }
            Ok(流式信令::完成(结果)) => return 结果,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                return Err(Error::模型(format!(
                    "流式响应超过 {} 秒无新数据（疑似网络挂死），看门狗强制判败",
                    间隔时限.as_secs().max(1)
                )));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(Error::模型("流式执行线程异常退出，看门狗判败".into()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;

    /// 快时限（测试用）：避免拖慢 CI
    const 快时限: Duration = Duration::from_millis(80);
    /// 慢任务耗时（模拟挂死）
    const 慢耗时: Duration = Duration::from_millis(600);

    #[test]
    fn 限时执行_按时完成放行() {
        let 开始 = Instant::now();
        let 结果 = 限时执行(快时限, || Ok::<_, Error>(42));
        assert_eq!(结果.expect("按时完成应返回结果"), 42);
        assert!(开始.elapsed() < 慢耗时, "不应等待慢时限");
    }

    #[test]
    fn 限时执行_超时判败带看门狗特征() {
        let 结果 = 限时执行(快时限, || {
            thread::sleep(慢耗时);
            Ok::<_, Error>(0)
        });
        let 错误 = format!("{}", 结果.unwrap_err());
        assert!(错误.contains("看门狗"), "错误消息应带看门狗特征：{错误}");
        assert!(错误.contains("挂死"), "错误消息应说明疑似挂死：{错误}");
    }

    #[test]
    fn 限时执行_子线程错误透传() {
        let 结果 = 限时执行(快时限, || -> Result<i32> { Err(Error::模型("上游错误".into())) });
        assert!(format!("{}", 结果.unwrap_err()).contains("上游错误"));
    }

    #[test]
    fn 流式限时执行_块转发实时且回调保持顺序() {
        let 收到 = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let 记录 = 收到.clone();
        let mut 已发 = false;
        let mut 回调 = |块: String| -> std::result::Result<(), Error> {
            record(&记录, 块);
            Ok(())
        };
        let 结果 = 流式限时执行(
            快时限,
            |发块| {
                发块("一".into())?;
                发块("二".into())?;
                Ok(模型响应 { 内容: Some("一二".into()), 工具调用: vec![], 思考: None })
            },
            &mut 已发,
            &mut 回调,
        );
        assert_eq!(结果.expect("流式应正常完成").内容.expect("应有正文"), "一二");
        assert_eq!(*记录.lock().expect("记录锁"), vec!["一".to_string(), "二".to_string()]);
        assert!(已发, "收到过块则已发块须置位");
    }

    fn record(记录: &Arc<std::sync::Mutex<Vec<String>>>, 块: String) {
        记录.lock().expect("记录锁").push(块);
    }

    #[test]
    fn 流式限时执行_首块等待超时判败() {
        let mut 已发 = false;
        let mut 回调 = |块: String| -> std::result::Result<(), Error> {
            let _ = 块;
            Ok(())
        };
        let 结果 = 流式限时执行(
            快时限,
            |发块| {
                thread::sleep(慢耗时); // 首块前就挂死
                发块("迟到的块".into())?;
                Ok(模型响应 { 内容: None, 工具调用: vec![], 思考: None })
            },
            &mut 已发,
            &mut 回调,
        );
        assert!(format!("{}", 结果.unwrap_err()).contains("看门狗"));
        assert!(!已发, "未收到任何块则已发块不得置位");
    }

    #[test]
    fn 流式限时执行_中流停顿超时判败且已发块置位() {
        let mut 已发 = false;
        let mut 回调 = |块: String| -> std::result::Result<(), Error> {
            let _ = 块;
            Ok(())
        };
        let 结果 = 流式限时执行(
            快时限,
            |发块| {
                发块("先到的块".into())?;
                thread::sleep(慢耗时); // 中流挂死，后续块与完成信号永不到达
                发块("永不送达".into())?;
                Ok(模型响应 { 内容: None, 工具调用: vec![], 思考: None })
            },
            &mut 已发,
            &mut 回调,
        );
        assert!(format!("{}", 结果.unwrap_err()).contains("无新数据"));
        assert!(已发, "中流判败时已发块须置位（调用方据此不重试）");
    }

    #[test]
    fn 流式限时执行_回调失败中止且不再收块() {
        let mut 已发 = false;
        let mut 回调 = |块: String| -> std::result::Result<(), Error> {
            let _ = 块;
            Err(Error::模型("下游断开".into())) // 首块即转发失败
        };
        let 结果 = 流式限时执行(
            快时限,
            |发块| {
                发块("一".into())?;
                发块("二".into())?;
                Ok(模型响应 { 内容: None, 工具调用: vec![], 思考: None })
            },
            &mut 已发,
            &mut 回调,
        );
        assert!(format!("{}", 结果.unwrap_err()).contains("下游断开"));
    }
}
