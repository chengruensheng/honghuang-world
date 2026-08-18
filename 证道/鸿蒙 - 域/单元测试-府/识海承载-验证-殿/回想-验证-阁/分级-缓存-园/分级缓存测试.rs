//! 分级-缓存-园 · 测试：三级缓存（永久 / 版本 / 会话）+ 预算生长。

#[cfg(test)]
mod 测试 {
    use shihai_fu::{
        三级缓存, 共享度, 固化度, 最低预算格位, 权格位, 格位, 模型存储, 经格位, 范畴, 记录,
        顺序档位,
    };

    /// 本 crate 测试进程级互斥锁：串行化同 crate 内并行测试下临时目录读写（防假阴）。
    /// 100 次 cargo test 验证：未加锁时"识海测试-缓存"固定路径多次跑写追加导致 JSON 多行 trailing characters 假阴。
    /// 加锁后用 process::id + 纳秒命名临时目录，每次跑独立。
    /// （2026-08-18 DSH 兜底：照 `缓存读取.rs` / `模型落盘测试.rs` / `落盘取队测试.rs` 同模式。）
    static 测试环境锁: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn 造格位(名字: &str, 固化: 固化度, 档位: 顺序档位) -> 格位 {
        格位::新(名字, 范畴::世界, "种子", "代码", 固化, 共享度::共享, 档位)
    }

    #[test]
    fn 按固化度分格位() {
        let 格位们 = vec![
            造格位("铁律", 固化度::经, 顺序档位::最前),
            造格位("结构", 固化度::权, 顺序档位::中间),
        ];
        assert_eq!(经格位(&格位们).len(), 1);
        assert_eq!(权格位(&格位们).len(), 1);
    }

    #[test]
    fn 最低预算不含中间() {
        let 格位们 = vec![
            造格位("铁律", 固化度::经, 顺序档位::最前),
            造格位("结构", 固化度::权, 顺序档位::中间),
            造格位("目标", 固化度::权, 顺序档位::最后),
        ];
        assert_eq!(最低预算格位(&格位们).len(), 2);
    }

    #[test]
    fn 永久缓存命中与失效() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 目录 = std::env::temp_dir().join(format!(
            "识海测试-缓存-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let 存储 = 模型存储::打开(&目录);
        存储
            .写记录(&记录::新("铁律", "约束一", "人", "人类"))
            .unwrap();

        let mut 缓存 = 三级缓存::新();
        assert_eq!(缓存.取永久(&存储, "铁律").unwrap().len(), 1);
        // 失效后再取：仍从存储读到（缓存已清，但存储有数据）
        缓存.失效永久("铁律");
        assert_eq!(缓存.取永久(&存储, "铁律").unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&目录);
    }

    #[test]
    fn 拼装结果指纹复用() {
        let mut 缓存 = 三级缓存::新();
        let 次数 = std::cell::Cell::new(0);
        let 拼装 = || {
            次数.set(次数.get() + 1);
            Ok::<_, String>("投影结果".to_string())
        };
        assert_eq!(缓存.拼装("指纹A", 拼装).unwrap(), "投影结果");
        assert_eq!(缓存.拼装("指纹A", 拼装).unwrap(), "投影结果");
        // 同一指纹只重拼一次
        assert_eq!(次数.get(), 1);
    }
}
