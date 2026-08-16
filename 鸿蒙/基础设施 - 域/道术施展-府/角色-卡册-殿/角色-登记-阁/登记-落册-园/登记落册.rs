//! 登记 - 落册 - 园：角色卡登记落册，供工作流引擎按职司调取。

use crate::类型_定义_殿::执行角色;
use rizhi_fu::{debug, error, info};
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
        debug!(身份 = %角色.身份, "角色已登记");
        self.角色们.insert(角色.身份.clone(), 角色);
    }

    /// 卸载一张角色卡（生命周期：卸载）。返回被卸载的角色，未登记返回 None。
    /// 本质：任何插件系统的卸载能力，登记不再「即永久」。
    pub fn 卸载(&mut self, 身份: &str) -> Option<执行角色> {
        let 角色 = self.角色们.remove(身份);
        if 角色.is_some() {
            info!(身份, "角色已卸载");
        }
        角色
    }

    /// 依赖校验：角色声明的模型池不在可用池中则返回缺失的池名（生命周期：就绪校验）。
    /// 本质：任何插件系统的「声明依赖 + 就绪校验」，防挂载了模型池却不可用的角色。
    /// 未声明依赖（模型池为空）视为无依赖，返回 None。
    pub fn 缺失模型池(&self, 身份: &str, 可用模型池: &[&str]) -> Option<String> {
        let 角色 = self.角色们.get(身份)?;
        if 角色.模型池.is_empty() {
            return None;
        }
        if 可用模型池.iter().any(|池| 池 == &角色.模型池) {
            None
        } else {
            Some(角色.模型池.clone())
        }
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
        std::fs::write(路径, 文本).map_err(|错误| {
            error!(路径, "保存角色册失败：{错误}");
            format!("保存角色册失败: {错误}")
        })?;
        debug!(路径, "角色册已保存");
        Ok(())
    }

    /// 从 json 文件加载。
    pub fn 加载(路径: &str) -> Result<角色册, String> {
        let 文本 = std::fs::read_to_string(路径).map_err(|错误| {
            error!(路径, "读取角色册失败：{错误}");
            format!("读取角色册失败: {错误}")
        })?;
        let 角色们: HashMap<String, 执行角色> =
            serde_json::from_str(&文本).map_err(|错误| format!("解析角色册失败: {错误}"))?;
        info!(路径, 角色数 = 角色们.len(), "角色册已加载");
        Ok(角色册 { 角色们 })
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::类型_定义_殿::执行角色;

    fn 造角色(身份: &str, 模型池: &str) -> 执行角色 {
        执行角色 {
            身份: 身份.to_string(),
            道: "代码炼化".to_string(),
            职司: "代码".to_string(),
            模型池: 模型池.to_string(),
            契约: "写代码".to_string(),
        }
    }

    #[test]
    fn 卸载_登记后卸载即取不到() {
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝", "executor"));
        assert_eq!(册.数量(), 1);
        let 卸 = 册.卸载("多宝");
        assert!(卸.is_some(), "应返回被卸载的角色");
        assert_eq!(册.数量(), 0);
        assert!(册.取("多宝").is_none(), "卸载后取不到");
    }

    #[test]
    fn 缺失模型池_依赖校验() {
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝", "executor"));
        // 可用池含 executor → 无缺失。
        assert!(册.缺失模型池("多宝", &["executor", "sage"]).is_none());
        // 可用池不含 executor → 缺失 executor。
        assert_eq!(册.缺失模型池("多宝", &["sage"]).as_deref(), Some("executor"));
        // 未登记 → None（不误报缺失）。
        assert!(册.缺失模型池("不存在", &["executor"]).is_none());
    }

    #[test]
    fn 缺失模型池_无声明依赖视为无依赖() {
        let mut 册 = 角色册::新();
        册.登记(造角色("无名", ""));
        assert!(册.缺失模型池("无名", &[]).is_none(), "模型池为空视为无依赖");
    }
}

