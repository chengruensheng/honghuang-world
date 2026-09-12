//! 文档生成的数据模型：生成配置、事实投影、校验结论、生成结果。
//!
//! 符号 / 边 / 悬空 直接复用上游契约，不另立同义类型——同义类型会立刻变成
//! 又一处需要同步的真相，正是本设计要避免的。

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use hm_symext::{悬空, 符号, 边};
use serde::{Deserialize, Serialize};

/// 生成配置
#[derive(Debug, Clone)]
pub struct 生成配置 {
    /// 符号索引文件路径（唯一事实源）
    pub 索引路径: PathBuf,
    /// 产出目录
    pub 输出目录: PathBuf,
    /// 范围限定：文件路径前缀；为空表示全量
    pub 范围: Option<String>,
}

/// 一个源文件在事实图上的投影
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 文档事实 {
    /// 仓库相对路径，同时是这篇文档的溯源坐标
    pub 文件: String,
    /// 多层语义坐标（由上游 join 得到）
    pub 坐标: BTreeMap<String, String>,
    pub 来源语言: String,
    /// 本文件定义的符号
    pub 定义: Vec<符号>,
    /// 本文件发出的边
    pub 出边: Vec<边>,
    /// 指向本文件符号的边
    pub 入边: Vec<边>,
    /// 本文件内的悬空引用
    pub 悬空: Vec<悬空>,
}

impl 文档事实 {
    /// 可引用的名字集合：本文件定义的符号 + 出入边与悬空引用的两端。
    ///
    /// 单点定义：叙述器用它约束模型（提示词里的「可引用名」清单），校验器用它
    /// 判定幻觉。两处若各算一份，任何一处漂移都会让「清单允许的名字」与「校验
    /// 接受的名字」错位——错位即误报或漏报，比不校验更坏。
    pub fn 可引用名(&self) -> BTreeSet<String> {
        let mut 集合 = BTreeSet::new();
        for 符 in &self.定义 {
            集合.insert(符.名.clone());
            集合.insert(符.全名.clone());
            集合.insert(符.id.clone());
        }
        for 边 in self.出边.iter().chain(self.入边.iter()) {
            for 端 in [&边.从, &边.到] {
                集合.insert(端.clone());
                集合.insert(短名(端).to_string());
            }
        }
        for 悬 in &self.悬空 {
            集合.insert(悬.目标文本.clone());
        }
        集合
    }
}

/// 取 ID 的最后一段（ID 规范：最后一段为符号名）
pub fn 短名(id: &str) -> &str {
    id.rsplit("::").next().unwrap_or(id)
}

/// 事实校验判定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum 判定 {
    /// 叙述中的符号引用均可溯源
    通过,
    /// 引用了事实集中不存在的符号名（疑似幻觉）
    疑似幻觉(Vec<String>),
}

/// 单篇校验结论
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 校验结论 {
    pub 文件: String,
    pub 判定: 判定,
}

/// 校验器自检结论
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 自检结论 {
    pub 通过: bool,
    pub 说明: String,
}

/// 整批生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 生成结果 {
    pub 篇数: usize,
    pub 符号数: usize,
    pub 边数: usize,
    /// 本次实际使用的叙述器名（模型后端不可用时整批回落，报告需注明）
    pub 叙述器: String,
    /// 叙述降级篇数（叙述器失败后回落模板的篇数）
    pub 降级篇数: usize,
    pub 校验: Vec<校验结论>,
    pub 自检: 自检结论,
}

impl 生成结果 {
    /// 命中疑似幻觉的篇数
    pub fn 可疑篇数(&self) -> usize {
        self.校验
            .iter()
            .filter(|结论| matches!(结论.判定, 判定::疑似幻觉(_)))
            .count()
    }
}
