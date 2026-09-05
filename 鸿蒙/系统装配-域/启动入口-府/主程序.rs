// 可执行入口：装配五行 + 起数据服务后挂起等待 Ctrl+C。
// 库 API（`启动`/`装配开发受理台`）由 `模块.rs` 对外暴露，桌面壳（hm-desktop）复用 `启动()`。
fn main() -> hm_error::Result<()> {
    hm_bootstrap::启动()?;

    // 挂起等待 Ctrl+C，保持数据服务线程存活；非交互环境注册失败仅告警
    if let Err(e) = ctrlc::set_handler(move || std::process::exit(0)) {
        tracing::warn!("注册 Ctrl+C 中断处理器失败（数据服务仍运行）: {e}");
    }
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}