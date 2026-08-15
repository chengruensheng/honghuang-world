//! 登记 - 落册 - 园：角色卡登记落册，供工作流引擎按职司调取。

use crate::类型_定义_殿::执行角色;
use std::collections::HashMap;

/// 角色册：身份 → 执行角色。
#[derive(Default, Clone)]
pub struct 角色册 {
    角色们: HashMap<String, 执行角色>,
}

impl 角色册 {
    /// 空册。
    pub fn 新() -> 角色册 {
        角色册::default()
    }

    /// 登记一张角色卡（同身份覆盖）。
    pub fn 登记(&mut self, 角色: 执行角色) {
        self.角色们.insert(角色.身份.clone(), 角色);
    }

    /// 按身份取角色卡。
    pub fn 取(&self, 身份: &str) -> Option<&执行角色> {
        self.角色们.get(身份)
    }

    /// 全部身份清单。
    pub fn 全部身份(&self) -> Vec<&String> {
        self.角色们.keys().collect()
    }

    /// 角色数量。
    pub fn 数量(&self) -> usize {
        self.角色们.len()
    }

    /// 保存到 json 文件。
    pub fn 保存(&self, 路径: &str) -> Result<(), String> {
        let 文本 = serde_json::to_string_pretty(&self.角色们).map_err(|错误| format!("序列化角色册失败: {错误}"))?;
        std::fs::write(路径, 文本).map_err(|错误| format!("保存角色册失败: {错误}"))
    }

    /// 从 json 文件加载。
    pub fn 加载(路径: &str) -> Result<角色册, String> {
        let 文本 = std::fs::read_to_string(路径).map_err(|错误| format!("读取角色册失败: {错误}"))?;
        let 角色们: HashMap<String, 执行角色> =
            serde_json::from_str(&文本).map_err(|错误| format!("解析角色册失败: {错误}"))?;
        Ok(角色册 { 角色们 })
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 造角色(身份: &str) -> 执行角色 {
        执行角色 {
            身份: 身份.to_string(),
            道: "代码".to_string(),
            职司: "实现".to_string(),
            模型池: "executor".to_string(),
            契约: "写代码".to_string(),
        }
    }

    #[test]
    fn 登记与取用() {
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝"));
        assert_eq!(册.数量(), 1);
        assert_eq!(册.取("多宝").unwrap().道, "代码");
    }
}
