//! 事件总线的注册表与分发实现。
//!
//! 两种设施：
//! - `事件总线`：命名事件注册表（通知/串行分发，载荷类型擦除 Any）；
//! - `流水线<T>`：类型化 waterfall 链（强类型，无 Any 开销）。
//!   注册均返回 `注销句柄`：删除动作闭包捕获「注册表弱引用 + 事件名 + 序号」，
//!   drop/手动注销时从注册表实际移除（可逆副作用，对齐 Cordis disposer）。
//!   并发安全：注册表用 RwLock；分发持读锁按序调用（洪荒注册量小、分发频率低，短持读锁可接受）。

use crate::类型_定义_殿::{分发模式, 回调, 注销句柄, 载荷};
use rizhi_fu::{debug, warn};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};

type 注册表 = RwLock<HashMap<String, Vec<注册项>>>;

/// 注册表条目。
struct 注册项 {
    模式: 分发模式,
    回调: 回调,
    序号: u64,
}

/// 命名事件总线：事件名 → 监听器表（并发安全注册/分发）。
#[derive(Clone, Default)]
pub struct 事件总线 {
    注册表: Arc<注册表>,
    下一序号: Arc<AtomicU64>,
}

impl 事件总线 {
    /// 新建空总线。
    pub fn 新() -> Self {
        Self::default()
    }

    /// 注册监听：返回注销句柄（drop 自动移除；也可手动 `.注销()`）。
    /// 同事件同模式按注册顺序分发。
    pub fn 注册(&self, 事件: &str, 模式: 分发模式, 回调: 回调) -> 注销句柄 {
        let 序号 = self
            .下一序号
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let 事件名 = 事件.to_string();
        {
            let mut 表 = self.注册表.write().expect("事件总线注册表写锁");
            表.entry(事件名.clone()).or_default().push(注册项 {
                模式, 回调, 序号
            });
        }
        debug!(事件, ?模式, "事件监听已注册");
        // 删除闭包：弱引用升级后按事件名+序号真移除。
        let 弱表 = Arc::downgrade(&self.注册表);
        注销句柄 {
            动作: Some(Box::new(move || {
                if let Some(表) = 弱表.upgrade() {
                    if let Ok(mut 表) = 表.write() {
                        if let Some(项们) = 表.get_mut(&事件名) {
                            项们.retain(|项| 项.序号 != 序号);
                        }
                    }
                }
            })),
        }
    }

    /// 通知分发（emit）：观察，按注册顺序调用；监听器 Err 只记录不中止。
    pub fn 通知(&self, 事件: &str, 载荷: &mut 载荷) {
        let 表 = self.注册表.read().expect("事件总线注册表读锁");
        let Some(项们) = 表.get(事件) else {
            return;
        };
        let mut 项们: Vec<&注册项> = 项们.iter().filter(|项| 项.模式 == 分发模式::通知).collect();
        项们.sort_by_key(|项| 项.序号);
        for 项 in 项们 {
            if let Err(说明) = (项.回调)(载荷) {
                warn!(事件, 序号 = 项.序号, 说明 = %说明, "事件监听器报错（通知不中止）");
            }
        }
    }

    /// 串行分发（serial）：按注册顺序调用；任一 Err 即中止（由事件声明方决定语义）。
    pub fn 串行(&self, 事件: &str, 载荷: &mut 载荷) -> Result<(), String> {
        let 表 = self.注册表.read().expect("事件总线注册表读锁");
        let Some(项们) = 表.get(事件) else {
            return Ok(());
        };
        let mut 项们: Vec<&注册项> = 项们.iter().filter(|项| 项.模式 == 分发模式::串行).collect();
        项们.sort_by_key(|项| 项.序号);
        for 项 in 项们 {
            (项.回调)(载荷)
                .map_err(|说明| format!("事件「{事件}」监听器 #{} 失败：{说明}", 项.序号))?;
        }
        Ok(())
    }

    /// 事件上的监听器数（观测用）。
    pub fn 监听数(&self, 事件: &str) -> usize {
        self.注册表
            .read()
            .expect("事件总线注册表读锁")
            .get(事件)
            .map(Vec::len)
            .unwrap_or(0)
    }
}

/// 全局事件总线：进程级单例（static OnceLock），供各消费方共享同一注册表。
/// 事件名由各府声明（如「验收/裁决」「重投/循环」），载荷类型由事件声明方约定。
pub fn 全局总线() -> &'static 事件总线 {
    static 全局: std::sync::OnceLock<事件总线> = std::sync::OnceLock::new();
    全局.get_or_init(事件总线::新)
}

// ── 类型化流水线（waterfall）──

/// 流水线监听器：`Fn(&mut T) -> Result<(), String>`（waterfall 语义）。
type 流水线监听<T> = Box<dyn Fn(&mut T) -> Result<(), String> + Send + Sync>;

struct 流水线项<T> {
    回调: 流水线监听<T>,
    序号: u64,
}

/// 类型化 waterfall 链：监听器按注册顺序执行，任一 Err 即中止。
pub struct 流水线<T: 'static> {
    项们: Arc<RwLock<Vec<流水线项<T>>>>,
    下一序号: AtomicU64,
}

impl<T: 'static> Default for 流水线<T> {
    fn default() -> Self {
        Self {
            项们: Arc::new(RwLock::new(Vec::new())),
            下一序号: AtomicU64::new(0),
        }
    }
}

impl<T: 'static> 流水线<T> {
    /// 新建空流水线。
    pub fn 新() -> Self {
        Self::default()
    }

    /// 注册监听：返回注销句柄（drop 自动移除）。
    pub fn 注册(
        &mut self,
        监听: impl Fn(&mut T) -> Result<(), String> + Send + Sync + 'static,
    ) -> 注销句柄 {
        let 序号 = self
            .下一序号
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        {
            let mut 项们 = self.项们.write().expect("流水线写锁");
            项们.push(流水线项 {
                回调: Box::new(监听),
                序号,
            });
        }
        let 弱项们 = Arc::downgrade(&self.项们);
        注销句柄 {
            动作: Some(Box::new(move || {
                if let Some(项们) = 弱项们.upgrade() {
                    if let Ok(mut 项们) = 项们.write() {
                        项们.retain(|项| 项.序号 != 序号);
                    }
                }
            })),
        }
    }

    /// 执行链：按注册顺序执行，任一 Err 即中止（waterfall 语义）。
    pub fn 执行(&self, 载荷: &mut T) -> Result<(), String> {
        let 项们 = self.项们.read().expect("流水线读锁");
        let mut 项们: Vec<&流水线项<T>> = 项们.iter().collect();
        项们.sort_by_key(|项| 项.序号);
        for 项 in 项们 {
            (项.回调)(载荷)?;
        }
        Ok(())
    }

    /// 链上监听器数（观测用）。
    pub fn 长度(&self) -> usize {
        self.项们.read().expect("流水线读锁").len()
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::类型_定义_殿::{守卫, 工具流水线, 工具结果, 工具请求, 裁决};

    fn 造请求(名: &str) -> 工具请求 {
        工具请求::新(
            名,
            serde_json::json!({}),
            std::path::PathBuf::from("根"),
            vec![],
        )
    }

    /// 通知分发：观察不中止，监听器 Err 不阻断后续。
    #[test]
    fn 通知_监听器报错不中止() {
        let 总线 = 事件总线::新();
        let _甲 = 总线.注册(
            "测试/事件",
            分发模式::通知,
            Box::new(|_| Err("故意失败".to_string())),
        );
        let _乙 = 总线.注册("测试/事件", 分发模式::通知, Box::new(|_| Ok(())));
        let mut 载荷: Box<载荷> = Box::new(0u32);
        总线.通知("测试/事件", 载荷.as_mut());
        assert_eq!(总线.监听数("测试/事件"), 2, "报错监听不注销，仅不中止");
    }

    /// 串行分发：按注册顺序，Err 中止。
    #[test]
    fn 串行_按序且报错中止() {
        let 总线 = 事件总线::新();
        let 序 = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let 序甲 = 序.clone();
        let _甲 = 总线.注册(
            "测试/串行",
            分发模式::串行,
            Box::new(move |_| {
                序甲.lock().unwrap().push(1);
                Ok(())
            }),
        );
        let 序乙 = 序.clone();
        let _乙 = 总线.注册(
            "测试/串行",
            分发模式::串行,
            Box::new(move |_| {
                序乙.lock().unwrap().push(2);
                Err("中止".to_string())
            }),
        );
        let 序丙 = 序.clone();
        let _丙 = 总线.注册(
            "测试/串行",
            分发模式::串行,
            Box::new(move |_| {
                序丙.lock().unwrap().push(3);
                Ok(())
            }),
        );
        let mut 载荷: Box<载荷> = Box::new(0u32);
        let 结果 = 总线.串行("测试/串行", 载荷.as_mut());
        assert!(结果.is_err(), "第二个监听报错应中止");
        assert_eq!(*序.lock().unwrap(), vec![1, 2], "第三个监听不应执行");
    }

    /// 注销句柄：drop 后监听真正移除（监听数归零），不再分发。
    #[test]
    fn 注销句柄_丢弃后真移除() {
        let 总线 = 事件总线::新();
        let 句柄 = 总线.注册(
            "测试/注销",
            分发模式::串行,
            Box::new(|_| Err("不该执行".to_string())),
        );
        assert_eq!(总线.监听数("测试/注销"), 1);
        drop(句柄);
        assert_eq!(总线.监听数("测试/注销"), 0, "drop 后应从注册表真移除");
        let mut 载荷: Box<载荷> = Box::new(0u32);
        assert!(
            总线.串行("测试/注销", 载荷.as_mut()).is_ok(),
            "注销后应无监听，串行应 Ok"
        );
    }

    /// 流水线：waterfall 短路（第二个 Err 后第三个不执行）。
    #[test]
    fn 流水线_短路中止() {
        let mut 链 = 流水线::<u32>::新();
        let 序 = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let 序甲 = 序.clone();
        let _甲 = 链.注册(move |载荷| {
            *载荷 += 1;
            序甲.lock().unwrap().push(*载荷);
            Ok(())
        });
        let 序乙 = 序.clone();
        let _乙 = 链.注册(move |_| {
            序乙.lock().unwrap().push(99);
            Err("中止".to_string())
        });
        let mut 载荷 = 0u32;
        let 结果 = 链.执行(&mut 载荷);
        assert!(结果.is_err());
        assert_eq!(*序.lock().unwrap(), vec![1, 99], "第三个（若有）不应执行");
    }

    /// 工具流水线：预执行拒绝 → 执行体不跑。
    #[test]
    fn 工具流水线_预执行拒绝则执行体不跑() {
        let 跑了 = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let 跑了探针 = 跑了.clone();
        let mut 链 = 工具流水线::新(move |_| {
            跑了探针.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(工具结果::新("结果"))
        });
        let _护栏 = 链.预执行注册(|_| Err("预执行拒绝".to_string()));
        let 结果 = 链.执行(&造请求("写文件"));
        assert!(结果.is_err());
        assert!(
            !跑了.load(std::sync::atomic::Ordering::SeqCst),
            "预执行拒绝后执行体不应运行"
        );
    }

    /// 工具流水线：守卫 拒绝/弃权/放行 语义。
    #[test]
    fn 工具流水线_守卫拒绝拦截() {
        struct 拦命令 {
            名: &'static str,
        }
        impl 守卫 for 拦命令 {
            fn 裁决(&self, 请求: &工具请求) -> 裁决 {
                if 请求.调用名 == self.名 {
                    裁决::拒绝("该命令被禁".to_string())
                } else {
                    裁决::弃权
                }
            }
        }
        let mut 链 = 工具流水线::新(|_| Ok(工具结果::新("结果")));
        链.加守卫(std::sync::Arc::new(拦命令 {
            名: "运行命令"
        }));
        let 结果 = 链.执行(&造请求("运行命令"));
        assert!(结果.is_err(), "守卫拒绝应拦截");
        assert!(结果.unwrap_err().contains("该命令被禁"));
        // 无关调用：弃权 → 放行
        let 结果 = 链.执行(&造请求("读文件"));
        assert!(结果.is_ok(), "弃权守卫不拦无关调用");
    }

    /// 工具流水线：后执行可改写结果（补上下文/观测留痕挂这里）。
    #[test]
    fn 工具流水线_后执行改写结果() {
        let mut 链 = 工具流水线::新(|_| Ok(工具结果::新("原始")));
        let _改写 = 链.后执行注册(|结果| {
            结果.文本.push_str("+补上下文");
            Ok(())
        });
        let 结果 = 链.执行(&造请求("读文件")).unwrap();
        assert_eq!(结果.文本, "原始+补上下文");
    }
}
