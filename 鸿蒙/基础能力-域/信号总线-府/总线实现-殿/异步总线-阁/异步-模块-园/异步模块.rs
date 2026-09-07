use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use hm_contract::Component;
use hm_error::Result;
use hm_signal::{信号, 信号总线};
use crate::落盘信号总线;

/// 异步信号总线：发布即入队，后台线程异步分发 + 落盘。
///
/// 相比同步的 `落盘信号总线`，`发布` 不阻塞调用线程（仅入队）；
/// 分发、落盘与历史记录由单一后台线程顺序消费，天然串行、无锁竞争。
/// 通过 `排空()` 保证此前发布的所有信号已被处理完毕（最终一致）。
/// 订阅关系与落盘/历史语义均委托给内部 `落盘信号总线`，保持一致。
/// 通过 `停止()` 优雅终止后台线程并等待其退出。
enum 消息 {
    信号(信号),
    排空(SyncSender<()>),
    停止,
}

pub struct 异步信号总线 {
    落盘总线: Arc<落盘信号总线>,
    发送端: Sender<消息>,
    句柄: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl 异步信号总线 {
    pub fn new() -> Self {
        let 落盘总线 = Arc::new(落盘信号总线::new());
        let (发送端, 接收端) = mpsc::channel::<消息>();
        let 消费总线 = 落盘总线.clone();
        let 句柄 = std::thread::spawn(move || 消费循环(接收端, 消费总线));
        异步信号总线 { 落盘总线, 发送端, 句柄: Mutex::new(Some(句柄)) }
    }

    /// 设置落盘路径（转发给内部落盘总线）
    pub fn 设置落盘路径(&self, path: &str) -> Result<()> {
        self.落盘总线.设置落盘路径(path)
    }

    /// 历史信号（`排空()` 后可见最新）
    pub fn 全部(&self) -> Vec<信号> {
        self.落盘总线.全部()
    }

    /// 等待此前发布的所有信号被后台线程处理完毕
    pub fn 排空(&self) {
        let (回复, 收到) = mpsc::sync_channel(0);
        if self.发送端.send(消息::排空(回复)).is_ok() {
            let _ = 收到.recv();
        }
    }

    /// 优雅停止：通知后台线程退出并等待其结束
    pub fn 停止(&self) {
        let _ = self.发送端.send(消息::停止);
        if let Some(句柄) = self.句柄.lock().expect("异步总线句柄锁中毒").take() {
            if 句柄.join().is_err() {
                tracing::warn!("异步总线后台线程 join 失败（线程可能 panic）");
            }
        }
    }
}

fn 消费循环(接收端: Receiver<消息>, 总线: Arc<落盘信号总线>) {
    while let Ok(消息) = 接收端.recv() {
        match 消息 {
            消息::信号(信号) => 总线.发布(&信号),
            消息::排空(回复) => {
                let _ = 回复.send(());
            }
            消息::停止 => break,
        }
    }
}

impl Default for 异步信号总线 {
    fn default() -> Self {
        异步信号总线::new()
    }
}

impl Component for 异步信号总线 {
    fn name(&self) -> &'static str { "异步信号总线" }
}

impl 信号总线 for 异步信号总线 {
    fn 发布(&self, 信号: &信号) {
        if let Err(e) = self.发送端.send(消息::信号(信号.clone())) {
            tracing::warn!("异步信号总线：发布失败（后台线程已停止）: {e}");
        }
    }

    fn 订阅(&self, 类型: &str, 处理器: Arc<dyn Fn(&信号) + Send + Sync>) {
        self.落盘总线.订阅(类型, 处理器);
    }
}