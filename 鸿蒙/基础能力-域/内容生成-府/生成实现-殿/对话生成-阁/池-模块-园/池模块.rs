use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hm_config::LlmConfig;
use hm_content_contract::{内容生成器, 工具对话器, 流式对话器, 对话消息, 模型响应};
use hm_contract::Component;
use hm_error::{Error, Result};
use serde_json::json;

use super::super::消息转json;
use super::解析防线::{提取对话响应, 提取生成文本};
use super::供应商::{池内供应商, 解析密钥};
use super::类型::池选择;

/// 商业级 LLM 池：多供应商（配置顺序即故障转移优先级）+ 运行时选择。
/// 实现 内容生成器 / 工具对话器，可无缝替代 对话生成器 装配进 智能体/驱动器。
pub struct LLM池 {
    /// 供应商表（接入向导需运行时增删 → Mutex；生成时锁内快照克隆）
    pub(super) 供应商们: Arc<Mutex<Vec<池内供应商>>>,
    pub(super) 选择: Arc<Mutex<池选择>>,
    /// 智能体绑定表：智能体身份 → 指定模型选择（覆盖全局选择；详见 绑定.rs）
    pub(super) 绑定: Arc<Mutex<HashMap<String, 池选择>>>,
    pub(super) 状态文件: Option<PathBuf>,
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
                // base_url 可能是完整 chat/completions 端点或 API 根，归一化后拼 /models
                let 根 = p
                    .base_url
                    .trim_end_matches('/')
                    .trim_end_matches("/chat/completions");
                format!("{}/models", 根)
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
                最大输出tokens: p.max_tokens,
                启用json模式: p.json_mode,
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
            绑定: Arc::new(Mutex::new(HashMap::new())),
            状态文件,
        };
        // 运行时选择文件（存在则覆盖配置默认；损坏忽略回退默认）
        if let Err(失败) = 池.加载选择文件() {
            tracing::warn!("加载 LLM 选择文件失败（回退配置默认）: {失败}");
        }
        // 智能体绑定文件（存在则恢复每智能体独立模型）
        if let Err(失败) = 池.加载绑定文件() {
            tracing::warn!("加载 LLM 绑定文件失败（回退全局选择）: {失败}");
        }
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
                    json_mode: false,
                    max_tokens: 32768,
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

    /// 全局选择（锁中毒按空选择处理）
    pub(super) fn 全局选择(&self) -> 池选择 {
        self.当前选择()
            .unwrap_or_else(|| 池选择 { 供应商: String::new(), 模型: String::new() })
    }

    /// 生成主路径：起始选择供应商优先（含其选中模型），按配置顺序故障转移；全失败返回最后错误。
    fn 生成经池<T, 造, 析>(&self, 起始: &池选择, 造体: &造, 解析: &析) -> Result<T>
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
        let 当前 = 起始;
        let mut 顺序: Vec<&池内供应商> = Vec::with_capacity(供应商们.len());
        // 起始供应商优先（含其选中模型），其余按配置顺序
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
                    tracing::warn!("LLM 供应商 {} 请求失败，尝试转移下一供应商：{错误}", 供应商.名);
                    最后错误 = Some(错误);
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

    /// 生成（指定起始选择）：指定供应商优先故障转移，池与绑定视图共用。
    pub(super) fn 生成_起(&self, 起: &池选择, 提示词: String) -> Result<String> {
        let 造体 = |供应商: &池内供应商| {
            let mut body = json!({
                "model": &供应商.模型,
                "messages": [{ "role": "user", "content": 提示词.as_str() }],
                // 显式给足输出预算：推理模型思考链会吃光网关默认上限，致正文为空（见 配置模块 max_tokens 注释）
                "max_tokens": 供应商.最大输出tokens,
            });
            // JSON 输出模式：追加 response_format（默认关闭；未开启时请求体与旧版逐字节一致）
            if 供应商.启用json模式 {
                body["response_format"] = json!({ "type": "json_object" });
            }
            body
        };
        // 空内容/缺 content 在 解析防线 就地判败 → 故障转移下一家（见 解析防线.rs）
        self.生成经池(起, &造体, &|值: &serde_json::Value| 提取生成文本(值))
    }

    /// 工具对话（指定起始选择）：池与绑定视图共用。
    pub(super) fn 对话_起(
        &self,
        起: &池选择,
        消息: Vec<对话消息>,
        工具: Vec<serde_json::Value>,
    ) -> Result<模型响应> {
        let 消息json: Vec<serde_json::Value> = 消息.iter().map(消息转json).collect();
        let 造体 = |供应商: &池内供应商| json!({
            "model": &供应商.模型,
            "messages": &消息json,
            "tools": &工具,
            // 显式给足输出预算：推理模型思考链会吃光网关默认上限，致正文为空（见 配置模块 max_tokens 注释）
            "max_tokens": 供应商.最大输出tokens,
        });
        // 纯空包裹（内容/工具/思考全空）在 解析防线 就地判败 → 故障转移下一家；
        // 工具调用或思考非空的「空内容」是合法轮次，放行（见 解析防线.rs）
        self.生成经池(起, &造体, &|值: &serde_json::Value| 提取对话响应(值))
    }

    /// 流式对话（指定起始选择）：起始供应商优先故障转移，池与绑定视图共用。
    pub(super) fn 对话流式_起(
        &self,
        起: &池选择,
        消息: Vec<对话消息>,
        工具: Vec<serde_json::Value>,
        on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), Error>,
    ) -> Result<模型响应> {
        let 消息json: Vec<serde_json::Value> = 消息.iter().map(消息转json).collect();
        let 供应商们 = self
            .供应商们
            .lock()
            .map_err(|_| Error::模型("LLM 池供应商锁中毒".into()))?
            .clone();
        if 供应商们.is_empty() {
            return Err(Error::模型("LLM 池无可用模型（未配置供应商）".into()));
        }
        let 当前 = 起;
        let mut 顺序: Vec<&池内供应商> = Vec::with_capacity(供应商们.len());
        // 起始供应商优先（含其选中模型），其余按配置顺序
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
            match 请求供应商.流式对话(&消息json, &工具, on_chunk) {
                Ok(响应) => return Ok(响应),
                Err(错误) => {
                    tracing::warn!("LLM 供应商 {} 流式请求失败，尝试转移下一供应商：{错误}", 供应商.名);
                    最后错误 = Some(错误);
                }
            }
        }
        Err(最后错误.unwrap_or_else(|| Error::模型("LLM 池流式生成失败".into())))
    }

}

impl Component for LLM池 {
    fn name(&self) -> &'static str { "LLM池" }
}

impl 内容生成器 for LLM池 {
    fn 生成(&self, 提示词: String) -> Result<String> {
        let 起 = self.全局选择();
        self.生成_起(&起, 提示词)
    }
}

impl 工具对话器 for LLM池 {
    fn 对话(&self, 消息: Vec<对话消息>, 工具: Vec<serde_json::Value>) -> Result<模型响应> {
        let 起 = self.全局选择();
        self.对话_起(&起, 消息, 工具)
    }
}

impl 流式对话器 for LLM池 {
    fn 对话流式(
        &self,
        消息: Vec<对话消息>,
        工具: Vec<serde_json::Value>,
        on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), Error>,
    ) -> Result<模型响应> {
        let 起 = self.全局选择();
        self.对话流式_起(&起, 消息, 工具, on_chunk)
    }
}

/// 从环境构造用的简易中间结构（避免直接造 LlmProvider 长字面量）
struct LlmProvider简易 {
    name: String,
    base_url: String,
    api_key: String,
    model: String,
}
