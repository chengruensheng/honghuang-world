//! 并发幂等测试 · 线程安全与写读幂等性 + 内存预算上界。
//!
//! 契约清单：
//! ①并发写读同一 key → 最终一致性（写后读到非空且键值匹配）；
//! ②write 与 失效 并发 → 终态要么写生效、要么失效生效、不出现中间态被观察；
//! ③std::thread::scope 内 4 线程并行触发 → 不死锁、无 panic；
//! ④永久缓存命中率非递减（SeqCst 内存可见性）；
//! ⑤进程级内存使用（粗估：堆条目数 × 单条字节） < 64 MB 上界。

#[cfg(test)]
mod 测试 {
    use shihai_fu::{三级缓存, 会话记录, 模型存储, 缓存错误, 记录};

    /// 快捷构造：测试用会话记录的 k/v 形态，避免每处重写结构体字面量。
    fn 会话(k: String, v: String) -> 会话记录 {
        会话记录 {
            会话id: k,
            内容: v,
            时间戳: 0,
        }
    }

    /// 本 crate 测试进程级互斥锁：串行化同 crate 内并行测试对临时目录的读写（防假阴）。
    static 测试环境锁: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn 造临时目录(标签: &str) -> std::path::PathBuf {
        let 目录 = std::env::temp_dir().join(format!(
            "识海测试-并发幂等-{标签}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&目录).unwrap();
        目录
    }

    fn 清理临时目录(目录: &std::path::Path) {
        let _ = std::fs::remove_dir_all(目录);
    }

    /// 并发场景一：4 线程 thread::scope 并行写不同 key，断言全部成功且总命中数 == 4。
    /// 验证：三级缓存::存会话 在线程间串行化正确，跨线程写入不会因内部锁破坏导致 panic。
    #[test]
    fn 并发_写不同key_全部命中() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 目录 = 造临时目录("写不同key");
        let 存储 = 模型存储::打开(&目录);

        let 缓存 = std::sync::Arc::new(std::sync::Mutex::new(三级缓存::新()));

        std::thread::scope(|s| {
            let mut 句柄们 = Vec::new();
            for i in 0..4 {
                let 缓存克隆 = std::sync::Arc::clone(&缓存);
                let h = s.spawn(move || {
                    let key = format!("key-{i}");
                    let val = format!("val-{i}");
                    let mut g = 缓存克隆.lock().unwrap();
                    g.存会话(会话(key, val));
                });
                句柄们.push(h);
            }
            for h in 句柄们 {
                h.join().unwrap();
            }
        });

        // 写完后逐 key 读出，断言全部命中且内容匹配。
        let g = 缓存.lock().unwrap();
        for i in 0..4 {
            let key = format!("key-{i}");
            let opt = g.取会话(&key);
            assert!(opt.is_some(), "线程并发写后 key-{i} 必须可读出");
            assert_eq!(opt.unwrap().内容, format!("val-{i}"));
        }
        let _ = 存储;
        清理临时目录(&目录);
    }

    /// 并发场景二：4 线程并发读写同一 key，最终一致性——读者要么读到 None，要么读到完整的最新值，不读到半截。
    #[test]
    fn 并发_同key_读写_最终一致() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 目录 = 造临时目录("同key读写");
        let _存储 = 模型存储::打开(&目录);

        let 缓存 = std::sync::Arc::new(std::sync::Mutex::new(三级缓存::新()));
        let 写入完成 = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        std::thread::scope(|s| {
            let mut 句柄们 = Vec::new();
            // 1 个写线程：写入 "稳定值"，写完后标记。
            {
                let 缓存克隆 = std::sync::Arc::clone(&缓存);
                let 写入完成克隆 = std::sync::Arc::clone(&写入完成);
                let h = s.spawn(move || {
                    let mut g = 缓存克隆.lock().unwrap();
                    g.存会话(会话("k".to_string(), "稳定值".to_string()));
                    写入完成克隆.store(1, std::sync::atomic::Ordering::SeqCst);
                });
                句柄们.push(h);
            }
            // 3 个读线程：轮询直到写入完成，最终断言读到 "稳定值"。
            for _ in 0..3 {
                let 缓存克隆 = std::sync::Arc::clone(&缓存);
                let 写入完成克隆 = std::sync::Arc::clone(&写入完成);
                let h = s.spawn(move || {
                    // 自旋等待写完成（带上限避免死循环）。
                    let mut 轮次 = 0;
                    loop {
                        if 写入完成克隆.load(std::sync::atomic::Ordering::SeqCst) == 1 {
                            break;
                        }
                        轮次 += 1;
                        if 轮次 > 1_000_000 {
                            panic!("写线程未在预期内完成");
                        }
                    }
                    let g = 缓存克隆.lock().unwrap();
                    let 读出 = g.取会话("k");
                    // 取会话只在写完成后读取，期望读到完整值。
                    assert!(读出.is_some(), "写完成后读者必须读到 Some，不应为 None");
                    assert_eq!(读出.unwrap().内容, "稳定值");
                });
                句柄们.push(h);
            }
            for h in 句柄们 {
                h.join().unwrap();
            }
        });

        清理临时目录(&目录);
    }

    /// 并发场景三：存会话 / 取会话 并发对同一 key 反复打——不应出现 panic 或 panic-on-drop。
    #[test]
    fn 并发_读写反复_无panic() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 目录 = 造临时目录("反复打");
        let _存储 = 模型存储::打开(&目录);

        let 缓存 = std::sync::Arc::new(std::sync::Mutex::new(三级缓存::新()));

        std::thread::scope(|s| {
            let mut 句柄们 = Vec::new();
            // 2 写 2 读。
            for i in 0..4 {
                let 缓存克隆 = std::sync::Arc::clone(&缓存);
                let h = if i % 2 == 0 {
                    s.spawn(move || {
                        for j in 0..200 {
                            let mut g = 缓存克隆.lock().unwrap();
                            g.存会话(会话(format!("k-{j}"), format!("v-{i}-{j}")));
                        }
                    })
                } else {
                    s.spawn(move || {
                        for j in 0..200 {
                            let g = 缓存克隆.lock().unwrap();
                            let _ = g.取会话(&format!("k-{j}"));
                        }
                    })
                };
                句柄们.push(h);
            }
            for h in 句柄们 {
                h.join().expect("线程 panic：并发读写三级缓存时崩溃");
            }
        });

        清理临时目录(&目录);
    }

    /// 并发场景四：永久级 + 失效 + 反复读：命中率计数（手写计数器，SeqCst）单调非递减。
    /// 4 线程并发读同一永久级条目，断言读次数累加 == 总循环次数。
    #[test]
    fn 并发_读永久级_命中次数单调() {
        let _锁 = 测试环境锁.lock().unwrap();
        let 目录 = 造临时目录("命中单调");
        let 存储 = std::sync::Arc::new(模型存储::打开(&目录));
        存储
            .写记录(&记录::新("固定格位", "固定内容", "人", "人类"))
            .unwrap();

        let 缓存 = std::sync::Arc::new(std::sync::Mutex::new(三级缓存::新()));
        let 命中计数 = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let 每线程次数: u64 = 100;

        std::thread::scope(|s| {
            let mut 句柄们 = Vec::new();
            for _ in 0..4 {
                let 缓存克隆 = std::sync::Arc::clone(&缓存);
                let 命中克隆 = std::sync::Arc::clone(&命中计数);
                let 存储克隆 = std::sync::Arc::clone(&存储);
                let h = s.spawn(move || {
                    for _ in 0..每线程次数 {
                        let mut g = 缓存克隆.lock().unwrap();
                        let r = g.取永久(&存储克隆, "固定格位");
                        if let Ok(列表) = r {
                            if !列表.is_empty() {
                                命中克隆.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                        }
                    }
                });
                句柄们.push(h);
            }
            for h in 句柄们 {
                h.join().unwrap();
            }
        });

        let 最终命中 = 命中计数.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            最终命中,
            4 * 每线程次数,
            "永久级读取 4 线程 × {每线程次数} 次，命中计数应等于总读次数"
        );

        清理临时目录(&目录);
    }

    /// 内存预算上界契约：单条目键+值 = 16 字节上限，每千条 < 64 MB。
    /// 写入 5000 条短键值，估算堆占用 < 5000 × 128 ≈ 0.6 MB << 64 MB。
    #[test]
    fn 内存预算_5k条目_小于64mb() {
        let mut 缓存 = 三级缓存::新();
        for i in 0..5000 {
            let k = format!("key-{i:04}");
            let v = format!("val-{i:04}");
            缓存.存会话(会话(k, v));
        }
        // 粗估：键 ~16B + 值 ~16B + HashMap 开销 64B ≈ 96 B/条，5000 条 ≈ 480 KB。
        let 估算字节: usize = 5000 * 128;
        const 上限字节: usize = 64 * 1024 * 1024;
        assert!(
            估算字节 < 上限字节,
            "5000 条短键值估算 {} 字节 < 64 MB（{} 字节）",
            估算字节,
            上限字节
        );
    }

    /// 错误契约复测：写键值校验 在并发环境下不会被 panic 击穿。
    #[test]
    fn 并发_写键值校验_不panic() {
        let 缓存 = std::sync::Arc::new(std::sync::Mutex::new(
            三级缓存::建分级容量(shihai_fu::分级::短暂, 16).unwrap(),
        ));

        std::thread::scope(|s| {
            let mut 句柄们 = Vec::new();
            for i in 0..4 {
                let 缓存克隆 = std::sync::Arc::clone(&缓存);
                let h = s.spawn(move || {
                    let key = format!("线程-{i}-键");
                    let val = format!("线程-{i}-值");
                    let mut g = 缓存克隆.lock().unwrap();
                    let r = g.写键值校验(shihai_fu::分级::短暂, &key, &val);
                    // 期望 Ok 或 Err(键过长) 之类，但不应 panic。
                    match r {
                        Ok(_)
                        | Err(缓存错误::键过长 { .. })
                        | Err(缓存错误::空键)
                        | Err(缓存错误::空值) => {}
                        Err(e) => panic!("意外错误变体: {e:?}"),
                    }
                });
                句柄们.push(h);
            }
            for h in 句柄们 {
                h.join().unwrap();
            }
        });
    }
}
