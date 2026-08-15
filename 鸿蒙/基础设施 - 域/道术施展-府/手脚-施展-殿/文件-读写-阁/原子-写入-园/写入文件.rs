//! 原子 - 写入 - 园：写文件全部内容，自动创建父目录。

use std::path::Path;

/// 写文件全部内容，父目录不存在时自动创建。
pub fn 写文件(路径: &str, 内容: &str) -> Result<(), String> {
    let 目标 = Path::new(路径);
    if let Some(父) = 目标.parent() {
        if !父.as_os_str().is_empty() {
            std::fs::create_dir_all(父)
                .map_err(|错误| format!("创建父目录失败：{}：{错误}", 父.display()))?;
        }
    }
    std::fs::write(目标, 内容).map_err(|错误| format!("写文件失败：{路径}：{错误}"))
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_写文件_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 写文件后可读回() {
        let 路径 = 临时路径("可读回.txt");
        写文件(路径.to_str().unwrap(), "内容").unwrap();
        assert_eq!(std::fs::read_to_string(&路径).unwrap(), "内容");
        std::fs::remove_file(&路径).unwrap();
    }

    #[test]
    fn 写文件自动建父目录() {
        let 目录 = 临时路径("子目录");
        let 路径 = 目录.join("a").join("b.txt");
        写文件(路径.to_str().unwrap(), "x").unwrap();
        assert_eq!(std::fs::read_to_string(&路径).unwrap(), "x");
        std::fs::remove_dir_all(&目录).unwrap();
    }
}
