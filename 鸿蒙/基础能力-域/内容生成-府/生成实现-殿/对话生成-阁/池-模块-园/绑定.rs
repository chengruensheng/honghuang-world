use std::path::PathBuf;
use std::sync::Arc;

use hm_content_contract::{内容生成器, 工具对话器, 流式对话器, 对话消息, 模型响应};
use hm_contract::Component;
use hm_error::{Error, Result};

use super::池模块::LLM池;
use super::类型::池选择;

/// 可绑定独立模型的智能体身份清单（绑定接口校验 + 前端展示的唯一来源）
pub const 智能体身份: &[&str] = &["道祖", "执行"];

/// 绑定文件 JSON 键常量（防硬编码告警，统一键名来源）
const 键_智能体: &str = "智能体";
const 键_供应商: &str = "供应商";
const 键_模型: &str = "模型";
const 绑定文件_名: &str = "llm-绑定.json";

/// 智能体绑定条目（清单接口响应；None = 跟随池全局选择）
#[derive(Debug, Clone)]
pub struct 智能体绑定 {
    pub 名: String,
    pub 绑定: Option<池选择>,
}

/// 绑定视图：实现三 trait 的按智能体分发视图。
/// 装配期注入各消费方（道祖接待/开发受理台/看板驱动台），运行期每次调用动态解析
/// 绑定 ?? 全局选择——UI 改绑定即时生效，无需重装配。
pub struct 绑定视图 {
    池: Arc<LLM池>,
    智能体: String,
}

impl 绑定视图 {
    /// 本智能体生效选择：绑定优先，否则回退池全局选择
    fn 生效选择(&self) -> 池选择 {
        match self.池.解析绑定(&self.智能体) {
            Some(选) => {
                tracing::info!(
                    "智能体 {} 绑定路由 → {}/{}",
                    self.智能体, 选.供应商, 选.模型
                );
                选
            }
            None => self.池.全局选择(),
        }
    }
}

impl Component for 绑定视图 {
    fn name(&self) -> &'static str { "绑定视图" }
}

impl 内容生成器 for 绑定视图 {
    fn 生成(&self, 提示词: String) -> Result<String> {
        let 起 = self.生效选择();
        self.池.生成_起(&起, 提示词)
    }
}

impl 工具对话器 for 绑定视图 {
    fn 对话(&self, 消息: Vec<对话消息>, 工具: Vec<serde_json::Value>) -> Result<模型响应> {
        let 起 = self.生效选择();
        self.池.对话_起(&起, 消息, 工具)
    }
}

impl 流式对话器 for 绑定视图 {
    fn 对话流式(
        &self,
        消息: Vec<对话消息>,
        工具: Vec<serde_json::Value>,
        on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), Error>,
    ) -> Result<模型响应> {
        let 起 = self.生效选择();
        self.池.对话流式_起(&起, 消息, 工具, on_chunk)
    }
}

impl LLM池 {
    /// 绑定视图工厂：为智能体身份构造分发视图（Arc 廉价克隆，绑定动态解析）
    pub fn 绑定视图(self: &Arc<Self>, 智能体: &str) -> Arc<绑定视图> {
        Arc::new(绑定视图 { 池: self.clone(), 智能体: 智能体.to_string() })
    }

    /// 为智能体绑定独立模型：校验身份合法 + 供应商在池内；成功落盘绑定文件
    pub fn 绑定智能体(&self, 智能体: &str, 供应商: &str, 模型: &str) -> Result<()> {
        let 身份 = 智能体.trim();
        let 供应 = 供应商.trim();
        let 模型值 = 模型.trim();
        if !智能体身份.contains(&身份) {
            return Err(Error::模型(format!(
                "未知智能体身份: {身份}（可选: {}）",
                智能体身份.join(", ")
            )));
        }
        if 供应.is_empty() || 模型值.is_empty() {
            return Err(Error::模型("绑定模型：供应商/模型 必填".into()));
        }
        let 在池 = self
            .供应商们
            .lock()
            .map_err(|_| Error::模型("LLM 池供应商锁中毒".into()))?
            .iter()
            .any(|s| s.名 == 供应);
        if !在池 {
            return Err(Error::模型(format!("LLM 供应商不存在: {供应}")));
        }
        {
            let mut 锁 = self
                .绑定
                .lock()
                .map_err(|_| Error::模型("LLM 池绑定锁中毒".into()))?;
            锁.insert(身份.into(), 池选择 { 供应商: 供应.into(), 模型: 模型值.into() });
        }
        if let Err(失败) = self.保存绑定文件() {
            tracing::warn!("保存 LLM 绑定失败: {失败}");
        }
        tracing::info!("智能体 {身份} 已绑定独立模型 {供应}/{模型值}");
        Ok(())
    }

    /// 解除智能体绑定（回退池全局选择）；未绑定时幂等成功
    pub fn 解绑智能体(&self, 智能体: &str) -> Result<()> {
        {
            let mut 锁 = self
                .绑定
                .lock()
                .map_err(|_| Error::模型("LLM 池绑定锁中毒".into()))?;
            锁.remove(智能体.trim());
        }
        if let Err(失败) = self.保存绑定文件() {
            tracing::warn!("保存 LLM 绑定失败: {失败}");
        }
        tracing::info!("智能体 {} 已解绑（回退全局选择）", 智能体.trim());
        Ok(())
    }

    /// 绑定清单：身份清单 × 当前绑定（供状态接口展示）
    pub fn 绑定清单(&self) -> Vec<智能体绑定> {
        智能体身份
            .iter()
            .map(|名| 智能体绑定 { 名: (*名).into(), 绑定: self.解析绑定(名) })
            .collect()
    }

    /// 解析绑定（无绑定返回 None）
    pub(super) fn 解析绑定(&self, 智能体: &str) -> Option<池选择> {
        self.绑定.lock().ok()?.get(智能体).cloned()
    }

    /// 绑定文件路径：与选择文件同目录（persistence.dir），无状态文件 → 仅运行时
    fn 绑定文件(&self) -> Option<PathBuf> {
        let 状态 = self.状态文件.as_ref()?;
        let 父 = 状态.parent()?;
        Some(父.join(绑定文件_名))
    }

    /// 从绑定文件恢复（启动时调用；未知身份/坏行跳过并告警）
    pub(super) fn 加载绑定文件(&self) -> Result<()> {
        let Some(路径) = self.绑定文件() else { return Ok(()) };
        if !路径.exists() {
            return Ok(());
        }
        let 文本 = std::fs::read_to_string(&路径).map_err(Error::Io)?;
        let 值: serde_json::Value = match serde_json::from_str(&文本) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("LLM 绑定文件损坏，忽略: {e}");
                return Ok(());
            }
        };
        let Some(数组) = 值.as_array() else { return Ok(()) };
        let mut 表 = self
            .绑定
            .lock()
            .map_err(|_| Error::模型("LLM 池绑定锁中毒".into()))?;
        for 条目 in 数组 {
            let Some(名) = 条目[键_智能体].as_str() else { continue };
            if !智能体身份.contains(&名) {
                continue;
            }
            let Some(供应商) = 条目[键_供应商].as_str().filter(|s| !s.is_empty()) else {
                continue;
            };
            let Some(模型) = 条目[键_模型].as_str().filter(|s| !s.is_empty()) else {
                continue;
            };
            表.insert(名.into(), 池选择 { 供应商: 供应商.into(), 模型: 模型.into() });
        }
        tracing::info!("LLM 绑定文件已恢复（{} 条绑定）", 表.len());
        Ok(())
    }

    /// 保存绑定文件（原子写：临时文件 + rename）
    fn 保存绑定文件(&self) -> Result<()> {
        let Some(路径) = self.绑定文件() else { return Ok(()) };
        let 表 = self
            .绑定
            .lock()
            .map_err(|_| Error::模型("LLM 池绑定锁中毒".into()))?;
        let 数组: Vec<serde_json::Value> = 表
            .iter()
            .map(|(名, 选)| {
                serde_json::json!({
                    键_智能体: 名,
                    键_供应商: 选.供应商,
                    键_模型: 选.模型,
                })
            })
            .collect();
        drop(表);
        if let Some(父) = 路径.parent() {
            if !父.as_os_str().is_empty() {
                std::fs::create_dir_all(父).map_err(Error::Io)?;
            }
        }
        let 临时 = 路径.with_extension("json.tmp");
        std::fs::write(&临时, serde_json::json!(数组).to_string()).map_err(Error::Io)?;
        std::fs::rename(&临时, &路径).map_err(Error::Io)?;
        Ok(())
    }
}
