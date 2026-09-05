#[path = "装配流程-殿/模块.rs"]
mod 装配流程_殿;

use 装配流程_殿::*;

fn main() -> hm_error::Result<()> {
    启动()?;

    // 挂起等待 Ctrl+C，保持数据服务线程存活；非交互环境注册失败仅告警
    if let Err(e) = ctrlc::set_handler(move || std::process::exit(0)) {
        tracing::warn!("注册 Ctrl+C 中断处理器失败（数据服务仍运行）: {e}");
    }
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
