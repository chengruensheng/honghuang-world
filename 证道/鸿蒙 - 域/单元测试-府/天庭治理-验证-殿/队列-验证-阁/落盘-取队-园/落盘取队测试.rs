//! 落盘 - 取队 - 园 · 落盘取队测试：入队取队水位与八态状态机。

#[cfg(test)]
mod 测试 {
    use serde::{Deserialize, Serialize};
    use std::fs;
    use tianting_fu::{要求状态, 落盘队列, 状态推进};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct 测试项 { 名: String }

    #[test]
    fn 入队取队水位() {
        let 路径 = std::env::temp_dir().join("识海测试-队列.jsonl");
        let 队列 = 落盘队列::<测试项>::打开(&路径);
        队列.入队(&测试项 { 名: "一".to_string() }).unwrap();
        队列.入队(&测试项 { 名: "二".to_string() }).unwrap();
        assert_eq!(队列.水位().unwrap(), 2);
        let 取 = 队列.取队().unwrap().unwrap();
        assert_eq!(取.名, "一");
        assert_eq!(队列.水位().unwrap(), 1);
        let _ = fs::remove_file(&路径);
    }

    #[test]
    fn 非法迁移被拒() {
        assert!(状态推进(&要求状态::待领, &要求状态::已存档).is_err());
        assert!(状态推进(&要求状态::待确认, &要求状态::设计中).is_ok());
    }
}
