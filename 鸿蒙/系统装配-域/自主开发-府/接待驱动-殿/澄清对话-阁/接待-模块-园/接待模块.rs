use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use hm_content_contract::{工具对话器, 流式对话器, 对话消息, 消息角色, 模型响应};
use hm_error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::认知注入;

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

/// 会话历史总量硬上限（条数）：每轮 用户+道祖 两条，保留最近 6 轮澄清。
/// 有界环形避免长会话下历史无限膨胀撑大上下文——MiniMax-M3 长上下文下 tool_call 决策不稳定，
/// 缩上下文后（跳过认知 + 历史截取 + tool_choice:auto）稳定触发「对齐总结」。数值可按真机实测调整。
const 会话历史上限: usize = 12;

/// 道祖系统提示：主控角色，接待 / 识别任务 / 澄清细节，不亲自执行
const 系统提示: &str = "你是洪荒世界的道祖，是主控角色。你负责接待来访者、识别他们下达的任务、澄清任务细节。你只做接待、澄清、终审，不亲自执行任何设计/实现/验收/清理工作——那些交由圣人（设计）、大罗金仙（实现）、准圣（验收）、太乙金仙（清理）完成。来访者提出任务时：若需求已清晰（目标、范围、验收标准明确），调用「对齐总结」给出结构化需求摘要（系统会自动发布到开发流水线，不需要来访者手动确认）；若需求不够清晰，调用「追问澄清」提出关键问题；若来访者只是闲聊或非任务，调用「闲聊」自然回应。";

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
    /// 流式对话器（可选）：装配后 接待流式 走增量推送；缺失时回退同步 对话 一次性回调
    流式: Option<Arc<dyn 流式对话器>>,
    会话: Mutex<会话状态>,
    存储路径: Option<PathBuf>,
    /// 项目认知注入（可选）：装配后接待时把推/拉认知记忆拼入系统提示，对齐看板驱动通道
    认知: Option<认知注入>,
}

impl 道祖接待 {
    /// 新建（不持久化，由 设置存储路径 开启）
    pub fn 新(对话器: Arc<dyn 工具对话器>) -> Self {
        道祖接待 {
            对话器,
            流式: None,
            会话: Mutex::new(会话状态 {
                历史: Vec::new(),
                阶段: 会话阶段::接待中,
                待确认需求: None,
            }),
            存储路径: None,
            认知: None,
        }
    }

    /// 装配流式对话器（同源 LLM 实现）：开启 接待流式 增量推送
    pub fn 装配流式(mut self, 流式: Arc<dyn 流式对话器>) -> Self {
        self.流式 = Some(流式);
        self
    }

    /// 设置持久化路径（下次 保存 起落盘）
    pub fn 设置存储路径(&mut self, 路径: impl Into<PathBuf>) {
        self.存储路径 = Some(路径.into());
    }

    /// 装配项目认知注入：接待时把推/拉认知记忆拼入系统提示（对齐看板驱动通道的认知装配）
    pub fn 装配认知(mut self, 认知: 认知注入) -> Self {
        self.认知 = Some(认知);
        self
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

    /// 流式接待：组装消息后**先释放会话锁**，LLM 增量回调推送文本，末尾落会话（持久化）。
    /// 流式对话器缺失或流式失败 → 回退同步 `对话` 一次性回调，保证功能可用。
    pub fn 接待流式(
        &self,
        用户消息: String,
        on_chunk: &mut dyn FnMut(String) -> std::result::Result<(), hm_error::Error>,
    ) -> Result<接待响应> {
        let 文案 = 用户消息.trim().to_string();
        if 文案.is_empty() {
            return Err(Error::Other("消息不能为空".into()));
        }
        // 组装对话消息：内部拿会话锁读历史，返回即释放（LLM 调用期间不持锁）
        let 消息 = self.组装对话消息(&文案);
        let 响应 = match &self.流式 {
            Some(流式) => {
                let mut 回调 = |块: String| on_chunk(块);
                match 流式.对话流式(消息.clone(), 工具定义(), &mut 回调) {
                    Ok(完整) => 完整,
                    Err(e) => {
                        tracing::warn!("流式对话失败，回退同步对话: {e}");
                        self.对话器.对话(消息.clone(), 工具定义())?
                    }
                }
            }
            None => self.对话器.对话(消息, 工具定义())?,
        };
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
            if let Err(失败) = self.保存() {
                tracing::warn!("道祖接待保存失败: {失败}");
            }
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
                流式: None,
                会话: Mutex::new(会话状态 {
                    历史: Vec::new(),
                    阶段: 会话阶段::接待中,
                    待确认需求: None,
                }),
                存储路径: Some(路径),
                认知: None,
            });
        }
        let 文本 = std::fs::read_to_string(&路径).map_err(Error::Io)?;
        let 快照: 会话快照 =
            serde_json::from_str(&文本).map_err(|e| Error::反序列化(format!("解析会话失败: {e}")))?;
        // 兼容历史冗长持久化文件：加载时即按上限截断，避免恢复后仍超界
        let mut 历史 = 快照.历史;
        保留最近(&mut 历史);
        Ok(道祖接待 {
            对话器,
            流式: None,
            会话: Mutex::new(会话状态 {
                历史,
                阶段: 快照.阶段,
                待确认需求: 快照.待确认需求,
            }),
            存储路径: Some(路径),
            认知: None,
        })
    }

    /// 组装对话消息：系统提示（含认知记忆）+ 历史 + 新用户消息
    fn 组装对话消息(&self, 文案: &str) -> Vec<对话消息> {
        // 需求已明确 → 跳过认知注入（缩短上下文），让道祖优先稳定触发「对齐总结」tool_call
        let 提示 = if 需求已明确(文案) {
            系统提示.to_string()
        } else if let Some(认知) = &self.认知 {
            let 记忆 = 认知.初始注入(文案);
            if 记忆.is_empty() {
                系统提示.to_string()
            } else {
                format!("{系统提示}\n\n【项目认知记忆】\n{记忆}")
            }
        } else {
            系统提示.to_string()
        };
        // 低侵入增强：来访者需求已明确 → 追加一行强指令，引导道祖走「对齐总结」工具而非文本
        let 提示 = if 需求已明确(文案) {
            format!("{提示}\n\n【当前来访者需求已明确（含目标/范围/验收标准等）】请务必调用「对齐总结」工具给出结构化需求摘要（含标题、描述、场景、优先级），不得仅以闲聊文本回答。若仍缺少关键信息才调用「追问澄清」。")
        } else {
            提示
        };
        let mut 消息 = vec![对话消息::系统(提示)];
        let 会话 = self.会话.lock().expect("道祖接待锁中毒");
        // 缩上下文：需求已明确 → 历史只带最近 1 轮，避免长上下文/重复累积干扰 tool_call
        let 起点 = if 需求已明确(文案) {
            会话.历史.len().saturating_sub(2)
        } else {
            0
        };
        for 条 in &会话.历史[起点..] {
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
            保留最近(&mut 会话.历史);
            会话.阶段 = 阶段;
            会话.待确认需求 = 需求.clone();
        }
        self.保存()?;
        Ok(接待响应 { 阶段, 回复, 需求 })
    }
}

/// 有界环形：保留历史最近 `会话历史上限` 条，丢弃最旧（超上限才截断）。
/// 长会话下避免历史无限膨胀撑大上下文，保证 MiniMax-M3 长上下文下 tool_call 决策稳定。
fn 保留最近(历史: &mut Vec<会话消息>) {
    let 上限 = 会话历史上限;
    if 历史.len() > 上限 {
        let 溢出 = 历史.len() - 上限;
        历史.drain(..溢出);
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
    Ok((format!("我理解为「{}」，现在开始安排，你可以看着它跑。", 摘要.标题), Some(摘要)))
}

/// 判断来访者需求是否已明确：检测到任务特征词（目标/范围/验收标准/技术栈/暴露接口等）
/// → 引导道祖以「对齐总结」工具落结构化摘要；否则维持原文（不干扰澄清/闲聊路径）。
fn 需求已明确(文案: &str) -> bool {
    let 特征词 = [
        "请实现", "请在", "新建", "创建", "暴露", "pub fn", "函数", "模块", "crate", "crates",
        "技术栈", "验收", "测试", "场景", "优先级", "Rust", "rust", "实现一个",
        "目标", "范围", "接口", "签名", "功能", "需求", "任务",
    ];
    特征词.iter().any(|词| 文案.contains(词))
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
                "description": "任务需求已清晰，给出结构化需求摘要（系统自动发布）",
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

#[cfg(test)]
mod tests {
    use super::*;
    use hm_contract::Component;
    use hm_content_contract::工具调用;

    /// mock 对话器：仅返回纯文本响应（无工具调用），供 道祖接待 集成测试构造
    struct Mock对话器;
    impl Component for Mock对话器 {
        fn name(&self) -> &'static str {
            "mock"
        }
    }
    impl 工具对话器 for Mock对话器 {
        fn 对话(&self, _: Vec<对话消息>, _: Vec<serde_json::Value>) -> Result<模型响应> {
            Ok(模型响应 { 内容: Some("好的".into()), 工具调用: vec![] })
        }
    }

    fn 构造工具调用(名称: &str, 参数: serde_json::Value) -> 工具调用 {
        工具调用 { id: "t1".into(), 名称: 名称.into(), 参数: 参数.to_string() }
    }

    /// 对齐总结 tool_call → 需求摘要 + 阶段=待确认
    #[test]
    fn 对齐总结工具调用_产出需求摘要() {
        let 响应 = 模型响应 {
            内容: None,
            工具调用: vec![构造工具调用(
                工具_对齐总结,
                json!({ 键_标题: "新建 jia-shang crate", 键_描述: "在 crates 目录新建纯函数 crate，暴露两数相加", 键_场景: "设计", 键_优先级: "P1" }),
            )],
        };
        let (回复, 需求) = 解析意图(&响应).unwrap();
        assert!(需求.is_some(), "对齐总结应产出需求摘要");
        let 摘要 = 需求.unwrap();
        assert_eq!(摘要.标题, "新建 jia-shang crate");
        assert_eq!(摘要.描述, "在 crates 目录新建纯函数 crate，暴露两数相加");
        assert_eq!(摘要.场景.as_deref(), Some("设计"));
        assert_eq!(摘要.优先级.as_deref(), Some("P1"));
        assert!(回复.contains("新建 jia-shang crate"));
    }

    /// 闲聊 工具 → 回复文本，无需求（阶段=接待中）
    #[test]
    fn 闲聊工具调用_无需求() {
        let 响应 = 模型响应 {
            内容: None,
            工具调用: vec![构造工具调用(工具_闲聊, json!({ 键_回复: "善。" }))],
        };
        let (回复, 需求) = 解析意图(&响应).unwrap();
        assert_eq!(回复, "善。");
        assert!(需求.is_none());
    }

    /// 无工具调用（纯文本答复）→ 无需求
    #[test]
    fn 无工具调用_纯文本_无需求() {
        let 响应 = 模型响应 { 内容: Some("好的".into()), 工具调用: vec![] };
        let (回复, 需求) = 解析意图(&响应).unwrap();
        assert_eq!(回复, "好的");
        assert!(需求.is_none());
    }

    /// 对齐总结缺标题/描述 → 报错（防脏数据入库）
    #[test]
    fn 对齐总结缺字段_报错() {
        let 响应 = 模型响应 {
            内容: None,
            工具调用: vec![构造工具调用(工具_对齐总结, json!({ 键_标题: "", 键_描述: "" }))],
        };
        assert!(解析意图(&响应).is_err());
    }

    /// 含任务特征词 → 需求已明确（触发对齐总结引导）
    #[test]
    fn 明确任务判为已明确() {
        assert!(需求已明确("请在 crates 目录新建 jia-shang crate，暴露 pub fn 两数相加"));
        assert!(需求已明确("请实现一个两数相加的函数，Rust，优先级 P1，场景设计"));
    }

    /// 纯闲聊/问候 → 不判定为明确（不干扰澄清/闲聊）
    #[test]
    fn 闲聊不判为明确() {
        assert!(!需求已明确("你好"));
        assert!(!需求已明确("随便聊聊今天天气"));
    }

    /// 长会话下 更新会话 把历史截断到上限（有界环形，最旧被丢弃）
    #[test]
    fn 更新会话_历史保持有界() {
        let 接待 = 道祖接待::新(Arc::new(Mock对话器));
        for i in 0..(会话历史上限 + 6) {
            接待
                .更新会话(&format!("消息{i}"), &模型响应 { 内容: Some(format!("回复{i}")), 工具调用: vec![] })
                .unwrap();
        }
        let 会话 = 接待.会话.lock().expect("道祖接待锁中毒");
        assert!(会话.历史.len() <= 会话历史上限, "历史应被截断到上限，实际 {}", 会话.历史.len());
        assert_eq!(会话.历史.last().unwrap().角色, 角色_道祖);
        assert!(会话.历史.iter().all(|m| !m.内容.contains("消息0")), "最旧消息应被丢弃");
    }

    /// 历史未超上限 → 不截断（避免误丢早期澄清）
    #[test]
    fn 保留最近_未超上限_不截断() {
        let mut 历史 = vec![会话消息 { 角色: "用户".into(), 内容: "你好".into() }];
        保留最近(&mut 历史);
        assert_eq!(历史.len(), 1);
    }
}