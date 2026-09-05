// 桌面壳入口：内嵌数据服务（复用 hm-bootstrap::启动）→ 等待端口就绪 → 开 Tauri 窗口加载前端。
// 前端零改动：窗口直接指向内嵌 axum 的 localhost 端口，Web 与桌面共用同一套页面与接口。
use std::path::Path;
use tauri::Manager;

#[path = "前端-监视-园/模块.rs"]
mod 前端监视园;

fn main() {
    let 配置 = hm_config::default_config();
    let 端口 = 配置.http.port;
    let 热更新 = 配置.http.hot_reload;
    let 静态目录 = 配置.http.static_dir.clone();

    // 把工作目录切到能解析前端静态目录的根目录，避免相对路径依赖启动方式（tauri dev 的 cwd 是 src-tauri）
    切换工作目录到静态目录根(&静态目录);

    if let Err(e) = hm_bootstrap::启动() {
        tracing::error!("数据服务启动失败: {e}");
        std::process::exit(1);
    }
    if !等待端口就绪(端口) {
        tracing::error!("数据服务端口 {端口} 未在超时内就绪，无法加载前端");
        std::process::exit(1);
    }

    tauri::Builder::default()
        .setup(move |app| {
            if 热更新 {
                match app.get_webview_window("main") {
                    Some(窗口) => {
                        std::thread::spawn(move || {
                            if let Err(e) = 前端监视园::启动前端监视(静态目录, 窗口) {
                                tracing::warn!("前端热更新启动失败: {e}");
                            }
                        });
                    }
                    None => tracing::warn!("前端热更新已开启但未找到主窗口，跳过"),
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("桌面壳运行失败");
}

/// 轮询等待本地端口就绪，避免窗口早于数据服务打开而显示空白
fn 等待端口就绪(端口: u16) -> bool {
    let 地址 = format!("127.0.0.1:{端口}");
    for _ in 0..50 {
        if std::net::TcpStream::connect(&地址).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    false
}

/// 把工作目录切换到能解析前端静态目录的根目录（向上查找，兼容任意启动方式）
fn 切换工作目录到静态目录根(静态目录: &str) {
    if Path::new(静态目录).is_absolute() {
        return;
    }
    let Ok(当前) = std::env::current_dir() else {
        return;
    };
    let mut 目录 = 当前;
    loop {
        if 目录.join(静态目录).exists() {
            if let Err(e) = std::env::set_current_dir(&目录) {
                tracing::warn!("切换工作目录到 {目录:?} 失败: {e}");
            }
            return;
        }
        match 目录.parent() {
            Some(父) => 目录 = 父.to_path_buf(),
            None => return,
        }
    }
}
