use std::path::PathBuf;
use std::time::Duration;

use hm_error::{Error, Result};
use serde_json::json;

use super::池模块::LLM池;
use super::供应商::{池内供应商, 解析密钥};
use super::类型::池选择;

/// 接入文件 JSON 键常量（防硬编码告警，统一键名来源）
const 键_名称: &str = "名称";
const 键_类型: &str = "类型";
const 键_地址: &str = "地址";
const 键_密钥引用: &str = "密钥引用";
const 键_模型: &str = "模型";
const 键_重试: &str = "重试";
const 接入文件_名: &str = "llm-接入.json";

impl LLM池 {
    /// 当前选择（供应商 + 模型）
    pub fn 当前选择(&self) -> Option<池选择> {
        let 锁 = self.选择.lock().ok()?;
        Some(锁.clone())
    }

    /// 切换选择：校验供应商存在；成功则落盘状态文件（尽力而为）。
    pub fn 选择(&self, 供应商: &str, 模型: &str) -> Result<()> {
        if !self
            .供应商们
            .lock()
            .map_err(|_| Error::模型("LLM 池供应商锁中毒".into()))?
            .iter()
            .any(|s| s.名 == 供应商)
        {
            return Err(Error::模型(format!("LLM 供应商不存在: {供应商}")));
        }
        {
            let mut 锁 = self
                .选择
                .lock()
                .map_err(|_| Error::模型("LLM 池选择锁中毒".into()))?;
            锁.供应商 = 供应商.to_string();
            锁.模型 = 模型.to_string();
        }
        if let Err(失败) = self.保存选择文件() {
            tracing::warn!("保存 LLM 选择失败: {失败}");
        }
        tracing::info!("LLM 选择切换: {供应商} / {模型}");
        Ok(())
    }

    /// 从接入文件合并已持久化的运行时供应商（启动时调用；密钥引用解析失败跳过并 warn）
    pub fn 从接入文件合并(&self) -> Result<()> {
        let Some(路径) = self.接入文件() else { return Ok(()) };
        if !路径.exists() {
            return Ok(());
        }
        let 文本 = std::fs::read_to_string(&路径).map_err(Error::Io)?;
        let 值: serde_json::Value = match serde_json::from_str(&文本) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("LLM 接入文件损坏，忽略: {e}");
                return Ok(());
            }
        };
        let Some(数组) = 值.as_array() else { return Ok(()) };
        for 条目 in 数组 {
            let Some(名称) = 条目[键_名称].as_str() else { continue };
            let 密钥引用 = 条目[键_密钥引用].as_str().unwrap_or_default();
            let 密钥 = 解析密钥(密钥引用);
            if 密钥.is_empty() {
                tracing::warn!("接入文件供应商 {} 密钥无效（env 引用缺失），跳过", 名称);
                continue;
            }
            let 端点 = 条目["地址"].as_str().unwrap_or_default();
            let 模型 = 条目["模型"].as_str().unwrap_or_default();
            if 端点.is_empty() || 模型.is_empty() {
                continue;
            }
            let 供应商 = 池内供应商 {
                名: 名称.into(),
                密钥,
                端点: 端点.into(),
                列表端点: format!("{}/models", 端点.trim_end_matches('/')),
                模型: 模型.into(),
                超时: Duration::from_secs(
                    条目["超时秒"].as_u64().unwrap_or(30).max(1),
                ),
                重试: 条目[键_重试].as_u64().unwrap_or_else(|| 2) as u32,
                启用json模式: false, // 接入文件不持久化 json_mode，恢复时默认关闭
            };
            let mut 锁 = self
                .供应商们
                .lock()
                .map_err(|_| Error::模型("LLM 池供应商锁中毒".into()))?;
            if let Some(旧) = 锁.iter_mut().find(|s| s.名 == 名称) {
                *旧 = 供应商;
            } else {
                锁.push(供应商);
            }
        }
        tracing::info!("LLM 接入文件已合并（{} 条有效记录）", 数组.len());
        Ok(())
    }

    /// 接入文件路径：与选择文件同目录（persistence.dir），无状态文件 → 仅运行时
    pub(super) fn 接入文件(&self) -> Option<PathBuf> {
        let 状态 = self.状态文件.as_ref()?;
        let 父 = 状态.parent()?;
        Some(父.join(接入文件_名))
    }

    /// 保存接入文件（env 引用条目；去重后原子写）
    pub(super) fn 保存接入文件(&self, 供应商: &池内供应商, 密钥引用: &str) -> Result<()> {
        let Some(路径) = self.接入文件() else { return Ok(()) };
        let mut 数组: Vec<serde_json::Value> = Vec::new();
        if let Ok(文本) = std::fs::read_to_string(&路径) {
            if let Ok(值) = serde_json::from_str::<serde_json::Value>(&文本) {
                if let Some(旧数组) = 值.as_array() {
                    数组 = 旧数组
                        .iter()
                        .filter(|旧| 旧[键_名称].as_str() != Some(供应商.名.as_str()))
                        .cloned()
                        .collect();
                }
            }
        }
        数组.push(json!({
            键_名称: 供应商.名,
            键_类型: "openai",
            键_地址: 供应商.端点,
            键_密钥引用: 密钥引用,
            键_模型: 供应商.模型,
            "超时秒": 供应商.超时.as_secs(),
            键_重试: 供应商.重试,
        }));
        if let Some(父) = 路径.parent() {
            if !父.as_os_str().is_empty() {
                std::fs::create_dir_all(父).map_err(Error::Io)?;
            }
        }
        let 临时 = 路径.with_extension("json.tmp");
        std::fs::write(&临时, json!(数组).to_string()).map_err(Error::Io)?;
        std::fs::rename(&临时, &路径).map_err(Error::Io)?;
        Ok(())
    }

    pub(super) fn 保存选择文件(&self) -> Result<()> {
        let Some(路径) = &self.状态文件 else { return Ok(()) };
        // 锁中毒时按空选择处理（落盘空选择文件，重启回退配置默认）
        let 选择 = self.当前选择().unwrap_or_else(|| 池选择 { 供应商: String::new(), 模型: String::new() });
        let 内容 = json!({ "供应商": 选择.供应商, "模型": 选择.模型 }).to_string();
        if let Some(父) = 路径.parent() {
            if !父.as_os_str().is_empty() {
                std::fs::create_dir_all(父).map_err(Error::Io)?;
            }
        }
        // 原子写：临时文件 + rename
        let 临时 = 路径.with_extension("json.tmp");
        std::fs::write(&临时, 内容).map_err(Error::Io)?;
        std::fs::rename(&临时, 路径).map_err(Error::Io)?;
        Ok(())
    }

    pub(super) fn 加载选择文件(&self) -> Result<()> {
        let Some(路径) = &self.状态文件 else { return Ok(()) };
        if !路径.exists() {
            return Ok(());
        }
        let 文本 = std::fs::read_to_string(路径).map_err(Error::Io)?;
        let 值: serde_json::Value = serde_json::from_str(&文本).map_err(|e| Error::反序列化(e.to_string()))?;
        let 供应商 = 值["供应商"].as_str().unwrap_or_default().to_string();
        let 模型 = 值["模型"].as_str().unwrap_or_default().to_string();
        if 供应商.is_empty() {
            return Ok(());
        }
        if !self
            .供应商们
            .lock()
            .map_err(|_| Error::模型("LLM 池供应商锁中毒".into()))?
            .iter()
            .any(|s| s.名 == 供应商)
        {
            tracing::warn!("选择文件供应商 {} 不在池内，忽略", 供应商);
            return Ok(());
        }
        {
            let mut 锁 = self.选择.lock().map_err(|_| Error::模型("LLM 池选择锁中毒".into()))?;
            锁.供应商 = 供应商;
            if !模型.is_empty() {
                锁.模型 = 模型;
            }
        }
        Ok(())
    }
}
