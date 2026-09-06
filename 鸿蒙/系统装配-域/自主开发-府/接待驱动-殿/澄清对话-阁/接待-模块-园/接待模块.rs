use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use hm_content_contract::{工具对话器, 对话消息, 消息角色, 模型响应};
use hm_error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 道祖意图工具名（function calling 的 function.name）
const 工具_闲聊: &str = "闲聊";
const 工具_追问澄清: &str = "追问澄清";
const 工具_对齐总结: &str = "对齐总结";

/// 工具参数载荷键常量（与函数定义 schema 的 property 名一致）
const 键_回复: &str = "回复";
const 键_问题: &str = "问题";
const 键_标题: &str = "标题";
const 键_描述: &str = "描述";
const 键_场景: &str = "场景";
const 键_优先级: &str = "优先级";

/// 历史消息角色常量
const 角色_用户: &str = "用户";
const 角色_道祖: &str = "道祖";

/// 道祖系统提示：主控角色，接待 / 识别任务 / 澄清细节，不亲自执行
const 系统提示: &str = "你是洪荒世界的道祖，是主控角色。你负责接待来访者、识别他们下达的任务、澄清任务细节。你只做接待、澄清、终审，不亲自执行任何设计/实现/验收/清理工作——那些交由圣人（设计）、大罗金仙（实现）、准圣（验收）、太乙金仙（清理）完成。来访者提出任务时：若需求已清晰（目标、范围、验收标准明确），调用「对齐总结」给出结构化需求摘要；若需求不够清晰，调用「追问澄清」提出关键问题；若来访者只是闲聊或非任务，调用「闲聊」自然回应。";

/// 会话阶段：接待中（追问/闲聊）或待确认（道祖已给对齐总结，等用户确认）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum 会话阶段 {
    接待中,
    待确认,
}

/// 需求摘要：道祖对齐后的结构化需求（待用户确认后发布进看板）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 需求摘要 {
    pub 标题: String,
    pub 描述: String,
    pub 场景: Option<String>,
    pub 优先级: Option<String>,
}

/// 接待响应：道祖对用户消息的回应
#[derive(Debug, Clone)]
pub struct 接待响应 {
    pub 阶段: 会话阶段,
    pub 回复: String,
    pub 需求: Option<需求摘要>,
}

/// 会话消息：澄清会话历史的一条（可序列化持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct 会话消息 {
    角色: String,
    内容: String,
}

/// 会话快照：持久化用（历史 + 阶段 + 待确认需求）
#[derive(Debug, Serialize, Deserialize)]
struct 会话快照 {
    历史: Vec<会话消息>,
    阶段: 会话阶段,
    待确认需求: Option<需求摘要>,
}

/// 会话状态：内存态
struct 会话状态 {
    历史: Vec<会话消息>,
    阶段: 会话阶段,
    待确认需求: Option<需求摘要>,
}

/// 道祖接待器：主控角色，接待用户、识别任务、澄清细节、对齐后给出需求摘要。
/// 会话支持持久化（JSON 原子写），应对断开/突发关闭后恢复。
pub struct 道祖接待 {
    对话器: Arc<dyn 工具对话器>,
    会话: Mutex<会话状态>,
    存储路径: Option<PathBuf>,
}

impl 道祖接待 {
    /// 新建（不持久化，由 设置存储路径 开启）
    pub fn 新(对话器: Arc<dyn 工具对话器>) -> Self {
        道祖接待 {
            对话器,
            会话: Mutex::new(会话状态 {
                历史: Vec::new(),
                阶段: 会话阶段::接待中,
                待确认需求: None,
            }),
            存储路径: None,
        }
    }

    /// 设置持久化路径（下次 保存 起落盘）
    pub fn 设置存储路径(&mut self, 路径: impl Into<PathBuf>) {
        self.存储路径 = Some(路径.into());
    }

    /// 接待用户消息：LLM 判断意图，更新会话，持久化，返回回应
    pub fn 接待(&self, 用户消息: String) -> Result<接待响应> {
        let 文案 = 用户消息.trim().to_string();
        if 文案.is_empty() {
            return Err(Error::Other("消息不能为空".into()));
        }
        let 消息 = self.组装对话消息(&文案);
        let 响应 = self.对话器.对话(消息, 工具定义())?;
        self.更新会话(&文案, &响应)
    }

    /// 当前待确认需求（无则 None）
    pub fn 待确认需求(&self) -> Option<需求摘要> {
        let 会话 = self.会话.lock().expect("道祖接待锁中毒");
        会话.待确认需求.clone()
    }

    /// 确认发布：返回待确认需求并清空会话（重新开始接待）
    pub fn 确认发布(&self) -> Option<需求摘要> {
        let mut 会话 = self.会话.lock().expect("道祖接待锁中毒");
        let 需求 = 会话.待确认需求.take();
        if 需求.is_some() {
            会话.历史.clear();
            会话.阶段 = 会话阶段::接待中;
        }
        drop(会话);
        if 需求.is_some() {
            let _ = self.保存();
        }
        需求
    }

    /// 持久化到 JSON 文件（原子写：临时文件 + rename）
    pub fn 保存(&self) -> Result<()> {
        let Some(路径) = &self.存储路径 else { return Ok(()) };
        let 会话 = self.会话.lock().expect("道祖接待锁中毒");
        let 快照 = 会话快照 {
            历史: 会话.历史.clone(),
            阶段: 会话.阶段,
            待确认需求: 会话.待确认需求.clone(),
        };
        let 内容 =
            serde_json::to_string(&快照).map_err(|e| Error::序列化(format!("序列化会话失败: {e}")))?;
        if let Some(父) = 路径.parent() {
            if !父.as_os_str().is_empty() {
                std::fs::create_dir_all(父).map_err(Error::Io)?;
            }
        }
        let 临时 = 路径.with_extension("json.tmp");
        std::fs::write(&临时, 内容).map_err(Error::Io)?;
        std::fs::rename(&临时, 路径).map_err(Error::Io)?;
        Ok(())
    }

    /// 从文件加载（不存在时返回空会话）；加载后自动绑定对话器
    pub fn 加载(对话器: Arc<dyn 工具对话器>, 路径: impl Into<PathBuf>) -> Result<Self> {
        let 路径 = 路径.into();
        if !路径.exists() {
            return Ok(道祖接待 {
                对话器,
                会话: Mutex::new(会话状态 {
                    历史: Vec::new(),
                    阶段: 会话阶段::接待中,
                    待确认需求: None,
                }),
                存储路径: Some(路径),
            });
        }
        let 文本 = std::fs::read_to_string(&路径).map_err(Error::Io)?;
        let 快照: 会话快照 =
            serde_json::from_str(&文本).map_err(|e| Error::反序列化(format!("解析会话失败: {e}")))?;
        Ok(道祖接待 {
            对话器,
            会话: Mutex::new(会话状态 {
                历史: 快照.历史,
                阶段: 快照.阶段,
                待确认需求: 快照.待确认需求,
            }),
            存储路径: Some(路径),
        })
    }

    /// 组装对话消息：系统提示 + 历史 + 新用户消息
    fn 组装对话消息(&self, 文案: &str) -> Vec<对话消息> {
        let mut 消息 = vec![对话消息::系统(系统提示.to_string())];
        let 会话 = self.会话.lock().expect("道祖接待锁中毒");
        for 条 in &会话.历史 {
            消息.push(按角色转消息(条));
        }
        消息.push(对话消息::用户(文案.to_string()));
        消息
    }

    /// 解析道祖意图并落定会话（更新历史/阶段/待确认需求），持久化后返回接待响应
    fn 更新会话(&self, 文案: &str, 响应: &模型响应) -> Result<接待响应> {
        let (回复, 需求) = 解析意图(响应)?;
        let 阶段 = if 需求.is_some() {
            会话阶段::待确认
        } else {
            会话阶段::接待中
        };
        {
            let mut 会话 = self.会话.lock().expect("道祖接待锁中毒");
            会话.历史.push(会话消息 { 角色: 角色_用户.into(), 内容: 文案.to_string() });
            会话.历史.push(会话消息 { 角色: 角色_道祖.into(), 内容: 回复.clone() });
            会话.阶段 = 阶段;
            会话.待确认需求 = 需求.clone();
        }
        self.保存()?;
        Ok(接待响应 { 阶段, 回复, 需求 })
    }
}

/// 按会话消息角色转对话消息（用户 / 道祖）
fn 按角色转消息(条: &会话消息) -> 对话消息 {
    if 条.角色 == 角色_用户 {
        对话消息::用户(条.内容.clone())
    } else {
        对话消息 {
            角色: 消息角色::Assistant,
            内容: Some(条.内容.clone()),
            工具调用: Vec::new(),
            工具调用id: None,
        }
    }
}

/// 解析道祖意图：返回（回复文本，对齐摘要）
fn 解析意图(响应: &模型响应) -> Result<(String, Option<需求摘要>)> {
    let Some(调用) = 响应.工具调用.first() else {
        return Ok((响应.内容.clone().unwrap_or_default(), None));
    };
    let 参数: serde_json::Value = serde_json::from_str(&调用.参数)
        .map_err(|e| Error::反序列化(format!("解析工具参数失败: {e}")))?;
    match 调用.名称.as_str() {
        工具_闲聊 => Ok((取字符串(&参数, 键_回复).unwrap_or_default(), None)),
        工具_追问澄清 => Ok((取字符串(&参数, 键_问题).unwrap_or_default(), None)),
        工具_对齐总结 => 解析对齐总结(&参数),
        _ => Ok((响应.内容.clone().unwrap_or_default(), None)),
    }
}

/// 解析对齐总结工具参数为需求摘要
fn 解析对齐总结(参数: &serde_json::Value) -> Result<(String, Option<需求摘要>)> {
    let 标题 = 取字符串(参数, 键_标题).unwrap_or_default();
    let 描述 = 取字符串(参数, 键_描述).unwrap_or_default();
    if 标题.is_empty() || 描述.is_empty() {
        return Err(Error::Other("对齐总结缺少标题或描述".into()));
    }
    let 摘要 = 需求摘要 {
        标题,
        描述,
        场景: 取字符串(参数, 键_场景),
        优先级: 取字符串(参数, 键_优先级),
    };
    Ok((format!("已对齐需求「{}」，请确认发布。", 摘要.标题), Some(摘要)))
}

/// 从工具参数取字符串字段
fn 取字符串(参数: &serde_json::Value, 键: &str) -> Option<String> {
    参数.get(键).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// 道祖三种意图的函数定义（OpenAI function calling 的 tools 数组元素）
fn 工具定义() -> Vec<serde_json::Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": 工具_闲聊,
                "description": "来访者只是在闲聊或非任务，给出自然回应",
                "parameters": {
                    "type": "object",
                    "properties": { 键_回复: {"type": "string", "description": "道祖的回应文本"} },
                    "required": [键_回复]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": 工具_追问澄清,
                "description": "任务需求不够清晰，提出关键澄清问题",
                "parameters": {
                    "type": "object",
                    "properties": { 键_问题: {"type": "string", "description": "要向来访者提出的澄清问题"} },
                    "required": [键_问题]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": 工具_对齐总结,
                "description": "任务需求已清晰，给出结构化需求摘要等待确认",
                "parameters": {
                    "type": "object",
                    "properties": {
                        键_标题: {"type": "string", "description": "任务标题"},
                        键_描述: {"type": "string", "description": "完整需求描述"},
                        键_场景: {"type": "string", "description": "任务场景：理解/设计/修改/调试/重构"},
                        键_优先级: {"type": "string", "description": "优先级：P0/P1/P2/P3"}
                    },
                    "required": [键_标题, 键_描述]
                }
            }
        }),
    ]
}