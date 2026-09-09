//! 预览入口：组装自包含界面页面并落盘。
//! 落点一（预览）：target/界面-预览/门面.html —— 供本地预览服务（零依赖静态服务）查看。
//! 落点二（接入）：artifacts/agent-workspace/门面.html —— 供数据服务同源托管，
//!   使门面页 fetch("/api/dev/chat") 等接口时与后端同源。
//! 运行：cargo run -p qk-web --bin 界面-预览（工作区根执行）

fn main() {
    let mut 失败 = false;
    for 目录 in ["target/界面-预览", "artifacts/agent-workspace"] {
        match qk_web::落盘页面(目录) {
            Ok(路径) => {
                let 绝对 = std::fs::canonicalize(&路径)
                    .unwrap_or_else(|_| std::path::PathBuf::from(&路径));
                println!("界面已落盘：{}", 绝对.display());
            }
            Err(e) => {
                eprintln!("落盘到 {目录} 失败：{e}");
                失败 = true;
            }
        }
    }
    if 失败 {
        std::process::exit(1);
    }
}
