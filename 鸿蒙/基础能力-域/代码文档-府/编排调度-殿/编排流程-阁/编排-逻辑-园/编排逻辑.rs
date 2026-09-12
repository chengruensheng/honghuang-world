//! 编排：读事实源 → 投影 → 叙述 → 校验 → 渲染落盘。
//!
//! 全量重生成，不做增量：模板叙述是秒级，而增量需判定输入变更，一旦漏项
//! 即产生静默陈旧（陈旧数据比慢更糟）。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use hm_error::{Error, Result};
use hm_symext::符号索引;

use crate::{
    校验叙述, 渲染校验报告, 渲染文档, 渲染索引, 文档事实, 文档文件名, 模板叙述器, 生成配置,
    生成结果, 自检, 叙述器,
};

/// 索引页文件名（产出契约的一部分）
const 索引文件: &str = "index.md";
/// 校验报告文件名（产出契约的一部分）
const 校验报告文件: &str = "校验报告.md";

/// 文档生成编排器
pub struct 编排器 {
    配置: 生成配置,
    叙述器: Arc<dyn 叙述器>,
}

impl 编排器 {
    /// 以模板叙述器构造（确定性、离线）
    pub fn 新(配置: 生成配置) -> Self {
        Self {
            配置,
            叙述器: Arc::new(模板叙述器),
        }
    }

    /// 以指定叙述器构造（模型叙述器由上层注入，本府不接 LLM）
    pub fn 带叙述器(配置: 生成配置, 叙述器: Arc<dyn 叙述器>) -> Self {
        Self { 配置, 叙述器 }
    }

    /// 执行整批生成
    pub fn 运行(&self) -> Result<生成结果> {
        let 索引 = self.读索引()?;
        let 事实们 = 投影事实(&索引, self.配置.范围.as_deref());

        let 篇目录 = self.配置.输出目录.join("篇");
        建目录(&篇目录)?;

        let mut 校验 = Vec::new();
        let mut 降级篇数 = 0usize;
        for 事实 in &事实们 {
            let (正文, 降级) = self.取得叙述(事实);
            if 降级 {
                降级篇数 += 1;
            }
            let 结论 = 校验叙述(事实, &正文);
            let 文档 = 渲染文档(事实, &正文, &结论);
            写文本(&篇目录.join(文档文件名(&事实.文件)), &文档)?;
            校验.push(结论);
        }

        let 结果 = 生成结果 {
            篇数: 事实们.len(),
            符号数: 事实们.iter().map(|事实| 事实.定义.len()).sum(),
            边数: 事实们.iter().map(|事实| 事实.出边.len()).sum(),
            叙述器: self.叙述器.name().to_string(),
            降级篇数,
            校验,
            自检: 自检(),
        };

        写文本(
            &self.配置.输出目录.join(索引文件),
            &渲染索引(&事实们, &结果),
        )?;
        写文本(
            &self.配置.输出目录.join(校验报告文件),
            &渲染校验报告(&结果),
        )?;
        Ok(结果)
    }

    /// 读事实源；读不到即明确失败，不产出空文档集
    fn 读索引(&self) -> Result<符号索引> {
        let 路径 = &self.配置.索引路径;
        let 文本 = fs::read_to_string(路径)
            .map_err(|原因| Error::Other(format!("读取符号索引失败（{}）：{原因}", 路径.display())))?;
        serde_json::from_str(&文本)
            .map_err(|原因| Error::Other(format!("符号索引反序列化失败：{原因}")))
    }

    /// 叙述一篇；失败则逐篇回落模板叙述器，不中断整批
    fn 取得叙述(&self, 事实: &文档事实) -> (String, bool) {
        match self.叙述器.叙述(事实) {
            Ok(文本) => (文本, false),
            Err(_) => {
                let 兜底 = 模板叙述器.叙述(事实).unwrap_or_default();
                (兜底, true)
            }
        }
    }
}

/// 把符号索引投影成每文件一份事实
pub fn 投影事实(索引: &符号索引, 范围: Option<&str>) -> Vec<文档事实> {
    let 归属 = 符号归属表(索引);
    let mut 表: BTreeMap<String, 文档事实> = BTreeMap::new();

    for 记 in &索引.符号 {
        if !在范围(&记.符号.文件, 范围) {
            continue;
        }
        let 项 = 表
            .entry(记.符号.文件.clone())
            .or_insert_with(|| 空事实(&记.符号.文件, &记.坐标, &记.来源语言));
        项.定义.push(记.符号.clone());
    }

    for 边 in &索引.边 {
        if let Some(源文件) = 归属.get(&边.从) {
            if 在范围(源文件, 范围) {
                if let Some(项) = 表.get_mut(源文件) {
                    项.出边.push(边.clone());
                }
            }
        }
        if let Some(目标文件) = 归属.get(&边.到) {
            if 在范围(目标文件, 范围) {
                if let Some(项) = 表.get_mut(目标文件) {
                    项.入边.push(边.clone());
                }
            }
        }
    }

    for 悬 in &索引.悬空 {
        if let Some(源文件) = 归属.get(&悬.从) {
            if 在范围(源文件, 范围) {
                if let Some(项) = 表.get_mut(源文件) {
                    项.悬空.push(悬.clone());
                }
            }
        }
    }

    表.into_values().collect()
}

/// 符号 id → 所属文件
fn 符号归属表(索引: &符号索引) -> BTreeMap<String, String> {
    let mut 表 = BTreeMap::new();
    for 记 in &索引.符号 {
        表.insert(记.符号.id.clone(), 记.符号.文件.clone());
    }
    表
}

fn 空事实(文件: &str, 坐标: &BTreeMap<String, String>, 来源语言: &str) -> 文档事实 {
    文档事实 {
        文件: 文件.to_string(),
        坐标: 坐标.clone(),
        来源语言: 来源语言.to_string(),
        定义: Vec::new(),
        出边: Vec::new(),
        入边: Vec::new(),
        悬空: Vec::new(),
    }
}

fn 在范围(文件: &str, 范围: Option<&str>) -> bool {
    match 范围 {
        None => true,
        Some(前缀) => 文件.starts_with(前缀),
    }
}

fn 建目录(路径: &Path) -> Result<()> {
    fs::create_dir_all(路径)
        .map_err(|原因| Error::Other(format!("创建产出目录失败（{}）：{原因}", 路径.display())))
}

fn 写文本(路径: &Path, 内容: &str) -> Result<()> {
    fs::write(路径, 内容)
        .map_err(|原因| Error::Other(format!("写入失败（{}）：{原因}", 路径.display())))
}
