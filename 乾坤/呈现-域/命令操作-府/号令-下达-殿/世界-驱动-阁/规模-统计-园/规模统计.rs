//! 规模统计：递归扫描项目目录，汇总四项规模指标。
//! 纯标准库实现，用 std::fs::read_dir 递归遍历，不依赖 walkdir 或任何外部 crate。
use std::fs;
use std::path::Path;

/// 规模指标：四项核心数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 规模指标 {
    pub 源码文件数: usize,
    pub 源码总行数: usize,
    pub 证道测试文件数: usize,
    pub crate数: usize,
}

/// 呈现项目规模：扫描工作区根，返回可读文本。
pub fn 呈现项目规模() -> String {
    let 根 = crate::工作区根();
    let 指标 = 统计规模(&根);
    format!(
        "项目规模指标\n  源码 rs 文件数：{}\n  源码总行数：{}\n  证道测试文件数：{}\n  crate 数：{}",
        指标.源码文件数, 指标.源码总行数, 指标.证道测试文件数, 指标.crate数,
    )
}

/// 统计规模：递归扫描目录，汇总四项指标。
fn 统计规模(根: &Path) -> 规模指标 {
    let mut 源码文件数 = 0usize;
    let mut 源码总行数 = 0usize;
    let mut 证道测试文件数 = 0usize;
    let mut crate数 = 0usize;
    递归扫描(
        根,
        &mut 源码文件数,
        &mut 源码总行数,
        &mut 证道测试文件数,
        &mut crate数,
    );
    规模指标 {
        源码文件数,
        源码总行数,
        证道测试文件数,
        crate数,
    }
}

/// 跳过这些目录（构建产物 / 版本控制 / 内部状态 / 临时文件）。
fn 应跳过目录(名: &str) -> bool {
    matches!(
        名,
        ".git" | "target" | ".上下文" | "node_modules" | ".codegraph" | "临时文件夹"
    )
}

/// 递归扫描目录，统计 .rs 文件数、行数、证道测试文件数、Cargo.toml 数。
fn 递归扫描(
    当前: &Path,
    源码文件数: &mut usize,
    源码总行数: &mut usize,
    证道测试文件数: &mut usize,
    crate数: &mut usize,
) {
    let 条目 = match fs::read_dir(当前) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in 条目.flatten() {
        let 路径 = entry.path();
        let 名 = entry.file_name();
        let 名串 = 名.to_string_lossy();

        if 路径.is_dir() {
            if 应跳过目录(&名串) {
                continue;
            }
            递归扫描(&路径, 源码文件数, 源码总行数, 证道测试文件数, crate数);
        } else if 路径.is_file() {
            if 名串 == "Cargo.toml" {
                *crate数 += 1;
            }
            if 名串.ends_with(".rs") {
                *源码文件数 += 1;
                if let Ok(内容) = fs::read_to_string(&路径) {
                    *源码总行数 += 内容.lines().count();
                }
                if 是证道测试文件(&路径) {
                    *证道测试文件数 += 1;
                }
            }
        }
    }
}

/// 判断是否是证道测试文件：路径含「证道」目录段的 .rs 文件。
fn 是证道测试文件(路径: &Path) -> bool {
    路径
        .components()
        .any(|c| c.as_os_str().to_string_lossy() == "证道")
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::测试设施::工作区测试锁;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    fn 临时目录() -> PathBuf {
        let mut p = env::temp_dir();
        p.push(format!(
            "honghuang_scale_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn 空目录规模为零() {
        let 目录 = 临时目录();
        let 指标 = 统计规模(&目录);
        assert_eq!(指标.源码文件数, 0);
        assert_eq!(指标.源码总行数, 0);
        assert_eq!(指标.证道测试文件数, 0);
        assert_eq!(指标.crate数, 0);
    }

    #[test]
    fn 统计rs文件和行数() {
        let 目录 = 临时目录();
        fs::write(目录.join("a.rs"), "fn a() {}\n").unwrap();
        fs::write(目录.join("b.rs"), "fn b() {}\nfn c() {}\n").unwrap();
        let 指标 = 统计规模(&目录);
        assert_eq!(指标.源码文件数, 2);
        assert_eq!(指标.源码总行数, 3);
    }

    #[test]
    fn 统计crate数() {
        let 目录 = 临时目录();
        fs::write(目录.join("Cargo.toml"), "[package]\n").unwrap();
        let 指标 = 统计规模(&目录);
        assert_eq!(指标.crate数, 1);
    }

    #[test]
    fn 跳过target目录() {
        let 目录 = 临时目录();
        fs::create_dir_all(目录.join("target")).unwrap();
        fs::write(目录.join("target").join("x.rs"), "fn x() {}\n").unwrap();
        fs::write(目录.join("a.rs"), "fn a() {}\n").unwrap();
        let 指标 = 统计规模(&目录);
        assert_eq!(指标.源码文件数, 1);
    }

    #[test]
    fn 递归子目录() {
        let 目录 = 临时目录();
        fs::create_dir_all(目录.join("子")).unwrap();
        fs::write(目录.join("a.rs"), "fn a() {}\n").unwrap();
        fs::write(目录.join("子").join("b.rs"), "fn b() {}\n").unwrap();
        let 指标 = 统计规模(&目录);
        assert_eq!(指标.源码文件数, 2);
    }

    #[test]
    fn 识别证道测试文件() {
        let 目录 = 临时目录();
        fs::create_dir_all(目录.join("证道")).unwrap();
        fs::write(目录.join("证道").join("t.rs"), "#[test]\nfn t() {}\n").unwrap();
        fs::write(目录.join("a.rs"), "fn a() {}\n").unwrap();
        let 指标 = 统计规模(&目录);
        assert_eq!(指标.源码文件数, 2);
        assert_eq!(指标.证道测试文件数, 1);
    }

    #[test]
    fn 呈现项目规模含四项() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 目录 = 临时目录();
        fs::write(目录.join("a.rs"), "fn a() {}\n").unwrap();
        fs::write(目录.join("Cargo.toml"), "[package]\n").unwrap();
        // 临时改工作区根
        std::env::set_var("WORLD_WORKSPACE_ROOT", &目录);
        let 文本 = 呈现项目规模();
        assert!(文本.contains("源码 rs 文件数"));
        assert!(文本.contains("源码总行数"));
        assert!(文本.contains("证道测试文件数"));
        assert!(文本.contains("crate 数"));
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
    }
}
