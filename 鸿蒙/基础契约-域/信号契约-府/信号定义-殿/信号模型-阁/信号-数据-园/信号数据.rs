use serde::{Deserialize, Serialize};

/// 引擎支撑宏：为领域引擎生成共用的信号注入、自动持久化与信号发布样板代码。
///
/// 前提：调用方结构体需具备字段
/// `信号总线: Option<Arc<dyn 信号总线>>` 与 `持久化路径: Option<String>`，
/// 并实现 `保存(&self, path: &str) -> Result<()>`。
#[macro_export]
macro_rules! 引擎支撑 {
    () => {
        /// 注入信号总线（府可插拔：装配层串联跨引擎信号）
        pub fn 设置信号总线(&mut self, 总线: std::sync::Arc<dyn $crate::信号总线>) {
            self.信号总线 = Some(总线);
        }

        /// 设置持久化路径（自动持久化：此后每次变更自动落盘）
        pub fn 设置持久化路径(&mut self, 路径: String) {
            self.持久化路径 = Some(路径);
        }

        /// 自动保存：已设置持久化路径时落盘，失败仅告警不 panic
        fn 自动保存(&self) {
            if let Some(路径) = &self.持久化路径 {
                if let Err(e) = self.保存(路径) {
                    ::tracing::warn!("自动持久化失败: {e}");
                }
            }
        }

        /// 发布信号（无总线时静默忽略）
        fn 发布信号(&self, 类型: &str, 载荷: $crate::信号载荷) {
            if let Some(总线) = &self.信号总线 {
                总线.发布(&$crate::信号::新建(类型.to_string(), 载荷));
            }
        }
    };
}

/// 信号载荷：跨引擎信号的强类型载荷，替代弱类型键值对列表（字段与 `载荷键` 常量一一对应）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct 信号载荷 {
    /// 主键标识（任务/迭代/事件 id 等）
    pub 标识: Option<String>,
    /// 标题
    pub 标题: Option<String>,
    /// 描述
    pub 描述: Option<String>,
    /// 版本
    pub 版本: Option<String>,
    /// 变更说明
    pub 变更说明: Option<String>,
    /// 内容
    pub 内容: Option<String>,
    /// 标签
    pub 标签: Option<String>,
    /// 规则名
    pub 规则名: Option<String>,
    /// 结论
    pub 结论: Option<String>,
    /// 类型
    pub 类型: Option<String>,
}

/// 信号：跨引擎编排的最小信号单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 信号 {
    /// 信号类型（见 `信号类型` 常量）
    pub 类型: String,
    /// 载荷：强类型载荷（见 `信号载荷`）
    pub 载荷: 信号载荷,
}

impl 信号 {
    pub fn 新建(类型: String, 载荷: 信号载荷) -> Self {
        信号 { 类型, 载荷 }
    }
}

/// 信号类型常量（跨引擎统一契约，消除魔法字符串）
pub mod 信号类型 {
    /// 木：任务完成
    pub const 任务完成: &str = "任务完成";
    /// 木：任务推进（非完成状态变更）
    pub const 任务推进: &str = "任务推进";
    /// 火：迭代完成
    pub const 迭代完成: &str = "迭代完成";
    /// 火：迭代放弃
    pub const 迭代放弃: &str = "迭代放弃";
    /// 土：记忆写入
    pub const 记忆写入: &str = "记忆写入";
    /// 金：规则命中
    pub const 规则命中: &str = "规则命中";
    /// 水：事件发布
    pub const 事件发布: &str = "事件发布";
}

/// 信号载荷键常量（跨引擎统一契约，消除魔法字符串）
pub mod 载荷键 {
    pub const 标识: &str = "标识";
    pub const 标题: &str = "标题";
    pub const 描述: &str = "描述";
    pub const 版本: &str = "版本";
    pub const 变更说明: &str = "变更说明";
    pub const 内容: &str = "内容";
    pub const 标签: &str = "标签";
    pub const 规则名: &str = "规则名";
    pub const 结论: &str = "结论";
    pub const 类型: &str = "类型";
}