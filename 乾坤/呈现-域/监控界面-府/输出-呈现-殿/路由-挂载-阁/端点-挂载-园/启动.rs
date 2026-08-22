//! 监控界面启动入口——编译为 `监控` 二进制，默认端口 8080。

#[tokio::main]
async fn main() {
    let 端口: u16 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    jiankong_fu::启动监控(端口).await;
}
