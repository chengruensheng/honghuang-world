use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use hm_content_contract::{工具对话器, 对话消息, 工具调用};
use hm_error::{Error, Result};
use hm_execute_contract::执行器;

/// 工具名常量（function calling 的 function.name）
const 读文件: &str = "读文件";
const 写文件: &str = "写文件";
const 运行命令: &str = "运行命令";

/// 工具参数载荷键常量（与函数定义 schema 的 property 名一致）
const 参数键_路径: &str = "路径";
const 参数键_内容: &str = "内容";
const 参数键_命令: &str = "命令";

/// 系统提示：约束 LLM 的角色与工具使用方式
const 系统提示: &str = "你是一个自主开发智能体，在指定工作区内完成开发任务。可调用「读文件」「写文件」「运行命令」三个工具。运行环境是 Windows，命令须用 cmd 语法（列目录用 dir、查看文件用 type、构建测试用 cargo）。每步先思考再行动；工具失败要读取错误信息并修正；任务完成后停止调用工具并给出简短说明。";

/// 自主开发智能体：LLM 大脑 + 执行器手脚的循环
pub struct 智能体 {
    对话器: Arc<dyn 工具对话器>,
    执行器: Arc<dyn 执行器>,
    最大轮数: usize,
    中断标志: Arc<AtomicBool>,
}

impl 智能体 {
    pub fn new(对话器: Arc<dyn 工具对话器>, 执行器: Arc<dyn 执行器>, 最大轮数: usize) -> Self {
        智能体 { 对话器, 执行器, 最大轮数, 中断标志: Arc::new(AtomicBool::new(false)) }
    }

    /// 返回中断句柄：外部设置 true 可请求停止循环（如 Ctrl+C 回调）
    pub fn 中断句柄(&self) -> Arc<AtomicBool> {
        self.中断标志.clone()
    }

    /// 运行自主开发循环，返回 LLM 最终答复
    pub fn 运行(&self, 任务: String) -> Result<String> {
        let mut 消息 = vec![
            对话消息::系统(系统提示.to_string()),
            对话消息::用户(任务),
        ];
        let 工具 = 工具定义();

        for 轮次 in 0..self.最大轮数 {
            if self.中断标志.load(Ordering::SeqCst) {
                tracing::warn!("收到中断请求，停止智能体循环");
                return Err(Error::中断("循环被用户中断".into()));
            }
            tracing::info!("══════ 第 {} 轮 ══════", 轮次 + 1);
            let 响应 = self.对话器.对话(消息.clone(), 工具.clone())?;

            if 响应.工具调用.is_empty() {
                let 答复 = 响应.内容.clone().unwrap_or_default();
                tracing::info!("【LLM 最终答复】{}", 答复);
                return Ok(答复);
            }

            消息.push(对话消息::助手调用(响应.工具调用.clone()));
            for 调用 in &响应.工具调用 {
                tracing::info!("【LLM 调用工具】{}  参数：{}", 调用.名称, 调用.参数);
                // 工具失败不回传终止循环，而是把错误信息回填给 LLM，让 LLM 看到错误后修正重试
                let 结果 = match self.执行调用(调用) {
                    Ok(输出) => 输出,
                    Err(e) => {
                        tracing::warn!("【工具失败】{}", e);
                        format!("工具执行失败: {e}")
                    }
                };
                tracing::info!("【工具返回】{}", 结果);
                消息.push(对话消息::工具结果(调用.id.clone(), 结果));
            }
        }

        Err(Error::超出轮数(format!("超过最大轮数 {} 仍未完成任务", self.最大轮数)))
    }

    /// 分发并执行单个工具调用
    fn 执行调用(&self, 调用: &工具调用) -> Result<String> {
        let 参数: serde_json::Value = serde_json::from_str(&调用.参数)
            .map_err(|e| Error::反序列化(format!("解析工具参数失败: {e}")))?;
        match 调用.名称.as_str() {
            读文件 => {
                let 路径 = 取参数字符串(&参数, 参数键_路径)?;
                self.执行器.读文件(&路径)
            }
            写文件 => {
                let 路径 = 取参数字符串(&参数, 参数键_路径)?;
                let 内容 = 取参数字符串(&参数, 参数键_内容)?;
                self.执行器.写文件(&路径, &内容)?;
                Ok("写入成功".to_string())
            }
            运行命令 => {
                let 命令 = 取参数字符串(&参数, 参数键_命令)?;
                self.执行器.运行命令(&命令)
            }
            _ => Err(Error::未知工具(调用.名称.clone())),
        }
    }
}

/// 从工具参数中取出字符串字段；缺失或非字符串时报错
fn 取参数字符串(参数: &serde_json::Value, 键: &str) -> Result<String> {
    参数.get(键)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::缺少参数(键.to_string()))
}

/// 三个工具的函数定义（OpenAI function calling 的 tools 数组元素）
fn 工具定义() -> Vec<serde_json::Value> {
    vec![
        单参函数(读文件, "读取工作区内文件的完整文本", 参数键_路径, "相对工作区的文件路径"),
        双参函数(写文件, "把内容写入工作区文件（覆盖）", 参数键_路径, "相对工作区的文件路径", 参数键_内容, "要写入的完整文本"),
        单参函数(运行命令, "在工作区目录下运行命令，返回标准输出", 参数键_命令, "要执行的命令"),
    ]
}

/// 构造单字符串参数的函数定义
fn 单参函数(名: &str, 描述: &str, 键: &str, 键说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键.to_string(), serde_json::json!({"type": "string", "description": 键说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": [键]
            }
        }
    })
}

/// 构造双字符串参数的函数定义
fn 双参函数(名: &str, 描述: &str, 键一: &str, 键一说明: &str, 键二: &str, 键二说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键一.to_string(), serde_json::json!({"type": "string", "description": 键一说明}));
    属性.insert(键二.to_string(), serde_json::json!({"type": "string", "description": 键二说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": [键一, 键二]
            }
        }
    })
}
