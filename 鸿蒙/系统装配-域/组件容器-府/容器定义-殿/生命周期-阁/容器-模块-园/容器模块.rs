use std::sync::{Arc, Mutex};
use hm_contract::{Component, Initializable, Shutdown};
use hm_error::Result;

/// 命名组件：仅携带静态名称，用于把无法直接 upcast 的领域引擎登记进容器。
///
/// 引擎字段为 `Arc<Mutex<dyn 领域契约>>`，其 `Mutex` 包装无法转为 `Arc<dyn Component>`，
/// 故容器以静态名称登记引擎，保证统一列名与按名称审计（领域操作仍走装配体的类型化字段）。
struct 命名组件 {
    名: &'static str,
}

impl Component for 命名组件 {
    fn name(&self) -> &'static str {
        self.名
    }
}

/// 运行时组件容器：统一管理可插拔组件的注册、初始化与关闭生命周期
///
/// 三个能力桶独立存储，注册时由调用方明确组件具备的能力；
/// 生产路径注册真实实现，测试路径注册 mock，核心无特权。
pub struct 组件容器 {
    组件: Mutex<Vec<Arc<dyn Component>>>,
    可初始化: Mutex<Vec<Arc<dyn Initializable>>>,
    可关闭: Mutex<Vec<Arc<dyn Shutdown>>>,
}

impl 组件容器 {
    pub fn new() -> Self {
        组件容器 {
            组件: Mutex::new(Vec::new()),
            可初始化: Mutex::new(Vec::new()),
            可关闭: Mutex::new(Vec::new()),
        }
    }

    /// 注册基础组件（仅标识，不参与生命周期）
    pub fn 注册(&self, 组件: Arc<dyn Component>) {
        self.组件.lock().expect("容器锁中毒").push(组件);
    }

    /// 按静态名称登记领域引擎（仅列名/审计，领域操作走装配体类型化字段）
    pub fn 注册命名(&self, 名: &'static str) {
        self.注册(Arc::new(命名组件 { 名 }));
    }

    /// 按名称获取已注册组件（类型擦除容器，仅返回 Component 标识）
    pub fn 按名称获取(&self, 名: &str) -> Option<Arc<dyn Component>> {
        let 列表 = self.组件.lock().expect("容器锁中毒");
        列表.iter().find(|组件| 组件.name() == 名).cloned()
    }

    /// 注册可初始化组件（如日志器）
    pub fn 注册初始化(&self, 组件: Arc<dyn Initializable>) {
        self.可初始化.lock().expect("容器锁中毒").push(组件);
    }

    /// 注册可关闭组件
    pub fn 注册关闭(&self, 组件: Arc<dyn Shutdown>) {
        self.可关闭.lock().expect("容器锁中毒").push(组件);
    }

    /// 初始化所有可初始化组件，任一失败即返回错误
    pub fn 初始化(&self) -> Result<()> {
        let 列表 = self.可初始化.lock().expect("容器锁中毒");
        for 组件 in 列表.iter() {
            组件.init()?;
        }
        Ok(())
    }

    /// 关闭所有可关闭组件
    pub fn 关闭(&self) {
        let 列表 = self.可关闭.lock().expect("容器锁中毒");
        for 组件 in 列表.iter() {
            组件.shutdown();
        }
    }

    /// 已注册组件的名称列表
    pub fn 组件名(&self) -> Vec<&'static str> {
        let 列表 = self.组件.lock().expect("容器锁中毒");
        列表.iter().map(|组件| 组件.name()).collect()
    }
}