//! 批量 - 列目 - 园：列出一个目录下的条目。

use std::fs;
use std::path::Path;

/// 目录条目：名称、是否目录、字节数（目录字节数为 0）。
#[derive(Clone, Debug, PartialEq)]
pub struct 目录条目 {
    pub 名称: String,
    pub 是目录: bool,
    pub 字节数: u64,
}

/// 列出一个目录下的条目，按名称升序。
pub fn 列举目录(路径: &str) -> Result<Vec<目录条目>, String> {
    let 目录 = Path::new(路径);
    if !目录.is_dir() {
        return Err(format!("目录不存在：{路径}"));
    }
    let 迭代 = fs::read_dir(目录).map_err(|错误| format!("列目录失败：{路径}：{错误}"))?;
    let mut 条目们 = Vec::new();
    for 条目 in 迭代.flatten() {
        let 条目路径 = 条目.path();
        let 是目录 = 条目路径.is_dir();
        let 字节数 = if 是目录 {
            0
        } else {
            条目.metadata().map(|元数据| 元数据.len()).unwrap_or(0)
        };
        条目们.push(目录条目 {
            名称: 条目.file_name().to_string_lossy().to_string(),
            是目录,
            字节数,
        });
    }
    条目们.sort_by(|甲, 乙| 甲.名称.cmp(&乙.名称));
    Ok(条目们)
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_列目录_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 列目录取条目() {
        let 目录 = 临时路径("目录");
        std::fs::create_dir_all(&目录).unwrap();
        std::fs::write(目录.join("b.txt"), "b").unwrap();
        std::fs::write(目录.join("a.txt"), "aa").unwrap();

        let 条目们 = 列举目录(目录.to_str().unwrap()).unwrap();
        assert_eq!(条目们.len(), 2);
        assert_eq!(条目们[0].名称, "a.txt");
        assert_eq!(条目们[0].字节数, 2);
        assert_eq!(条目们[1].名称, "b.txt");

        std::fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 列目录不存在报错() {
        let 目录 = 临时路径("不存在目录");
        assert!(列举目录(目录.to_str().unwrap()).is_err());
    }
}
