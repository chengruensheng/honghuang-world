//! 类型声明：状态共享的键值类型。
//!
//! 状态共享按 `TypeId` 索引运行时状态，键值类型为辅助类型别名，
//! 便于调用方表达"状态键""状态值"语义，实际存取由 状态-存取-殿 完成。
//! 业务状态类型（当前想法id/当前要求id）也定义于此，供各府经状态共享读写。

use std::any::TypeId;
use std::sync::Arc;

/// 状态键：按类型 ID 索引，同一类型全局唯一。
pub type 状态键 = TypeId;

/// 状态值：类型擦除的共享状态实例，须 Send + Sync 以跨线程共享。
pub type 状态值 = Arc<dyn std::any::Any + Send + Sync>;

/// 由类型生成状态键——`状态键::of::<T>()` 等价于 `TypeId::of::<T>()`。
pub fn 状态键_of<T: 'static>() -> 状态键 {
    TypeId::of::<T>()
}

/// 当前想法id——主循环正在处理的想法id（运行时状态，供观览查询读取）。
/// 主政一轮开始时写入，观览查询经 取全局状态().读取::<当前想法id>() 取最新值。
#[derive(Clone, Debug)]
pub struct 当前想法id(pub String);

/// 当前要求id——主循环正在处理的要求id（运行时状态，供观览查询读取）。
/// 运行一轮开始时写入，观览查询经 取全局状态().读取::<当前要求id>() 取最新值。
#[derive(Clone, Debug)]
pub struct 当前要求id(pub String);

/// 验证用id——端到端测试专用状态 id newtype，序列化契约同 当前设计id（字段名 `verify_id`）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct 验证用id(pub String);

impl 验证用id {
    pub const 序列化字段名: &'static str = "verify_id";

    pub fn 新(id: impl Into<String>) -> Self {
        验证用id(id.into())
    }

    pub fn 值(&self) -> &str {
        &self.0
    }
}

/// 当前设计id：跨府共享当前主控推进的设计稿标识。
/// 序列化形式统一 snake_case：JSON 字段名 `current_design_id`。
/// 并发契约：调用方保证启动期一次性写入、运行期只读，选用 RwLock 读多写少模式。
/// 生命周期：全局唯一单例槽位，session 内由主控独占写入，跨 session 不互覆盖。
/// 持久化禁止：仅作运行时内存标签，禁止落盘。
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct 当前设计id(pub String);

impl 当前设计id {
    /// JSON 字段名（snake_case 锁定），用于跨府契约序列化。
    pub const 序列化字段名: &'static str = "current_design_id";

    pub fn 新(id: impl Into<String>) -> Self {
        当前设计id(id.into())
    }

    pub fn 值(&self) -> &str {
        &self.0
    }
}

/// 序列化能力：将自身序列化为 JSON 字符串字段值（不含字段名）。
pub trait Serialize {
    fn serialize(&self) -> String;
}

/// 反序列化能力：从 JSON 字符串字段值还原。
pub trait Deserialize: Sized {
    type Error;
    fn deserialize(输入: &str) -> Result<Self, Self::Error>;
}

/// 状态键值错误：当前府类型在序列化/反序列化过程中抛出的错误。
#[derive(Debug)]
pub enum 状态键值错误 {
    反序列化失败 { 输入: String },
}

impl std::fmt::Display for 状态键值错误 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            状态键值错误::反序列化失败 { 输入 } => {
                write!(
                    f,
                    "状态键值反序列化失败：输入不是合法 JSON 字段值：{}",
                    输入
                )
            }
        }
    }
}

impl std::error::Error for 状态键值错误 {}

impl Serialize for 当前设计id {
    fn serialize(&self) -> String {
        format!("\"{}\"", self.0)
    }
}

impl Deserialize for 当前设计id {
    type Error = 状态键值错误;
    fn deserialize(输入: &str) -> Result<Self, Self::Error> {
        serde_json::from_str::<String>(输入)
            .map(当前设计id)
            .map_err(|_| 状态键值错误::反序列化失败 {
                输入: 输入.to_string(),
            })
    }
}

impl Serialize for 验证用id {
    fn serialize(&self) -> String {
        format!("\"{}\"", self.0)
    }
}

impl Deserialize for 验证用id {
    type Error = 状态键值错误;
    fn deserialize(输入: &str) -> Result<Self, Self::Error> {
        serde_json::from_str::<String>(输入)
            .map(验证用id)
            .map_err(|_| 状态键值错误::反序列化失败 {
                输入: 输入.to_string(),
            })
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 当前设计id_字段名锁定为_snake_case() {
        assert_eq!(当前设计id::序列化字段名, "current_design_id");
    }

    #[test]
    fn 当前设计id_序列化为_json字段值() {
        let id = 当前设计id::新("req-001");
        assert_eq!(id.serialize(), "\"req-001\"");
    }

    #[test]
    fn 当前设计id_反序列化合法字段值() {
        let id = 当前设计id::deserialize("\"req-001\"").unwrap();
        assert_eq!(id.0, "req-001");
        assert_eq!(id.值(), "req-001");
    }

    #[test]
    fn 当前设计id_反序列化空串合法() {
        // 空串视为「未推进任何设计」——合法状态。
        let id = 当前设计id::deserialize("\"\"").unwrap();
        assert_eq!(id.0, "");
        assert!(id.值().is_empty());
    }

    #[test]
    fn 当前设计id_反序列化损坏字符串报错() {
        assert!(当前设计id::deserialize("req-001").is_err());
        assert!(当前设计id::deserialize("\"未闭合").is_err());
        assert!(当前设计id::deserialize("\"a\"b\"").is_err());
        assert!(当前设计id::deserialize("").is_err());
    }

    #[test]
    fn 当前设计id_round_trip_含中划线() {
        let 原始 = 当前设计id::新("session-7/design-42");
        let 字段值 = 原始.serialize();
        let 还原 = 当前设计id::deserialize(&字段值).unwrap();
        assert_eq!(原始, 还原);
    }

    #[test]
    fn 当前设计id_round_trip_含中文() {
        let 原始 = 当前设计id::新("设计稿-甲阶段");
        let 字段值 = 原始.serialize();
        let 还原 = 当前设计id::deserialize(&字段值).unwrap();
        assert_eq!(原始, 还原);
    }

    #[test]
    fn 验证用id_字段名锁定为_verify_id() {
        assert_eq!(验证用id::序列化字段名, "verify_id");
    }

    #[test]
    fn 验证用id_round_trip_端到端契约() {
        let 原始 = 验证用id::新("verify-001/甲阶段");
        let 字段值 = 原始.serialize();
        assert_eq!(字段值, "\"verify-001/甲阶段\"");
        let 还原 = 验证用id::deserialize(&字段值).unwrap();
        assert_eq!(原始, 还原);
        assert_eq!(原始.值(), "verify-001/甲阶段");
    }
}
