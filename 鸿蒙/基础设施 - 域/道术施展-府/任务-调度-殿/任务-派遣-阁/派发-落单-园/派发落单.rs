//! 派发 - 落单 - 园：把执行任务派发落单，真实写文件 + 构建验证，跟踪状态，收产物，失败重试。

use crate::类型_定义_殿::{执行任务, 执行状态, 产物条目, 执行回执};
use crate::{写文件, 读文件, 运行命令};
use moxing_fu::{调用模型, 对话消息, 模型配置};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// 最大重试次数（≤2 次）。
const 最大重试: u32 = 2;

/// 落盘文件：LLM 声明要写的一个文件。
#[derive(Deserialize)]
struct 落盘文件 {
    路径: String,
    内容: String,
}

/// 落盘指令：LLM 输出的结构化文件落盘清单。
#[derive(Deserialize)]
struct 落盘指令 {
    文件们: Vec<落盘文件>,
    说明: String,
}

/// 任务调度：派遣执行任务，真实落盘改代码，跟踪状态，收产物。
pub struct 任务调度 {
    回执们: HashMap<String, 执行回执>,
    重试次数: HashMap<String, u32>,
    配置: 模型配置,
    工作区根: PathBuf,
}

impl 任务调度 {
    /// 新建调度器（持有模型配置与工作区根）。
    pub fn 新(配置: 模型配置, 工作区根: PathBuf) -> 任务调度 {
        任务调度 { 回执们: HashMap::new(), 重试次数: HashMap::new(), 配置, 工作区根 }
    }

    /// 派遣执行：让 LLM 输出结构化落盘指令，真实写文件，跑 cargo build 验证，失败回喂重试。
    pub fn 派遣执行(&mut self, 任务id: &str, 任务: &执行任务, 背景: &str) -> Result<执行回执, String> {
        let 现状 = self.读现状(任务, 背景)?;
        let 初提示 = format!(
            "你是执行角色，要在项目里真实写代码。\n\n项目背景（目录 / 文件 / 结构）：\n{}\n\n相关文件现状：\n{}\n\n任务目标：{}\n\n只输出一个 JSON（不要任何其它文字）：\n{{\"文件们\":[{{\"路径\":\"相对项目根的文件路径\",\"内容\":\"该文件的完整内容\"}}],\"说明\":\"一句话说明做了什么\"}}\n\n规则：路径必须相对项目根；内容必须是该文件的完整内容，不能是片段；只新增或修改必要的文件。",
            背景, 现状, 任务.目标
        );

        let mut 下一提示 = 初提示;
        let mut 尝试 = 0u32;
        loop {
            let 回复 = 调用模型(&self.配置, &[对话消息::用户(&下一提示)])?;
            let 干净 = 提取JSON(&回复)?;
            let 指令: 落盘指令 = serde_json::from_str(&干净).map_err(|错误| format!("解析落盘指令失败: {错误}"))?;

            let mut 产物们 = Vec::new();
            for 文件 in &指令.文件们 {
                let 绝对路径 = self.工作区根.join(&文件.路径);
                let 路径文本 = 绝对路径.to_str().ok_or_else(|| "路径含非 UTF-8 字符".to_string())?;
                写文件(路径文本, &文件.内容)?;
                产物们.push(产物条目 {
                    路径: 文件.路径.clone(),
                    类别: "代码".to_string(),
                    字节数: 文件.内容.len() as u64,
                });
            }

            // 真实构建验证
            let 构建 = 运行命令("cargo", &["build", "--workspace"], self.工作区根.to_str())?;
            if 构建.退出码 == Some(0) {
                let 回执 = 执行回执 {
                    状态: 执行状态::成功,
                    产物们,
                    说明: format!("{}；cargo build 通过", 指令.说明),
                };
                self.回执们.insert(任务id.to_string(), 回执.clone());
                return Ok(回执);
            }

            尝试 += 1;
            if 尝试 >= 最大重试 {
                let 回执 = 执行回执 {
                    状态: 执行状态::失败,
                    产物们,
                    说明: format!("cargo build 失败（重试 {尝试} 次）：{}", 构建.标准错误),
                };
                self.回执们.insert(任务id.to_string(), 回执.clone());
                return Ok(回执);
            }
            下一提示 = format!(
                "上一次你输出的文件落盘后，cargo build 失败，报错如下：\n{}\n\n请修正后重新输出完整 JSON（必须包含所有需要文件的完整内容，不能只写改动片段）：",
                构建.标准错误
            );
        }
    }

    /// 读现状：让 LLM 决定要读哪些文件，系统读回内容，供写阶段参考。
    fn 读现状(&self, 任务: &执行任务, 背景: &str) -> Result<String, String> {
        let 读提示 = format!(
            "你要在项目里完成一个开发任务。\n\n项目背景（目录 / 文件 / 结构）：\n{}\n\n任务目标：{}\n\n为了准确改代码，请列出你需要先读取现状的文件路径。只输出一个 JSON（不要其它文字）：\n{{\"读文件们\":[\"相对项目根的文件路径\"]}}\n\n若任务只是新建独立文件、无需读现状，输出 {{\"读文件们\":[]}}。",
            背景, 任务.目标
        );
        let 回复 = 调用模型(&self.配置, &[对话消息::用户(&读提示)])?;
        let 干净 = 提取JSON(&回复)?;
        let 值: serde_json::Value = serde_json::from_str(&干净).map_err(|错误| format!("解析读请求失败: {错误}"))?;

        let mut 现状 = String::new();
        if let Some(路径们) = 值["读文件们"].as_array() {
            for 路径 in 路径们 {
                if let Some(路径) = 路径.as_str() {
                    let 绝对 = self.工作区根.join(路径);
                    match 读文件(绝对.to_str().unwrap_or("")) {
                        Ok(内容) => 现状.push_str(&format!("【文件：{路径}】\n{内容}\n\n")),
                        Err(_) => 现状.push_str(&format!("【文件：{路径}】\n（读取失败或不存在）\n\n")),
                    }
                }
            }
        }
        if 现状.is_empty() {
            现状.push_str("（无需读取现状）");
        }
        Ok(现状)
    }

    /// 查任务执行状态。
    pub fn 查状态(&self, 任务id: &str) -> Option<执行状态> {
        self.回执们.get(任务id).map(|回执| 回执.状态.clone())
    }

    /// 收产物清单。
    pub fn 收产物(&self, 任务id: &str) -> Option<Vec<产物条目>> {
        self.回执们.get(任务id).map(|回执| 回执.产物们.clone())
    }

    /// 失败重试：仅在重试次数未满时放行。
    pub fn 可重试(&self, 任务id: &str) -> bool {
        self.重试次数.get(任务id).copied().unwrap_or(0) < 最大重试
    }

    /// 登记一次重试。
    pub fn 记重试(&mut self, 任务id: &str) {
        let 次数 = self.重试次数.entry(任务id.to_string()).or_insert(0);
        *次数 += 1;
    }
}

/// 从模型回复中提取 JSON：剥 markdown 围栏，取首个 { 到最后一个 }。
fn 提取JSON(文本: &str) -> Result<String, String> {
    let 文本 = 文本.trim();
    let 文本 = 文本.trim_start_matches("```json").trim_start_matches("```").trim();
    let 开始 = 文本.find('{').ok_or_else(|| format!("模型未返回 JSON：{文本}"))?;
    let 结束 = 文本.rfind('}').ok_or_else(|| format!("模型未返回 JSON：{文本}"))?;
    Ok(文本[开始..=结束].to_string())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 造配置() -> 模型配置 {
        模型配置 { 密钥: "k".to_string(), 地址: "http://127.0.0.1:1".to_string(), 模型: "m".to_string() }
    }

    #[test]
    fn 重试不超过两次() {
        let 调度 = 任务调度::新(造配置(), PathBuf::from("."));
        assert!(调度.可重试("t1"));
    }

    #[test]
    fn 记重试后达上限() {
        let mut 调度 = 任务调度::新(造配置(), PathBuf::from("."));
        调度.记重试("t1");
        调度.记重试("t1");
        assert!(!调度.可重试("t1"));
    }
}
