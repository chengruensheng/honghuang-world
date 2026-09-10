//! 预览入口：组装水镜自包含页面并落盘。
//! 落点一（预览）：target/水镜-预览/水镜.html —— 本地静态预览。
//! 落点二（接入·同源 A/B）：artifacts/agent-workspace/水镜.html ——
//!   与 门面.html 同目录新增文件（不覆盖任何现有文件），
//!   数据服务 8321 直接访问 /水镜.html 即与 /门面.html A/B 对比。
//! 运行：cargo run --manifest-path "乾坤/水镜映真-域/镜面承载-府/Cargo.toml" --bin 水镜-预览

fn main() {
    let mut 失败 = false;
    for 目录 in ["target/水镜-预览", "artifacts/agent-workspace"] {
        match qk_mirror::落盘页面(目录) {
            Ok(路径) => {
                let 绝对 = std::fs::canonicalize(&路径)
                    .unwrap_or_else(|_| std::path::PathBuf::from(&路径));
                println!("水镜已落盘：{}", 绝对.display());
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
