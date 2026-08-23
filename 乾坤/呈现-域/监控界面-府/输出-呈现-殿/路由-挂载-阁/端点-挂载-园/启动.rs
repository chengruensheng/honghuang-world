//! 监控界面启动入口——编译为 `监控` 二进制，默认端口 8080。
//!
//! 启动行为：
//! - 监听 Ctrl+C / SIGTERM，触发 graceful shutdown
//! - 绑定失败立即 eprintln 报错并返回非零退码（便于任务计划 / 守护进程检测）
//! - 端口参数：第一个 CLI 参数，默认 8080

#[tokio::main]
async fn main() {
    let 端口: u16 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // graceful shutdown：监听 Ctrl+C / SIGTERM / 进程退出信号。
    // Windows 上 tokio::signal::ctrl_c 工作；Unix 上额外监听 SIGTERM。
    let 关 = tokio::signal::ctrl_c();
    #[cfg(unix)]
    let 关 = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut s = signal(SignalKind::terminate()).expect("install SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = s.recv() => {},
        }
    };

    tokio::select! {
        res = jiankong_fu::启动监控(端口) => {
            if let Err(e) = res {
                eprintln!("监控启动失败: {e}");
                std::process::exit(1);
            }
        }
        _ = 关 => {
            eprintln!("监控收到关闭信号，正在退出");
            std::process::exit(0);
        }
    }
}
