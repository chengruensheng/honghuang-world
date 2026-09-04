#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use hm_contract::Component;
    use hm_signal::{信号, 信号总线, 信号载荷};
    use hm_signal_bus::{落盘信号总线, 异步信号总线};

    fn 临时路径(名: &str) -> String {
        std::env::temp_dir()
            .join(format!("zd_signal_test_{名}.jsonl"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn 落盘总线发布后写入日志文件() {
        let path = 临时路径("发布落盘");
        let _ = std::fs::remove_file(&path);
        let 总线 = 落盘信号总线::new();
        总线.设置落盘路径(&path).unwrap();
        总线.发布(&信号::新建("类型一".into(), 信号载荷 { 内容: Some("v".into()), ..信号载荷::default() }));
        总线.发布(&信号::新建("类型二".into(), 信号载荷::default()));
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 2);
        assert!(content.contains("类型一"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 加载重放历史信号() {
        let path = 临时路径("加载重放");
        let _ = std::fs::remove_file(&path);
        let 总线 = 落盘信号总线::new();
        总线.设置落盘路径(&path).unwrap();
        总线.发布(&信号::新建("任务完成".into(), 信号载荷 { 标识: Some("1".into()), ..信号载荷::default() }));
        总线.发布(&信号::新建("迭代完成".into(), 信号载荷::default()));
        drop(总线);
        let 重放 = 落盘信号总线::加载(&path).unwrap();
        let 历史 = 重放.全部();
        assert_eq!(历史.len(), 2);
        assert_eq!(历史[0].类型, "任务完成");
        assert_eq!(历史[0].载荷.标识, Some("1".to_string()));
        assert_eq!(历史[1].类型, "迭代完成");
        assert!(历史[1].载荷.标识.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 落盘总线同步分发处理器() {
        let 总线 = 落盘信号总线::new();
        let 计数 = Arc::new(AtomicU64::new(0));
        let 处理器: Arc<dyn Fn(&信号) + Send + Sync> = {
            let c = 计数.clone();
            Arc::new(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            })
        };
        总线.订阅("类型一", 处理器);
        总线.发布(&信号::新建("类型一".into(), 信号载荷::default()));
        总线.发布(&信号::新建("类型二".into(), 信号载荷::default()));
        assert_eq!(计数.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn 组件名称正确() {
        let 总线 = 落盘信号总线::new();
        assert_eq!(总线.name(), "落盘信号总线");
    }

    #[test]
    fn 异步总线排空后写入日志文件() {
        let path = 临时路径("异步落盘");
        let _ = std::fs::remove_file(&path);
        let 总线 = 异步信号总线::new();
        总线.设置落盘路径(&path).unwrap();
        总线.发布(&信号::新建("类型一".into(), 信号载荷 { 内容: Some("v".into()), ..信号载荷::default() }));
        总线.发布(&信号::新建("类型二".into(), 信号载荷::default()));
        总线.排空();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 2);
        assert!(content.contains("类型一"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 异步总线排空后分发处理器() {
        let 总线 = 异步信号总线::new();
        let 计数 = Arc::new(AtomicU64::new(0));
        let 处理器: Arc<dyn Fn(&信号) + Send + Sync> = {
            let c = 计数.clone();
            Arc::new(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            })
        };
        总线.订阅("类型一", 处理器);
        总线.发布(&信号::新建("类型一".into(), 信号载荷::default()));
        总线.发布(&信号::新建("类型二".into(), 信号载荷::default()));
        总线.排空();
        assert_eq!(计数.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn 异步总线组件名称正确() {
        let 总线 = 异步信号总线::new();
        assert_eq!(总线.name(), "异步信号总线");
    }

    #[test]
    fn 落盘总线并发发布无丢失() {
        let 总线 = Arc::new(落盘信号总线::new());
        let 线程数 = 8;
        let 每线程 = 25;
        let mut 线程集 = Vec::new();
        for t in 0..线程数 {
            let b = 总线.clone();
            线程集.push(std::thread::spawn(move || {
                for _ in 0..每线程 {
                    b.发布(&信号::新建(format!("并发{t}").into(), 信号载荷::default()));
                }
            }));
        }
        for 句柄 in 线程集 {
            句柄.join().unwrap();
        }
        assert_eq!(总线.全部().len(), 线程数 * 每线程);
    }

    #[test]
    fn 异步总线并发发布无丢失() {
        let 总线 = Arc::new(异步信号总线::new());
        let 线程数 = 8;
        let 每线程 = 25;
        let mut 线程集 = Vec::new();
        for t in 0..线程数 {
            let b = 总线.clone();
            线程集.push(std::thread::spawn(move || {
                for _ in 0..每线程 {
                    b.发布(&信号::新建(format!("并发{t}").into(), 信号载荷::default()));
                }
            }));
        }
        for 句柄 in 线程集 {
            句柄.join().unwrap();
        }
        总线.排空();
        assert_eq!(总线.全部().len(), 线程数 * 每线程);
    }
}