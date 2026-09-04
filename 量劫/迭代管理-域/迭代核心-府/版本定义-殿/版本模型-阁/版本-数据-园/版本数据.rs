use serde::{Deserialize, Serialize};

/// 语义化版本：主.次.修订（火之演进刻度）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub 主: u32,
    pub 次: u32,
    pub 修订: u32,
}

impl Version {
    pub fn new(主: u32, 次: u32, 修订: u32) -> Self {
        Version { 主, 次, 修订 }
    }

    /// 主版本演进（重大变革，归零次与修订）
    pub fn 递增主(&self) -> Self {
        Version { 主: self.主 + 1, 次: 0, 修订: 0 }
    }

    /// 次版本演进（新能力，归零修订）
    pub fn 递增次(&self) -> Self {
        Version { 主: self.主, 次: self.次 + 1, 修订: 0 }
    }

    /// 修订演进（修复）
    pub fn 递增修订(&self) -> Self {
        Version { 主: self.主, 次: self.次, 修订: self.修订 + 1 }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.主, self.次, self.修订)
    }
}