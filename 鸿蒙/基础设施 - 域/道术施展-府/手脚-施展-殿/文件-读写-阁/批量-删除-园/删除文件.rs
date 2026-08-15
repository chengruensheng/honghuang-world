//! 批量 - 删除 - 园：批量删除文件。

/// 批量删除多个文件，任一失败即返回错误。
pub fn 删文件(路径们: &[&str]) -> Result<(), String> {
    for 路径 in 路径们 {
        std::fs::remove_file(路径).map_err(|错误| format!("删文件失败：{路径}：{错误}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_删文件_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 删文件后不存在() {
        let 路径 = 临时路径("删除.txt");
        std::fs::write(&路径, "x").unwrap();
        let 文本 = 路径.to_str().unwrap().to_string();
        删文件(&[&文本]).unwrap();
        assert!(!路径.exists());
    }

    #[test]
    fn 删不存在的文件报错() {
        let 路径 = 临时路径("已不存在.txt");
        assert!(删文件(&[路径.to_str().unwrap()]).is_err());
    }
}
