//! 主政 - 落笔 - 园：鸿钧主政落笔（化要求 / 确认设计 / 验收裁决）。

use crate::类型_定义_殿::*;
use moxing_fu::{调用模型, 对话消息, 模型配置};

/// 化为要求：来源 + 方向 + 类别 + 验收标准 → 要求书。
pub fn 化为要求(
    id: &str,
    来源: 要求来源,
    阶段: 阶段,
    方向: &str,
    类别: 要求类别,
    验收标准: &str,
    优先级: 优先级,
) -> 要求书 {
    要求书 {
        id: id.to_string(),
        来源,
        想法id: None,
        阶段,
        方向: 方向.to_string(),
        类别,
        验收标准: 验收标准.to_string(),
        约束: 约束 { 涉及路径: vec![], 不允许: vec![], 优先级 },
        状态: 要求状态::待领,
        确认意见: None,
        验收: None,
        版本: None,
    }
}

/// 解析想法：模型一次调用，把想法内容结构化为要求书（带项目记忆背景）。
pub fn 解析想法(id: &str, 想法内容: &str, 背景: &str, 配置: &模型配置) -> Result<要求书, String> {
    let 背景 = if 背景.is_empty() { "（无项目记忆）" } else { 背景 };
    let 提示 = format!(
        "项目记忆背景：\n{}\n\n把下面的想法结构化为一个开发要求，只输出 JSON：{{\"方向\":\"一句话目标\",\"类别\":\"功能|性能|美观|优化|维护|新能力|补基础\",\"验收标准\":\"可核对的完成判据\"}}\n想法：{}",
        背景, 想法内容
    );
    let 回复 = 调用模型(配置, &[对话消息::用户(提示)])?;
    let 干净 = 提取JSON(&回复).map_err(|错误| format!("解析想法失败: {错误}"))?;
    let 解析: serde_json::Value =
        serde_json::from_str(&干净).map_err(|错误| format!("解析想法失败: {错误}"))?;
    let 方向 = 解析["方向"].as_str().unwrap_or("未命名").to_string();
    let 类别 = 解析["类别"].as_str().unwrap_or("功能").to_string();
    let 验收标准 = 解析["验收标准"].as_str().unwrap_or("").to_string();
    Ok(化为要求(id, 要求来源::界主, 阶段::甲, &方向, 解析类别(&类别), &验收标准, 优先级::中))
}

/// 从模型回复中提取 JSON：剥 markdown 围栏，取首个 { 到最后一个 }。
fn 提取JSON(文本: &str) -> Result<String, String> {
    let 文本 = 文本.trim();
    let 文本 = 文本.trim_start_matches("```json").trim_start_matches("```").trim();
    let 开始 = 文本.find('{').ok_or_else(|| format!("模型未返回 JSON：{文本}"))?;
    let 结束 = 文本.rfind('}').ok_or_else(|| format!("模型未返回 JSON：{文本}"))?;
    Ok(文本[开始..=结束].to_string())
}

/// 类别字符串 → 枚举。
pub fn 解析类别(类别: &str) -> 要求类别 {
    match 类别 {
        "性能" => 要求类别::性能,
        "美观" => 要求类别::美观,
        "优化" => 要求类别::优化,
        "维护" => 要求类别::维护,
        "新能力" => 要求类别::新能力,
        "补基础" => 要求类别::补基础,
        _ => 要求类别::功能,
    }
}

/// 确认设计：机械校验（拆解非空 / 工作流合法 / 自评非空）。
pub fn 确认设计(方案: &设计方案) -> 验收结论 {
    let 拆解合法 = !方案.拆解.is_empty()
        && 方案.拆解.iter().all(|项| !项.目标.is_empty() && 合法工作流(&项.工作流));
    if 拆解合法 && !方案.自评.is_empty() {
        验收结论::通过
    } else {
        验收结论::打回
    }
}

/// 工作流标识是否合法。
pub fn 合法工作流(工作流: &str) -> bool {
    matches!(工作流, "L1_qa" | "L2_script" | "L3_program" | "L4_complex")
}

/// 验收裁决：产物清单 → 机械校验（有产物即通过）。
pub fn 验收裁决(要求id: &str, 产物们: &[产物条目], 耗时秒: f64) -> 验收回执 {
    let 结论 = if 产物们.is_empty() { 验收结论::打回 } else { 验收结论::通过 };
    let 验收意见 = if 结论 == 验收结论::打回 {
        Some("实现层：无产物".to_string())
    } else {
        None
    };
    验收回执 { 要求id: 要求id.to_string(), 结论, 验收意见, 产物: 产物们.to_vec(), 耗时秒 }
}

/// 定档：验收回执 → 回填识海承载-府的「验收结果」格位。
pub fn 定档(存储: &shihai_fu::模型存储, 回执: &验收回执) -> Result<(), String> {
    存储.写记录(&shihai_fu::记录::新(
        "验收结果",
        &format!("{}：{:?}", 回执.要求id, 回执.结论),
        &format!("验收裁决「{}」", 回执.要求id),
        "代码",
    ))
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 确认设计拒空拆解() {
        let 方案 = 设计方案 { 要求id: "r1".to_string(), 设计: "".to_string(), 拆解: vec![], 自评: "自评".to_string() };
        assert_eq!(确认设计(&方案), 验收结论::打回);
    }

    #[test]
    fn 确认设计通过合法方案() {
        let 方案 = 设计方案 {
            要求id: "r1".to_string(),
            设计: "设计".to_string(),
            拆解: vec![拆解项 { 目标: "目标".to_string(), 执行层角色: vec![], 工作流: "L2_script".to_string() }],
            自评: "自评".to_string(),
        };
        assert_eq!(确认设计(&方案), 验收结论::通过);
    }

    #[test]
    fn 验收无产物打回() {
        let 回执 = 验收裁决("r1", &[], 0.0);
        assert_eq!(回执.结论, 验收结论::打回);
    }
}
