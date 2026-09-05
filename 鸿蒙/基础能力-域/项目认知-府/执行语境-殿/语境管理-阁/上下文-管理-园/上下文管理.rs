use std::collections::HashMap;
use std::path::PathBuf;
use hm_contract::当前时间戳;
use hm_error::{Error, Result};
use serde::{Deserialize, Serialize};
use crate::AgentRole;
use crate::语境消息;

/// 工具调用记录：一次工具调用的摘要（名称/参数/结果/时间）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub 工具名: String,
    pub 参数: String,
    pub 结果摘要: String,
    pub 时间戳: u64,
}

/// 执行上下文：每个 Agent 独立的执行过程，不共享。
///
/// 核心原则：上下文是单个层级角色（道祖/圣人/大罗金仙/准圣）的私有思考与工具调用记录，
/// 任务完成后清理或归档，但不传递给下一层级；下一层级只从任务看板取任务元信息并新建自己的上下文。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub context_id: String,
    pub 层级角色: AgentRole,
    pub task_id: Option<u64>,
    pub 消息: Vec<语境消息>,
    pub 工具调用: Vec<ToolCallRecord>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// 上下文管理器：管理每个 Agent 的独立上下文，支持持久化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManager {
    上下文集: HashMap<String, Context>,
    下一序号: u64,
    #[serde(skip)]
    存储路径: PathBuf,
}

impl ContextManager {
    /// 新建上下文管理器，绑定存储路径（用于 保存/加载）
    pub fn 新(存储路径: impl Into<PathBuf>) -> Self {
        ContextManager {
            上下文集: HashMap::new(),
            下一序号: 1,
            存储路径: 存储路径.into(),
        }
    }

    /// 创建一个新上下文并返回其唯一 ID（每 Agent 开始工作时创建）
    pub fn 创建上下文(&mut self, role: AgentRole, task_id: Option<u64>) -> String {
        let id = format!("ctx-{}-{}", 当前时间戳(), self.下一序号);
        self.下一序号 += 1;
        let 时间 = 当前时间戳();
        let context = Context {
            context_id: id.clone(),
            层级角色: role,
            task_id,
            消息: Vec::new(),
            工具调用: Vec::new(),
            created_at: 时间,
            updated_at: 时间,
        };
        self.上下文集.insert(id.clone(), context);
        id
    }

    /// 按 ID 查询上下文
    pub fn 查询(&self, context_id: &str) -> Option<&Context> {
        self.上下文集.get(context_id)
    }

    /// 向指定上下文追加一条消息
    pub fn 添加消息(&mut self, context_id: &str, message: 语境消息) -> Result<()> {
        let context = self
            .上下文集
            .get_mut(context_id)
            .ok_or_else(|| Error::上下文不存在(context_id.to_string()))?;
        context.消息.push(message);
        context.updated_at = 当前时间戳();
        Ok(())
    }

    /// 向指定上下文记录一次工具调用
    pub fn 记录工具调用(&mut self, context_id: &str, call: ToolCallRecord) -> Result<()> {
        let context = self
            .上下文集
            .get_mut(context_id)
            .ok_or_else(|| Error::上下文不存在(context_id.to_string()))?;
        context.工具调用.push(call);
        context.updated_at = 当前时间戳();
        Ok(())
    }

    /// 清理指定上下文（任务完成后调用，不保留在共享空间）
    pub fn 清理上下文(&mut self, context_id: &str) -> Result<()> {
        self.上下文集
            .remove(context_id)
            .map(|_| ())
            .ok_or_else(|| Error::上下文不存在(context_id.to_string()))
    }

    /// 列出全部上下文
    pub fn 全部(&self) -> Vec<&Context> {
        self.上下文集.values().collect()
    }

    /// 持久化到文件（原子写：先写临时文件再重命名）
    pub fn 保存(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::序列化(format!("序列化上下文失败: {e}")))?;
        let 临时路径 = self.存储路径.with_extension("jsonl.tmp");
        std::fs::write(&临时路径, content).map_err(Error::Io)?;
        std::fs::rename(&临时路径, &self.存储路径).map_err(Error::Io)?;
        Ok(())
    }

    /// 从文件加载；文件不存在时返回空管理器
    pub fn 加载(存储路径: impl Into<PathBuf>) -> Result<Self> {
        let path = 存储路径.into();
        if !path.exists() {
            return Ok(ContextManager::新(path));
        }
        let content = std::fs::read_to_string(&path).map_err(Error::Io)?;
        let mut manager: ContextManager = serde_json::from_str(&content)
            .map_err(|e| Error::反序列化(format!("解析上下文文件失败: {e}")))?;
        manager.存储路径 = path;
        Ok(manager)
    }
}