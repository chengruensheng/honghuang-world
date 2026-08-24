//! §B.2.4 元数据：版本 + 兼容矩阵（12 crate 跨府版本协商）。
//!
//! 当前 crate（shihai_fu）作为基线版本；其他府通过 元数据::版本() 报告自身版本。
//! 兼容矩阵：major 不同 = 不兼容（API 变）；minor 不同 = 后向兼容（仅新增）；patch 不同 = 完全兼容。

/// 语义化版本
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct 版本 {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl 版本 {
    pub const fn 新(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    pub fn 字符串(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// 兼容矩阵：调用方调用方版本 vs 接收方版本 → 兼容性枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 兼容性 {
    /// patch 不同（1.x.y vs 1.x.y+1）— 完全兼容
    完全兼容,
    /// minor 不同（1.x vs 1.x+1）— 后向兼容（仅新增）
    后向兼容,
    /// major 不同（1.x vs 2.x）— 不兼容
    不兼容,
}

impl 兼容性 {
    /// 比较两版本：调用方 vs 接收方（返回 接收方 是否能处理 调用方 请求）
    pub fn 比较(调用方: 版本, 接收方: 版本) -> Self {
        if 调用方.major != 接收方.major {
            Self::不兼容
        } else if 调用方.minor != 接收方.minor {
            Self::后向兼容
        } else {
            Self::完全兼容
        }
    }
}

/// 当前 shihai_fu 版本
pub const 当前版本: 版本 = 版本::新(0, 1, 0);

/// 跨府版本协商：当前 库 vs 调用方期望
pub fn 协商(调用方期望: 版本) -> 兼容性 {
    兼容性::比较(调用方期望, 当前版本)
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 版本字符串() {
        assert_eq!(版本::新(1, 2, 3).字符串(), "1.2.3");
    }

    #[test]
    fn 比较_完全兼容() {
        assert_eq!(
            兼容性::比较(版本::新(1, 0, 0), 版本::新(1, 0, 5)),
            兼容性::完全兼容
        );
    }

    #[test]
    fn 比较_后向兼容() {
        assert_eq!(
            兼容性::比较(版本::新(1, 0, 0), 版本::新(1, 1, 0)),
            兼容性::后向兼容
        );
    }

    #[test]
    fn 比较_不兼容() {
        assert_eq!(
            兼容性::比较(版本::新(1, 0, 0), 版本::新(2, 0, 0)),
            兼容性::不兼容
        );
    }
}
