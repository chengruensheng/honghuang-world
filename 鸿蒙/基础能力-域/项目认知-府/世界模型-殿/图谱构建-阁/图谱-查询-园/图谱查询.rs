use crate::{图谱, 模块, 符号, 符号种类};

/// 图谱查询扩展：按名 / 按类型 / 按路径 / 依赖 / 被谁依赖（对齐原型 图谱库.py）
impl 图谱 {
    /// 按名称（子串）查模块，可能有多个同名
    pub fn 按模块名(&self, 名称: &str) -> Vec<&模块> {
        self.模块集
            .iter()
            .filter(|模块| 模块.名称.contains(名称))
            .collect()
    }

    /// 按名称（子串）查符号，可能有多个同名
    pub fn 按符号名(&self, 名称: &str) -> Vec<&符号> {
        self.符号集
            .iter()
            .filter(|符号| 符号.名称.contains(名称))
            .collect()
    }

    /// 按路径片段（子串）查模块，用于「这个模块在哪」
    pub fn 按路径(&self, 片段: &str) -> Vec<&模块> {
        self.模块集
            .iter()
            .filter(|模块| 模块.路径.contains(片段))
            .collect()
    }

    /// 按符号种类列出符号（函数/类型/常量）
    pub fn 按符号种类(&self, 种类: 符号种类) -> Vec<&符号> {
        self.符号集
            .iter()
            .filter(|符号| 符号.种类 == 种类)
            .collect()
    }

    /// 全部依赖边（源→目标）
    pub fn 全部依赖(&self) -> &[crate::依赖边] {
        &self.依赖集
    }

    /// 全部模块
    pub fn 全部模块(&self) -> &[crate::模块] {
        &self.模块集
    }

    /// 全部符号
    pub fn 全部符号(&self) -> &[crate::符号] {
        &self.符号集
    }
}
