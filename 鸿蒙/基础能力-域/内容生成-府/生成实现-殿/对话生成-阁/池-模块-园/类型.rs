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