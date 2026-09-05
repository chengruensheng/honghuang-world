//! 前端热更新监视：监视前端静态目录，文件变化时自动刷新窗口。
//! 仅开发期启用（配置 http.hot_reload = true），生产默认关闭。

use std::path::Path;
use std::sync::mpsc::channel;
use notify::{recommended_watcher, RecursiveMode, Watcher};
use tauri::WebviewWindow;

/// 启动前端目录监视（阻塞当前线程直到监视结束，应在独立线程调用）
pub fn 启动前端监视(目录: String, 窗口: WebviewWindow) -> Result<(), String> {
    let (发送, 接收) = channel();

    let mut 监视器 = recommended_watcher(move |结果| {
        // 接收端关闭（本线程退出）时发送失败，属正常关闭信号，非业务错误
        let _ = 发送.send(结果);
    })
    .map_err(|e| format!("创建前端监视器失败: {e}"))?;

    监视器
        .watch(Path::new(&目录), RecursiveMode::Recursive)
        .map_err(|e| format!("监视前端目录 {目录} 失败: {e}"))?;

    tracing::info!("前端热更新已开启，监视目录: {目录}");

    for 结果 in 接收 {
        match 结果 {
            Ok(_) => {
                if let Err(e) = 窗口.eval("window.location.reload()") {
                    tracing::warn!("前端热更新刷新窗口失败: {e}");
                }
            }
            Err(e) => tracing::warn!("前端热更新监视事件错误: {e}"),
        }
    }
    Ok(())
}
