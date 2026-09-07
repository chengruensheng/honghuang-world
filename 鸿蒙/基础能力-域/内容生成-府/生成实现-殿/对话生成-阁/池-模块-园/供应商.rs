use std::time::Duration;

use hm_error::{Error, Result};

use super::池模块::LLM池;
use super::类型::{模型条目, 供应商信息, 模型发现, 池选择, 模板模型};

/// 池内供应商：配置解析后的可调用单元
#[derive(Debug, Clone)]
pub(super) struct 池内供应商 {
    pub(super) 名: String,
    pub(super) 密钥: String,
    pub(super) 端点: String,
    pub(super) 列表端点: String,
    pub(super) 模型: String,
    pub(super) 超时: Duration,
    pub(super) 重试: u32,
    /// JSON 输出模式（生成请求追加 response_format: json_object；带工具「对话」不生效）
    pub(super) 启用json模式: bool,
}

impl 池内供应商 {
    /// 单次 chat/completions 请求
    pub(super) fn 请求(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
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
    pub(super) fn 列表请求(&self) -> Result<Vec<String>> {
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

impl LLM池 {
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
            启用json模式: false, // 接入向导默认关闭；需开启请配置 [llm.providers] 设 json_mode = true
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
        if let Err(失败) = self.保存选择文件() {
            tracing::warn!("保存 LLM 选择失败: {失败}");
        }
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
}
