//! 类型声明：事件总线的分发模式、监听回调、守卫裁决与工具流水线契约。
//!
//! 对齐 DeepSeek Harness 事件域（docs/architecture.zh.md §事件 / tool-execution-pipeline.zh.md）：
//! - 命名事件总线：通知（emit，观察）/ 串行（serial，按序 Err 中止）；并行由调用方线程表达；
//! - 类型化流水线：waterfall 语义（监听器可改写载荷，Err 即中止/拒绝）；
//! - 单调守卫：deny-or-abstain，identity 保护。
//!   事件总线 = 进程内实时扩展点；事件流（识海承载-府 追加落流.rs）= append-only 持久事实源。两者分工不混淆。

use std::path::PathBuf;

use shihai_fu::世界结果;

/// 分发模式（命名事件总线）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum 分发模式 {
    /// 通知（emit）：观察，按注册顺序调用，不因监听器错误中止。
    通知,
    /// 串行（serial）：按注册顺序调用，任一 Err 即中止（由事件声明方决定语义）。
    串行,
}

/// 事件载荷：类型擦除（Any），由事件声明方约定具体类型并 downcast。
/// 载荷须 Send（回调可跨线程），注册/分发在同一进程内完成。
pub type 载荷 = dyn std::any::Any + Send;

/// 监听回调：`Fn(&mut 载荷) -> 世界结果<()>`。
/// Err 语义由事件声明方定义（通知 = 记录不中止；串行 = 中止链）。
pub type 回调 = Box<dyn Fn(&mut 载荷) -> 世界结果<()> + Send + Sync>;

/// 注销句柄：drop 或手动 `.注销()` 时执行删除闭包（从注册表实际移除监听）。
/// 可逆副作用，对齐 Cordis disposer。删除动作由 分发-执行-殿 构造时捕获（事件名/序号/注册表弱引用）。
/// 不 derive Clone/Debug（FnOnce 删除闭包不可克隆），手动实现 Debug。
pub struct 注销句柄 {
    pub(crate) 动作: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl std::fmt::Debug for 注销句柄 {
    fn fmt(&self, 格式: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        格式.write_str(if self.动作.is_some() {
            "注销句柄{待注销}"
        } else {
            "注销句柄{已注销}"
        })
    }
}

impl 注销句柄 {
    /// 手动注销（幂等；drop 也会自动注销）。
    pub fn 注销(&mut self) {
        if let Some(动作) = self.动作.take() {
            动作();
        }
    }
}

impl Drop for 注销句柄 {
    fn drop(&mut self) {
        self.注销();
    }
}

/// 守卫裁决（单调守卫：deny-or-abstain）。
#[derive(Clone, Debug, PartialEq)]
pub enum 裁决 {
    /// 放行：本次请求通过（守卫只裁决一次，放行即止）。
    放行,
    /// 拒绝：本次请求被拦截（附原因）。
    拒绝(String),
    /// 弃权：本守卫不表态，交由下一守卫/默认策略。
    弃权,
}

/// 单调守卫：对一个请求做一次裁决（identity 保护——守卫只应裁决自己的职责面）。
pub trait 守卫: Send + Sync {
    fn 裁决(&self, 请求: &工具请求) -> 裁决;
}

/// 工具请求：工具流水线入参（调用名/参数/工作区/涉及路径）。
#[derive(Clone, Debug)]
pub struct 工具请求 {
    pub 调用名: String,
    pub 参数: serde_json::Value,
    pub 工作区根: PathBuf,
    pub 涉及路径: Vec<String>,
}

impl 工具请求 {
    pub fn 新(
        调用名: impl Into<String>,
        参数: serde_json::Value,
        工作区根: PathBuf,
        涉及路径: Vec<String>,
    ) -> Self {
        Self {
            调用名: 调用名.into(),
            参数,
            工作区根,
            涉及路径,
        }
    }
}

/// 工具结果：工具流水线出参（模型可见文本 + 本次写入文件 + 本次尝试写入的路径）。
#[derive(Clone, Debug, Default)]
pub struct 工具结果 {
    pub 文本: String,
    pub 写入文件们: Vec<(String, u64)>,
    /// 尝试写入的路径（写/改工具被调用即记，无论是否空操作跳过）——
    /// 供上层统计「同一路径反复尝试写入」防打磨空转（空操作跳过不入 写入文件们，但仍算尝试）。
    pub 尝试写入们: Vec<String>,
}

impl 工具结果 {
    pub fn 新(文本: impl Into<String>) -> Self {
        Self {
            文本: 文本.into(),
            写入文件们: Vec::new(),
            尝试写入们: Vec::new(),
        }
    }
}

/// 工具执行器：execute 段的真实工具函数体（映射 手脚-施展-殿 的函数）。
pub type 执行器 = Box<dyn Fn(&工具请求) -> 世界结果<工具结果> + Send + Sync>;

/// 工具流水线：四段（预执行 → 守卫们 → 执行 → 后执行）→ 落定。
/// 时序对齐 dsh tool-execution-pipeline：pre-execute → guards → execute → post-execute → result。
pub struct 工具流水线 {
    /// pre-execute：审批/护栏/沙箱准备（waterfall，任一拒绝 → 工具体跳过）。
    pub 预执行: crate::流水线<工具请求>,
    /// guards：单调守卫们（deny-or-abstain，identity 保护）。
    pub 守卫们: Vec<std::sync::Arc<dyn 守卫>>,
    /// execute：真实工具函数体（映射 手脚-施展-殿 的函数）。
    pub 执行: 执行器,
    /// post-execute：结果改写/补上下文/观测留痕（waterfall）。
    pub 后执行: crate::流水线<工具结果>,
}

impl 工具流水线 {
    /// 构造流水线（执行器必填；预执行/守卫/后执行默认空，可后注册）。
    pub fn 新(
        执行: impl Fn(&工具请求) -> 世界结果<工具结果> + Send + Sync + 'static
    ) -> Self {
        Self {
            预执行: crate::流水线::新(),
            守卫们: Vec::new(),
            执行: Box::new(执行),
            后执行: crate::流水线::新(),
        }
    }

    /// 预执行段注册监听（审批/护栏挂这里）。
    pub fn 预执行注册(
        &mut self,
        监听: impl Fn(&mut 工具请求) -> 世界结果<()> + Send + Sync + 'static,
    ) -> crate::注销句柄 {
        self.预执行.注册(监听)
    }

    /// 守卫段追加守卫。
    pub fn 加守卫(&mut self, 守卫: std::sync::Arc<dyn 守卫>) {
        self.守卫们.push(守卫);
    }

    /// 后执行段注册监听（结果改写/观测留痕挂这里）。
    pub fn 后执行注册(
        &mut self,
        监听: impl Fn(&mut 工具结果) -> 世界结果<()> + Send + Sync + 'static,
    ) -> crate::注销句柄 {
        self.后执行.注册(监听)
    }

    /// 执行完整流水线：预执行 → 守卫们 → 执行 → 后执行 → 落定。
    pub fn 执行(&self, 请求: &工具请求) -> 世界结果<工具结果> {
        let mut 请求 = 请求.clone();
        // pre-execute：任一拒绝 → 工具体跳过。
        self.预执行.执行(&mut 请求)?;
        // guards：monotonic deny-or-abstain（放行即止；全部弃权则放行）。
        for 守卫 in &self.守卫们 {
            match 守卫.裁决(&请求) {
                裁决::拒绝(原因) => return Err(format!("守卫拒绝：{原因}").into()),
                裁决::弃权 => continue,
                裁决::放行 => break,
            }
        }
        // execute：真实工具函数体。
        let mut 结果 = (self.执行)(&请求)?;
        // post-execute：结果改写/补上下文（waterfall）。
        self.后执行.执行(&mut 结果)?;
        // result：冻结，返回模型可见结果。
        Ok(结果)
    }
}
