//! 符号索引统一契约：插件清单、解析定义、符号、边、产出包、索引
//!
//! 本文件是「语言解析插件架构」的数据契约，见
//! `.传承/设计/落地设计/语言解析插件-落地设计.md` §四。
//! 引擎只认这里的类型，不认任何具体语言。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 当前协议版本；插件与引擎版本不一致时拒绝加载
pub const 当前协议版本: &str = "v1";

// ============ 一、插件自述 ============

/// 置信度档位（设计 §4.5）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum 置信度 {
    /// 编译级精确（语言官方语义分析器 / 官方 AST）
    高,
    /// 语法级可靠（通用语法树解析器）
    中,
    /// 启发式（正则 / 文本匹配）
    低,
}

/// 插件清单（设计 §4.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 插件清单 {
    pub 协议版本: String,
    pub 语言: String,
    pub 显示名: String,
    /// 引擎分发的唯一依据
    pub 文件后缀: Vec<String>,
    /// 启动插件的完整命令；数组形式以免引号转义问题
    pub 入口: Vec<String>,
    /// 边类型名 → 置信度；未列出的边视为「不支持」
    pub 能力: BTreeMap<String, 置信度>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub 备注: Option<String>,
}

/// 解析定义中的一条插件登记
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 插件登记 {
    pub 语言: String,
    /// 插件清单的仓库相对路径
    pub 清单: String,
}

/// 解析定义（项目特化数据，见设计 §六）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 解析定义 {
    pub 版本: String,
    pub 启用插件: Vec<插件登记>,
}

// ============ 二、符号与边 ============

/// 统一符号种类
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum 符号种类 {
    Crate,
    模块,
    函数,
    结构体,
    枚举,
    联合,
    /// trait / 接口（各语言对应物）
    特征,
    /// impl 块
    实现块,
    类型别名,
    常量,
    静态量,
}

/// 统一边词表（设计 §4.3）；禁止自造，新增须先改设计
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum 边种类 {
    包含,
    导入,
    定义,
    实现,
    调用,
    依赖,
}

/// 源码位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 位置 {
    pub 文件: String,
    pub 行: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub 列: Option<usize>,
}

/// 符号节点（设计 §4.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 符号 {
    /// 稳定 ID：`<语言>::<包名>::<模块路径>::<符号名>`；**不含行号**（设计 §4.4）
    pub id: String,
    pub 种类: 符号种类,
    pub 名: String,
    pub 全名: String,
    pub 文件: String,
    pub 行: usize,
    pub 可见性: String,
    /// 所属容器符号的 id（嵌套符号靠它成边，不靠命名拼接）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub 容器: Option<String>,
}

/// 边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 边 {
    pub 从: String,
    pub 到: String,
    pub 类型: 边种类,
    pub 置信度: 置信度,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub 位置: Option<位置>,
}

/// 悬空引用：解析不到目标的引用（设计 §4.2 要点 2）
///
/// 悬空正是「幽灵引用」诊断的原始素材，**不得丢弃**。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 悬空 {
    pub 从: String,
    /// 原始标识符文本
    pub 目标文本: String,
    pub 类型: 边种类,
    pub 置信度: 置信度,
}

/// 插件向人报告自身局限（不参与图谱合并）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 插件诊断 {
    pub 级别: String,
    pub 文件: String,
    pub 信息: String,
}

/// 插件产出包（进程契约的返回体，设计 §4.2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 产出 {
    pub 协议版本: String,
    pub 语言: String,
    pub 符号: Vec<符号>,
    pub 边: Vec<边>,
    #[serde(default)]
    pub 悬空: Vec<悬空>,
    #[serde(default)]
    pub 诊断: Vec<插件诊断>,
}

// ============ 三、引擎产出（仓库固化索引） ============

/// 带坐标与来源的符号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 索引符号 {
    pub 符号: 符号,
    pub 来源语言: String,
    /// 多层语义坐标（由坐标索引 join 得到）
    pub 坐标: BTreeMap<String, String>,
}

/// 单个插件的执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 插件状态 {
    pub 语言: String,
    /// 成功 / 跳过 / 失败
    pub 状态: String,
    pub 符号数: usize,
    pub 边数: usize,
    pub 信息: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 索引元信息 {
    pub 图谱: String,
    pub 版本: String,
    pub 生成时间: String,
    pub 仓库根: String,
    pub 定义文件: String,
    pub 生成器: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 索引统计 {
    pub 符号总数: usize,
    pub 边总数: usize,
    pub 悬空总数: usize,
    pub 按种类: BTreeMap<String, usize>,
    pub 按语言: BTreeMap<String, usize>,
    pub 按坐标根: BTreeMap<String, usize>,
    pub 插件: Vec<插件状态>,
}

/// 符号索引（引擎最终产出，落 `.传承/图谱/知识图谱/符号索引.json`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 符号索引 {
    pub 元信息: 索引元信息,
    pub 统计: 索引统计,
    pub 符号: Vec<索引符号>,
    pub 边: Vec<边>,
    pub 悬空: Vec<悬空>,
}
