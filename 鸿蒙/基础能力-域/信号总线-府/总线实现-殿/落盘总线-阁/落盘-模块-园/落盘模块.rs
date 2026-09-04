use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use hm_contract::Component;
use hm_error::{Error, Result};
use hm_signal::{信号, 信号总线};

type 处理器闭包 = Arc<dyn Fn(&信号) + Send + Sync>;
type 订阅条目 = (String, 处理器闭包);
type 订阅表 = Mutex<Vec<订阅条目>>;

/// 落盘信号总线：同步分发 + 发布即追加落盘（可靠事件流）。
///
/// 订阅关系不落盘；发布时把信号序列化为一行 JSON 追加到日志文件，
/// 供后续重放与审计。分发语义与内存信号总线一致（同步、重入安全）。
pub struct 落盘信号总线 {
    subscribers: 订阅表,
    日志文件: Mutex<Option<std::fs::File>>,
    历史: Mutex<Vec<信号>>,
}

impl 落盘信号总线 {
    pub fn new() -> Self {
        落盘信号总线 {
            subscribers: Mutex::new(Vec::new()),
            日志文件: Mutex::new(None),
            历史: Mutex::new(Vec::new()),
        }
    }

    /// 设置落盘路径：打开（或创建）追加日志文件；此后每次发布自动追加落盘
    pub fn 设置落盘路径(&self, path: &str) -> Result<()> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(Error::Io)?;
        *self.日志文件.lock().expect("落盘总线锁中毒") = Some(file);
        Ok(())
    }

    /// 历史信号（内存镜像，按发布顺序）
    pub fn 全部(&self) -> Vec<信号> {
        self.历史.lock().expect("落盘总线锁中毒").clone()
    }

    /// 从 JSONL 日志文件加载历史信号（订阅关系需重新注册）
    pub fn 加载(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path).map_err(Error::Io)?;
        let mut 历史 = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(Error::Io)?;
            if line.trim().is_empty() {
                continue;
            }
            let 信号: 信号 = serde_json::from_str(&line)
                .map_err(|e| Error::反序列化(format!("解析信号日志失败: {e}")))?;
            历史.push(信号);
        }
        Ok(落盘信号总线 {
            subscribers: Mutex::new(Vec::new()),
            日志文件: Mutex::new(None),
            历史: Mutex::new(历史),
        })
    }

    /// 追加一行信号到日志文件（落盘失败仅告警，不影响内存分发）
    fn 追加落盘(&self, 信号: &信号) {
        let mut guard = self.日志文件.lock().expect("落盘总线锁中毒");
        if let Some(file) = guard.as_mut() {
            match serde_json::to_string(信号) {
                Ok(line) => {
                    if writeln!(file, "{line}").is_err() {
                        tracing::warn!("信号落盘失败：写入日志文件出错");
                    } else if file.flush().is_err() {
                        tracing::warn!("信号落盘失败：刷新日志文件出错");
                    }
                }
                Err(e) => tracing::warn!("信号落盘失败：序列化出错 {e}"),
            }
        }
    }
}

impl Default for 落盘信号总线 {
    fn default() -> Self {
        落盘信号总线::new()
    }
}

impl Component for 落盘信号总线 {
    fn name(&self) -> &'static str { "落盘信号总线" }
}

impl 信号总线 for 落盘信号总线 {
    fn 发布(&self, 信号: &信号) {
        // 先落盘并入历史，再分发：即使订阅者 panic，数据已可靠落盘
        self.追加落盘(信号);
        self.历史.lock().expect("落盘总线锁中毒").push(信号.clone());
        let handlers: Vec<处理器闭包> = {
            let subs = self.subscribers.lock().expect("落盘总线锁中毒");
            subs.iter()
                .filter(|(t, _)| t == &信号.类型)
                .map(|(_, h)| h.clone())
                .collect()
        };
        for h in handlers {
            h(信号);
        }
    }

    fn 订阅(&self, 类型: &str, 处理器: 处理器闭包) {
        self.subscribers
            .lock()
            .expect("落盘总线锁中毒")
            .push((类型.to_string(), 处理器));
    }
}