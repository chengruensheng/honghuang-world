use serde::{Deserialize, Serialize};

/// 符号种类：代码中具名实体的类别
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum 符号种类 {
    函数,
    类型,
    常量,
}

/// 符号：代码中的具名实体（函数/类型/常量），附签名与所属模块
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct 符号 {
    pub 名称: String,
    pub 种类: 符号种类,
    pub 所属模块: String,
    pub 签名: Option<String>,
}

/// 模块：代码组织单元（如 crate / 文件）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct 模块 {
    pub 名称: String,
    pub 路径: String,
}

/// 依赖边：模块间的有向依赖（源依赖目标）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct 依赖边 {
    pub 源: String,
    pub 目标: String,
}

/// 图谱：项目整体模型，代码即真源；由模块/符号节点与依赖边构成
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct 图谱 {
    pub 模块集: Vec<模块>,
    pub 符号集: Vec<符号>,
    pub 依赖集: Vec<依赖边>,
}

impl 图谱 {
    pub fn 新() -> Self {
        图谱::default()
    }

    pub fn 添加模块(&mut self, 模块: 模块) {
        self.模块集.push(模块);
    }

    pub fn 添加符号(&mut self, 符号: 符号) {
        self.符号集.push(符号);
    }

    pub fn 添加依赖(&mut self, 依赖: 依赖边) {
        self.依赖集.push(依赖);
    }

    /// 返回指定模块直接依赖的所有模块名
    pub fn 查依赖(&self, 模块名: &str) -> Vec<String> {
        self.依赖集
            .iter()
            .filter(|边| 边.源 == 模块名)
            .map(|边| 边.目标.clone())
            .collect()
    }

    /// 返回直接依赖指定模块的所有模块名
    pub fn 被谁依赖(&self, 模块名: &str) -> Vec<String> {
        self.依赖集
            .iter()
            .filter(|边| 边.目标 == 模块名)
            .map(|边| 边.源.clone())
            .collect()
    }

    /// 按名称查询符号；不存在返回 None
    pub fn 查符号(&self, 名称: &str) -> Option<&符号> {
        self.符号集.iter().find(|符号| 符号.名称 == 名称)
    }
}