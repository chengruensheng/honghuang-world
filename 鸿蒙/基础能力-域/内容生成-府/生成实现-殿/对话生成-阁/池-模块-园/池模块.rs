use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hm_config::LlmConfig;
use hm_content_contract::{内容生成器, 工具对话器, 对话消息, 模型响应};
use hm_contract::Component;
use hm_error::{Error, Result};
use serde_json::json;

use super::super::{消息转json, 解析工具调用};

/// 接入文件 JSON 键常量（防硬编码告警，统一键名来源）
const 键_名称: &str = "名称";
const 键_类型: &str = "类型";
const 键_地址: &str = "地址";
const 键_密钥引用: &str = "密钥引用";
const 键_模型: &str = "模型";
const 键_重试: &str = "重试";
const 接入文件_名: &str = "llm-接入.json";

/// 运行时选择：当前生效的 供应商 + 模型（全池共享，受理台/看板驱动台 同时生效）
#[derive(Debug, Clone)]

pub struct 池选择 {
    pub 供应商: String,
    pub 模型: String,
}

/// 供应商信息（脱敏：不含 api_key，供状态接口展示）
#[derive(Debug, Clone, serde::Serialize)]
pub struct 供应商信息 {
    pub 名称: String,
    pub 类型: String,
    pub 地址: String,
    pub 模型: String,
    pub 超时秒: u64,
    pub 启用: bool,
}

/// 模型条目（list-models 返回的单个模型；名称/上下文窗/最大输出为可选能力字段）
#[derive(Debug, Clone, serde::Serialize)]
pub struct 模型条目 {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub 名称: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub 上下文窗: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub 最大输出: Option<u64>,
}

impl 模型条目 {
    pub fn 仅id(id: String) -> Self {
        模型条目 { id, 名称: None, 上下文窗: None, 最大输出: None }
    }
}

/// 内置供应商模板（目录权威：目录命中直接返回默认模型，零网络）
#[derive(Debug, Clone, serde::Serialize)]
pub struct 模板 {
    pub 名称: String,
    pub 显示名: String,
    pub 地址: String,
    pub 环境变量: String,
    pub 默认模型: Vec<String>,
}

/// 热门供应商模板（OpenAI 兼容端点，第一版 9 家；环境变量为约定名，供接入提示）
pub fn 热门模板() -> Vec<模板> {
    vec![
        模板 { 名称: "deepseek".into(), 显示名: "DeepSeek".into(), 地址: "https://api.deepseek.com".into(), 环境变量: "DEEPSEEK_API_KEY".into(), 默认模型: vec!["deepseek-chat".into(), "deepseek-reasoner".into()] },
        模板 { 名称: "zhipu".into(), 显示名: "智谱 GLM".into(), 地址: "https://open.bigmodel.cn/api/paas/v4".into(), 环境变量: "ZHIPU_API_KEY".into(), 默认模型: vec!["glm-4-plus".into(), "glm-4-flash".into()] },
        模板 { 名称: "qwen".into(), 显示名: "通义 Qwen".into(), 地址: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(), 环境变量: "DASHSCOPE_API_KEY".into(), 默认模型: vec!["qwen-plus".into(), "qwen-turbo".into()] },
        模板 { 名称: "kimi".into(), 显示名: "Kimi".into(), 地址: "https://api.moonshot.cn/v1".into(), 环境变量: "MOONSHOT_API_KEY".into(), 默认模型: vec!["moonshot-v1-32k".into(), "moonshot-v1-8k".into()] },
        模板 { 名称: "minimax".into(), 显示名: "MiniMax".into(), 地址: "https://api.minimax.chat/v1".into(), 环境变量: "LLM_API_KEY".into(), 默认模型: vec!["abab6.5s-chat".into(), "abab6.5g-chat".into()] },
        模板 { 名称: "volcengine".into(), 显示名: "火山方舟(豆包)".into(), 地址: "https://ark.cn-beijing.volces.com/api/v3".into(), 环境变量: "ARK_API_KEY".into(), 默认模型: vec!["doubao-1-5-pro-32k-250115".into(), "doubao-1-5-lite-32k-250115".into()] },
        模板 { 名称: "openai".into(), 显示名: "OpenAI".into(), 地址: "https://api.openai.com/v1".into(), 环境变量: "OPENAI_API_KEY".into(), 默认模型: vec!["gpt-4o".into(), "gpt-4o-mini".into()] },
        模板 { 名称: "openrouter".into(), 显示名: "OpenRouter".into(), 地址: "https://openrouter.ai/api/v1".into(), 环境变量: "OPENROUTER_API_KEY".into(), 默认模型: vec!["openai/gpt-4o".into(), "anthropic/claude-3-5-sonnet".into()] },
        模板 { 名称: "ollama".into(), 显示名: "Ollama(本地)".into(), 地址: "http://127.0.0.1:11434/v1".into(), 环境变量: String::new(), 默认模型: vec!["llama3.1".into(), "qwen2.5".into()] },
    ]
}

/// 模板的默认模型（目录命中时零网络返回）
pub fn 模板模型(名称: &str) -> Option<Vec<模型条目>> {
    热门模板()
        .into_iter()
        .find(|t| t.名称 == 名称)
        .map(|t| t.默认模型.into_iter().map(模型条目::仅id).collect())
}

/// 模型发现结果：单个供应商的列表模型结果（失败标注错误，不 panic）
#[derive(Debug, Clone, serde::Serialize)]
pub struct 模型发现 {
    pub 供应商: String,
    pub 模型: Option<Vec<模型条目>>,
    pub 错误: Option<String>,
}

/// 池内供应商：配置解析后的可调用单元
#[derive(Debug, Clone)]
struct 池内供应商 {
    名: String,
    密钥: String,
    端点: String,
    列表端点: String,
    模型: String,
    超时: Duration,
    重试: u32,
}

/// 商业级 LLM 池：多供应商（配置顺序即故障转移优先级）+ 运行时选择。
/// 实现 内容生成器 / 工具对话器，可无缝替代 对话生成器 装配进 智能体/驱动器。
pub struct LLM池 {
    /// 供应商表（接入向导需运行时增删 → Mutex；生成时锁内快照克隆）
    供应商们: Arc<Mutex<Vec<池内供应商>>>,
    选择: Arc<Mutex<池选择>>,
    状态文件: Option<PathBuf>,
}

impl LLM池 {
    /// 从配置构造：启用且密钥非空的供应商入池；空配置 → 空池（由调用方回退 从环境）。
    pub fn 从配置(配置: &LlmConfig) -> Self {
        let mut 供应商们 = Vec::new();
        for p in &配置.providers {
            if !p.enabled {
                continue;
            }
            let 密钥 = 解析密钥(&p.api_key);
            if 密钥.is_empty() {
                tracing::warn!("LLM 供应商 {} 无有效密钥（api_key 为空或 env 引用缺失），跳过", p.name);
                continue;
            }
            let 列表端点 = if p.models_url.is_empty() {
                format!("{}/models", p.base_url.trim_end_matches('/'))
            } else {
                p.models_url.clone()
            };
            供应商们.push(池内供应商 {
                名: p.name.clone(),
                密钥,
                端点: p.base_url.clone(),
                列表端点,
                模型: p.model.clone(),
                超时: Duration::from_secs(p.timeout_secs.max(1)),
                重试: p.retry,
            });
        }
        let mut 默认选择 = 池选择 {
            供应商: 配置.selected_provider.clone(),
            模型: 配置.selected_model.clone(),
        };
        if 默认选择.供应商.is_empty() {
            if let Some(首个) = 供应商们.first() {
                默认选择.供应商 = 首个.名.clone();
                默认选择.模型 = 首个.模型.clone();
            }
        }
        let 状态文件 = if 配置.state_file.is_empty() {
            None
        } else {
            Some(PathBuf::from(&配置.state_file))
        };
        let 池 = LLM池 {
            供应商们: Arc::new(Mutex::new(供应商们)),
            选择: Arc::new(Mutex::new(默认选择)),
            状态文件,
        };
        // 运行时选择文件（存在则覆盖配置默认；损坏忽略回退默认）
        let _ = 池.加载选择文件();
        池
    }

    /// 从环境变量构造（兼容 v1.43 语义）：LLM_API_KEY/BASE_URL/MODEL 必填 fail-loud，
    /// LLM_FALLBACK_* 三项齐全则作为第二供应商（故障转移）。
    pub fn 从环境() -> Result<Self> {
        let _ = dotenvy::dotenv();
        let api_key = std::env::var("LLM_API_KEY")
            .map_err(|_| Error::Config("缺少 LLM_API_KEY，请在环境密钥文件中配置".into()))?;
        let base_url = std::env::var("LLM_BASE_URL")
            .map_err(|_| Error::Config("缺少 LLM_BASE_URL，请配置环境变量".into()))?;
        let model = std::env::var("LLM_MODEL")
            .map_err(|_| Error::Config("缺少 LLM_MODEL，请配置环境变量".into()))?;
        let mut 提供 = vec![LlmProvider简易 {
            name: "主".into(),
            base_url,
            api_key,
            model,
        }];
        if let (Ok(备密钥), Ok(备地址), Ok(备模型)) = (
            std::env::var("LLM_FALLBACK_API_KEY"),
            std::env::var("LLM_FALLBACK_BASE_URL"),
            std::env::var("LLM_FALLBACK_MODEL"),
        ) {
            提供.push(LlmProvider简易 {
                name: "备选".into(),
                base_url: 备地址,
                api_key: 备密钥,
                model: 备模型,
            });
        }
        let 配置 = LlmConfig {
            providers: 提供
                .into_iter()
                .map(|p| hm_config::LlmProvider {
                    name: p.name,
                    kind: "openai".into(),
                    base_url: p.base_url,
                    models_url: String::new(),
                    api_key: p.api_key,
                    model: p.model,
                    timeout_secs: 30,
                    retry: 2,
                    enabled: true,
                })
                .collect(),
            selected_provider: String::new(),
            selected_model: String::new(),
            state_file: String::new(),
        };
        Ok(Self::从配置(&配置))
    }

    /// 是否有可用供应商
    pub fn 可用(&self) -> bool {
        self.供应商们
            .lock()
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

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
        let _ = self.保存选择文件();
        tracing::info!("LLM 选择切换: {供应商} / {模型}");
        Ok(())
    }

    /// 供应商清单（脱敏：不含密钥）
    pub fn 供应商清单(&self) -> Vec<供应商信息> {
        self.供应商们
            .lock()
            .map(|锁| {
                锁.iter()
                    .map(|s| 供应商信息 {
                        名称: s.名.clone(),
                        类型: "openai".into(),
                        地址: s.端点.clone(),
                        模型: s.模型.clone(),
                        超时秒: s.超时.as_secs(),
                        启用: true,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 列表模型：逐个调供应商 list-models API，失败供应商标注错误（不 panic、不阻断其余）
    pub fn 列表模型(&self) -> Vec<模型发现> {
        self.供应商们
            .lock()
            .map(|锁| {
                锁.iter()
                    .map(|s| match s.列表请求() {
                        Ok(ids) => 模型发现 {
                            供应商: s.名.clone(),
                            模型: Some(ids.into_iter().map(模型条目::仅id).collect()),
                            错误: None,
                        },
                        Err(e) => 模型发现 {
                            供应商: s.名.clone(),
                            模型: None,
                            错误: Some(e.to_string()),
                        },
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 按名找池内供应商（探测模型时回退已存密钥）
    pub fn 按名找供应商(&self, 名称: &str) -> Option<(String, String)> {
        self.供应商们
            .lock()
            .ok()?
            .iter()
            .find(|s| s.名 == 名称)
            .map(|s| (s.端点.clone(), s.密钥.clone()))
    }

    /// 探测模型（dsh「获取可用模型」语义）：目录命中 → 模板默认模型（零网络）；
    /// 否则网络 GET {地址}/models。密钥三级：请求内密钥 > 池内已存密钥 > 无鉴权。
    pub fn 探测模型(
        &self,
        供应商: Option<&str>,
        地址: Option<&str>,
        密钥: Option<&str>,
    ) -> Result<(String, Vec<模型条目>)> {
        if let Some(名) = 供应商 {
            if let Some(模型) = 模板模型(名) {
                return Ok(("目录".into(), 模型));
            }
            if let Some((已存地址, 已存密钥)) = self.按名找供应商(名) {
                let 实际地址 = 地址
                    .filter(|a| !a.is_empty())
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| 已存地址);
                let 实际密钥 = 密钥
                    .filter(|k| !k.is_empty())
                    .map(|k| k.to_string())
                    .or_else(|| Some(已存密钥));
                let 列表端点 = format!("{}/models", 实际地址.trim_end_matches('/'));
                return 网络探测模型(&列表端点, 实际密钥.as_deref());
            }
        }
        let Some(地址值) = 地址.filter(|a| !a.is_empty()) else {
            return Err(Error::模型("模型探测需提供 base_url 或选择内置模板".into()));
        };
        let 实际密钥 = 密钥.filter(|k| !k.is_empty()).map(|k| k.to_string());
        let 列表端点 = format!("{}/models", 地址值.trim_end_matches('/'));
        网络探测模型(&列表端点, 实际密钥.as_deref())
    }

    /// 接入供应商：注册进池（重名覆盖）+ 选中 + env 引用落盘接入文件（明文密钥永不落盘）。
    /// 返回 (新选择, TOML 配置片段)。
    pub fn 接入(&self, 名称: &str, 地址: &str, 密钥: &str, 模型: &str) -> Result<(池选择, String)> {
        let 名 = 名称.trim();
        let 端点 = 地址.trim();
        let 模型值 = 模型.trim();
        if 名.is_empty() || 端点.is_empty() || 模型值.is_empty() {
            return Err(Error::模型("接入供应商：名称/地址/模型 必填".into()));
        }
        let 解析后 = 解析密钥(密钥);
        if 解析后.is_empty() {
            return Err(Error::模型(format!(
                "供应商 {名} 密钥无效（为空或 env 引用缺失），请检查密钥"
            )));
        }
        let 供应商 = 池内供应商 {
            名: 名.into(),
            密钥: 解析后,
            端点: 端点.into(),
            列表端点: format!("{}/models", 端点.trim_end_matches('/')),
            模型: 模型值.into(),
            超时: Duration::from_secs(30),
            重试: 2,
        };
        {
            let mut 锁 = self
                .供应商们
                .lock()
                .map_err(|_| Error::模型("LLM 池供应商锁中毒".into()))?;
            if let Some(旧) = 锁.iter_mut().find(|s| s.名 == 名) {
                *旧 = 供应商.clone();
            } else {
                锁.push(供应商.clone());
            }
        }
        {
            let mut 锁 = self
                .选择
                .lock()
                .map_err(|_| Error::模型("LLM 池选择锁中毒".into()))?;
            锁.供应商 = 名.into();
            锁.模型 = 模型值.into();
        }
        // 密钥引用（env:）→ 落盘接入文件（密钥不入库铁律：明文永不写入）
        if 密钥.starts_with("env:") {
            self.保存接入文件(&供应商, 密钥)?;
        }
        let _ = self.保存选择文件();
        let 环境变量 = 密钥.strip_prefix("env:").unwrap_or_else(|| 密钥);
        let 片段 = format!(
            "[[llm.providers]]
name = \"{名}\"
kind = \"openai\"
base_url = \"{端点}\"
# models_url = \"\"
api_key = \"env:{环境变量}\"  # 请将密钥写入 .env，重启后池自动恢复
model = \"{模型值}\"
timeout_secs = 30
retry = 2
enabled = true",
            名 = 名,
            端点 = 端点,
            模型值 = 模型值,
            环境变量 = 环境变量,
        );
        tracing::info!(
            "LLM 供应商接入: {名} / {模型值}（{}）",
            if 密钥.starts_with("env:") { "env 引用，已落盘" } else { "明文，仅会话内有效" }
        );
        Ok((池选择 { 供应商: 名.into(), 模型: 模型值.into() }, 片段))
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
    fn 接入文件(&self) -> Option<PathBuf> {
        let 状态 = self.状态文件.as_ref()?;
        let 父 = 状态.parent()?;
        Some(父.join(接入文件_名))
    }

    /// 保存接入文件（env 引用条目；去重后原子写）
    fn 保存接入文件(&self, 供应商: &池内供应商, 密钥引用: &str) -> Result<()> {
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

    /// 生成主路径：选中供应商优先，按配置顺序故障转移；全失败返回最后错误。
    fn 生成经池<T, 造, 析>(&self, 造体: &造, 解析: &析) -> Result<T>
    where
        造: Fn(&池内供应商) -> serde_json::Value,
        析: Fn(&serde_json::Value) -> Result<T>,
    {
        let 供应商们 = self
            .供应商们
            .lock()
            .map_err(|_| Error::模型("LLM 池供应商锁中毒".into()))?
            .clone();
        if 供应商们.is_empty() {
            return Err(Error::模型("LLM 池无可用模型（未配置供应商）".into()));
        }
        // 锁中毒时按空选择处理（回退配置顺序第一供应商）
        let 当前 = self.当前选择().unwrap_or_else(|| 池选择 { 供应商: String::new(), 模型: String::new() });
        let mut 顺序: Vec<&池内供应商> = Vec::with_capacity(供应商们.len());
        // 选中供应商优先（含其选中模型），其余按配置顺序
        if let Some(选中) = 供应商们.iter().find(|s| s.名 == 当前.供应商) {
            顺序.push(选中);
        }
        for s in &供应商们 {
            if s.名 != 当前.供应商 {
                顺序.push(s);
            }
        }
        let mut 最后错误: Option<Error> = None;
        for 供应商 in 顺序 {
            let 实际模型 = if 供应商.名 == 当前.供应商 && !当前.模型.is_empty() {
                当前.模型.clone()
            } else {
                供应商.模型.clone()
            };
            let 请求供应商 = 池内供应商 { 模型: 实际模型, ..供应商.clone() };
            match self.请求解析(&请求供应商, &造体(&请求供应商), 解析) {
                Ok(值) => return Ok(值),
                Err(错误) => {
                    最后错误 = Some(错误);
                    tracing::warn!("LLM 供应商 {} 请求失败，尝试转移下一供应商", 供应商.名);
                }
            }
        }
        Err(最后错误.unwrap_or_else(|| Error::模型("LLM 池生成失败".into())))
    }

    /// 单供应商：请求 + 重试 + 解析
    fn 请求解析<T, 析>(&self, 供应商: &池内供应商, body: &serde_json::Value, 解析: &析) -> Result<T>
    where
        析: Fn(&serde_json::Value) -> Result<T>,
    {
        let mut 最后一次错误: Option<Error> = None;
        for 尝试 in 0..=供应商.重试 {
            if 尝试 > 0 {
                std::thread::sleep(Duration::from_secs(1));
            }
            match 供应商.请求(body) {
                Ok(值) => return 解析(&值),
                Err(错误) => 最后一次错误 = Some(错误),
            }
        }
        Err(最后一次错误.unwrap_or_else(|| Error::模型("请求模型失败".into())))
    }

    fn 保存选择文件(&self) -> Result<()> {
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

    fn 加载选择文件(&self) -> Result<()> {
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

impl Component for LLM池 {
    fn name(&self) -> &'static str { "LLM池" }
}

impl 内容生成器 for LLM池 {
    fn 生成(&self, 提示词: String) -> Result<String> {
        let 造体 = |供应商: &池内供应商| json!({
            "model": &供应商.模型,
            "messages": [{ "role": "user", "content": 提示词.as_str() }],
        });
        let 解析 = |值: &serde_json::Value| {
            值["choices"][0]["message"]["content"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| Error::模型("模型响应缺少 choices[0].message.content".into()))
        };
        self.生成经池(&造体, &解析)
    }
}

impl 工具对话器 for LLM池 {
    fn 对话(&self, 消息: Vec<对话消息>, 工具: Vec<serde_json::Value>) -> Result<模型响应> {
        let 消息json: Vec<serde_json::Value> = 消息.iter().map(消息转json).collect();
        let 造体 = |供应商: &池内供应商| json!({
            "model": &供应商.模型,
            "messages": &消息json,
            "tools": &工具,
        });
        let 解析 = |值: &serde_json::Value| {
            let message = &值["choices"][0]["message"];
            Ok(模型响应 {
                内容: message["content"].as_str().map(|s| s.to_string()),
                工具调用: 解析工具调用(message),
            })
        };
        self.生成经池(&造体, &解析)
    }
}

impl 池内供应商 {
    /// 单次 chat/completions 请求
    fn 请求(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
        let resp = ureq::post(&self.端点)
            .set("Authorization", &format!("Bearer {}", self.密钥))
            .set("Content-Type", "application/json")
            .timeout(self.超时)
            .send_string(&body.to_string())
            .map_err(|e| Error::模型(format!("请求模型失败: {e}")))?;
        let text = resp
            .into_string()
            .map_err(|e| Error::模型(format!("读取模型响应失败: {e}")))?;
        serde_json::from_str(&text).map_err(|e| Error::模型(format!("解析模型响应失败: {e}")))
    }

    /// list-models 请求：GET {列表端点} → {"data":[{"id":"..."}]}
    fn 列表请求(&self) -> Result<Vec<String>> {
        Ok(网络探测模型(&self.列表端点, Some(&self.密钥))?
            .1
            .into_iter()
            .map(|m| m.id)
            .collect())
    }
}

/// 网络探测模型：GET {列表端点}（OpenAI 兼容 /models）。
/// 4MB 响应上限（content-length 预检 + 读后长度复核）、401/403 提示检查密钥、
/// data 非数组报错（提示手输）、坏行（无 id）跳过不整段失败。
pub fn 网络探测模型(列表端点: &str, 密钥: Option<&str>) -> Result<(String, Vec<模型条目>)> {
    let mut 请求 = ureq::get(列表端点).timeout(Duration::from_secs(15));
    if let Some(k) = 密钥 {
        if !k.is_empty() {
            请求 = 请求.set("Authorization", &format!("Bearer {k}"));
        }
    }
    let resp = match 请求.call() {
        Ok(r) => r,
        Err(e) => {
            let 文本 = e.to_string();
            if 文本.contains("401") || 文本.contains("403") {
                return Err(Error::模型(format!(
                    "模型发现失败（{列表端点} 拒绝访问）：检查 API 密钥"
                )));
            }
            return Err(Error::模型(format!("模型发现失败（{列表端点}）：{e}")));
        }
    };
    if let Some(长度) = resp.header("Content-Length") {
        if let Ok(n) = 长度.parse::<usize>() {
            if n > 4 * 1024 * 1024 {
                return Err(Error::模型(format!("模型列表响应过大（{n} 字节，上限 4MB）")));
            }
        }
    }
    let text = resp
        .into_string()
        .map_err(|e| Error::模型(format!("读取模型列表失败: {e}")))?;
    if text.len() > 4 * 1024 * 1024 {
        return Err(Error::模型("模型列表响应超过 4MB".into()));
    }
    let 值: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| Error::模型(format!("模型列表不是合法 JSON: {e}")))?;
    let Some(数据) = 值["data"].as_array() else {
        return Err(Error::模型("模型列表缺少 data 数组，请手动输入模型".into()));
    };
    let mut 条目们 = Vec::new();
    for 条目 in 数据 {
        let Some(id) = 条目["id"].as_str().map(|s| s.to_string()) else {
            continue;
        };
        let 名称 = 条目["name"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| 条目["display_name"].as_str().map(|s| s.to_string()));
        let 上下文窗 = 正数(&条目["context_window"]).or_else(|| 正数(&条目["context_length"]));
        let 最大输出 = 正数(&条目["max_output_tokens"]).or_else(|| 正数(&条目["max_tokens"]));
        条目们.push(模型条目 { id, 名称, 上下文窗, 最大输出 });
    }
    if 条目们.is_empty() {
        return Err(Error::模型(format!(
            "模型列表为空（{列表端点}），请手动输入模型"
        )));
    }
    Ok(("网络".into(), 条目们))
}

/// 正整数字段提取（能力字段辅助）
fn 正数(值: &serde_json::Value) -> Option<u64> {
    值.as_u64().filter(|n| *n > 0)
}

/// 解析 api_key 配置值：`env:变量名` 引用环境变量；其他视为直接值
pub fn 解析密钥(配置值: &str) -> String {
    if let Some(变量名) = 配置值.strip_prefix("env:") {
        std::env::var(变量名).unwrap_or_default()
    } else {
        配置值.to_string()
    }
}

/// 从环境构造用的简易中间结构（避免直接造 LlmProvider 长字面量）
struct LlmProvider简易 {
    name: String,
    base_url: String,
    api_key: String,
    model: String,
}
