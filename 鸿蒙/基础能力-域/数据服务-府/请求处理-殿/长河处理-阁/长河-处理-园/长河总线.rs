//! 长河总线：水镜统一通道的单一真相源（环形河 + 全局游标）。
//!
//! 设计稿 v1 目标「通道数 5→1、断线续传（游标归约）」的后端落点：
//! - 接待流与驱动过程流译制后的长河事件统一写入本总线；
//! - 事件携带单调递增的全局河序（河序），供 /api/river/events 按游标增量回放；
//! - 环形上限：超限丢弃最旧记录，河序仍单调递增（续传 since 永不失配）。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use qk_mirror::长河事件;

/// 一条带全局河序的长河事件（河序 = 总线续传游标）
#[derive(Debug, Clone)]
pub struct 河帧 {
    pub 河序: u64,
    pub 事: 长河事件,
}

/// 环形河：全局单调河序 + 有界存储。
///
/// 接待流与过程流共用同一总线，事件按到达先后混流排序；
/// 界面侧对同一事件序列归约即得同一会话视图（Rust/JS 同构验收）。
pub struct 长河总线 {
    河: Mutex<VecDeque<河帧>>,
    河序: AtomicU64,
    上限: usize,
}

impl 长河总线 {
    /// 新建环形河；`上限` 为内存内最大保留事件数。
    pub fn 新(上限: usize) -> Self {
        长河总线 {
            河: Mutex::new(VecDeque::new()),
            河序: AtomicU64::new(0),
            上限,
        }
    }

    /// 记录一条长河事件，返回其全局河序（从 1 起）。
    pub fn 记录(&self, 事: 长河事件) -> u64 {
        let 河序 = self.河序.fetch_add(1, Ordering::SeqCst) + 1;
        let mut 河 = self.河.lock().expect("长河总线锁中毒");
        if 河.len() >= self.上限 {
            河.pop_front();
        }
        河.push_back(河帧 { 河序, 事 });
        河序
    }

    /// 返回河序大于 `since` 的事件增量（有序）。
    pub fn 增量(&self, since: u64) -> Vec<河帧> {
        let 河 = self.河.lock().expect("长河总线增量锁中毒");
        河.iter()
            .filter(|帧| 帧.河序 > since)
            .cloned()
            .collect()
    }

    /// 返回当前最高河序（未记录任何事件时为 0）。
    pub fn 当前河序(&self) -> u64 {
        self.河序.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod 总线验证 {
    use super::*;

    #[test]
    fn 记录后河序单调_增量按游标过滤() {
        let 河 = 长河总线::新(100);
        let a = 河.记录(长河事件::回复开始 { 消息id: "m1".into(), 角色: "道祖".into() });
        let b = 河.记录(长河事件::回复增量 { 消息id: "m1".into(), 文: "你好".into() });
        assert!(b > a);

        let 增量 = 河.增量(a);
        assert_eq!(增量.len(), 1);
        assert_eq!(增量[0].河序, b);
    }

    #[test]
    fn 超限丢弃最旧_河序不重置() {
        let 河 = 长河总线::新(2);
        河.记录(长河事件::回复开始 { 消息id: "m1".into(), 角色: "道祖".into() });
        河.记录(长河事件::回复增量 { 消息id: "m1".into(), 文: "一".into() });
        let c = 河.记录(长河事件::回复结束 { 消息id: "m1".into() });
        assert_eq!(河.当前河序(), 3);
        // 最旧一条（河序 1）已被丢弃，增量(1) 只剩 2、3
        assert_eq!(河.增量(1).len(), 2);
        assert_eq!(河.增量(2).len(), 1);
        let _ = c;
    }
}
