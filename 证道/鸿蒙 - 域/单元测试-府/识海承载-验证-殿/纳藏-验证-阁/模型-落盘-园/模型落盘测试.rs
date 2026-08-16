//! 模型-落盘-园 · 测试：工作区定位 + 格位/记录落盘读写。

#[cfg(test)]
mod 测试 {
    use shihai_fu::{模型存储, 记录, 工作区};
    use std::fs;

    #[test]
    fn 写入再读回一致() {
        let 目录 = std::env::temp_dir().join("识海测试-模型落盘");
        let 存储 = 模型存储::打开(&目录);
        let 记录 = 记录::新("结构", "鸿蒙/基础设施-域", "测试", "代码");
        存储.写记录(&记录).unwrap();
        let 读回 = 存储.读格位("结构").unwrap();
        assert_eq!(读回.len(), 1);
        assert_eq!(读回[0].内容, "鸿蒙/基础设施-域");
        let _ = fs::remove_dir_all(&目录);
    }

    #[test]
    fn 工作区初始化建目录() {
        let 根 = std::env::temp_dir().join("识海测试-工作区");
        let 工作区 = 工作区::新(&根);
        工作区.初始化().unwrap();
        assert!(工作区.格位目录().is_dir());
        assert!(工作区.会话目录().is_dir());
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 在工作区落盘到上下文() {
        let 根 = std::env::temp_dir().join("识海测试-工作区落盘");
        let 工作区 = 工作区::新(&根);
        let 存储 = 模型存储::在工作区(&工作区);
        let 记录 = 记录::新("结构", "落盘", "测试", "代码");
        存储.写记录(&记录).unwrap();
        assert!(工作区.格位目录().join("结构.jsonl").exists());
        let _ = fs::remove_dir_all(&根);
    }
}
