//! 增量 - 改写 - 园：把文件里第一次出现的旧文替换为新文。

/// 把文件里第一次出现的旧文替换为新文，旧文不存在则报错。
pub fn 改文件(路径: &str, 旧文: &str, 新文: &str) -> Result<(), String> {
    let 原文 = std::fs::read_to_string(路径).map_err(|错误| format!("读文件失败：{路径}：{错误}"))?;
    if !原文.contains(旧文) {
        return Err(format!("改文件失败：{路径}：未找到待替换内容"));
    }
    let 改后 = 原文.replacen(旧文, 新文, 1);
    std::fs::write(路径, 改后).map_err(|错误| format!("写文件失败：{路径}：{错误}"))
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 临时路径(名: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("手脚架_改文件_{}_{}", std::process::id(), 名))
    }

    #[test]
    fn 改文件替换首处() {
        let 路径 = 临时路径("替换首处.txt");
        std::fs::write(&路径, "甲乙甲").unwrap();
        改文件(路径.to_str().unwrap(), "甲", "丙").unwrap();
        assert_eq!(std::fs::read_to_string(&路径).unwrap(), "丙乙甲");
        std::fs::remove_file(&路径).unwrap();
    }

    #[test]
    fn 改文件找不到旧文报错() {
        let 路径 = 临时路径("找不到.txt");
        std::fs::write(&路径, "内容").unwrap();
        assert!(改文件(路径.to_str().unwrap(), "没有", "x").is_err());
        std::fs::remove_file(&路径).unwrap();
    }
}
