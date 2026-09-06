use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hm_config::LlmConfig;
use hm_content_contract::{内容生成器, 工具对话器, 对话消息, 模型响应};
use hm_contract::Component;
use hm_error::{Error, Result};
use serde_json::json;

use super::super::{消息转json, 解析工具调用};

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

/// 模型条目（list-models 返回的单个模型）
#[derive(Debug, Clone, serde::Serialize)]
pub struct 模型条目 {
    pub id: String,
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
    供应商们: Vec<池内供应商>,
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
            供应商们,
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
        !self.供应商们.is_empty()
    }

    /// 当前选择（供应商 + 模型）
    pub fn 当前选择(&self) -> Option<池选择> {
        let 锁 = self.选择.lock().ok()?;
        Some(锁.clone())
    }

    /// 切换选择：校验供应商存在；成功则落盘状态文件（尽力而为）。
    pub fn 选择(&self, 供应商: &str, 模型: &str) -> Result<()> {
        if !self.供应商们.iter().any(|s| s.名 == 供应商) {
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
            .iter()
            .map(|s| 供应商信息 {
                名称: s.名.clone(),
                类型: "openai".into(),
                地址: s.端点.clone(),
                模型: s.模型.clone(),
                超时秒: s.超时.as_secs(),
                启用: true,
            })
            .collect()
    }

    /// 列表模型：逐个调供应商 list-models API，失败供应商标注错误（不 panic、不阻断其余）
    pub fn 列表模型(&self) -> Vec<模型发现> {
        self.供应商们
            .iter()
            .map(|s| match s.列表请求() {
                Ok(ids) => 模型发现 {
                    供应商: s.名.clone(),
                    模型: Some(ids.into_iter().map(|id| 模型条目 { id }).collect()),
                    错误: None,
                },
                Err(e) => 模型发现 {
                    供应商: s.名.clone(),
                    模型: None,
                    错误: Some(e.to_string()),
                },
            })
            .collect()
    }

    /// 生成主路径：选中供应商优先，按配置顺序故障转移；全失败返回最后错误。
    fn 生成经池<T, 造, 析>(&self, 造体: &造, 解析: &析) -> Result<T>
    where
        造: Fn(&池内供应商) -> serde_json::Value,
        析: Fn(&serde_json::Value) -> Result<T>,
    {
        if self.供应商们.is_empty() {
            return Err(Error::模型("LLM 池无可用模型（未配置供应商）".into()));
        }
        // 锁中毒时按空选择处理（回退配置顺序第一供应商）
        let 当前 = self.当前选择().unwrap_or_else(|| 池选择 { 供应商: String::new(), 模型: String::new() });
        let mut 顺序: Vec<&池内供应商> = Vec::with_capacity(self.供应商们.len());
        // 选中供应商优先（含其选中模型），其余按配置顺序
        if let Some(选中) = self.供应商们.iter().find(|s| s.名 == 当前.供应商) {
            顺序.push(选中);
        }
        for s in &self.供应商们 {
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
        if !self.供应商们.iter().any(|s| s.名 == 供应商) {
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
        let resp = ureq::get(&self.列表端点)
            .set("Authorization", &format!("Bearer {}", self.密钥))
            .timeout(self.超时)
            .call()
            .map_err(|e| Error::模型(format!("列表模型请求失败: {e}")))?;
        let text = resp
            .into_string()
            .map_err(|e| Error::模型(format!("读取模型列表失败: {e}")))?;
        let 值: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| Error::模型(format!("解析模型列表失败: {e}")))?;
        let mut ids = Vec::new();
        if let Some(数据) = 值["data"].as_array() {
            for 条目 in 数据 {
                if let Some(id) = 条目["id"].as_str() {
                    ids.push(id.to_string());
                }
            }
        }
        if ids.is_empty() {
            return Err(Error::模型(format!("模型列表为空（端点 {}）", self.列表端点)));
        }
        Ok(ids)
    }
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
