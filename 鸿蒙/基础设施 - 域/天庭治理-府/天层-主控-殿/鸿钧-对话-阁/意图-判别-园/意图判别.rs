//! 意图判别-园：界主消息 → LLM 判意图 + 提取关键字段。
//! 兜底纪律：调用失败/解析失败一律回退「闲聊」，不阻塞对话（与需求拆分同款机械兜底）。

use moxing_fu::{对话消息, 模型配置, 精简上限, 调用模型};
use rizhi_fu::{info, warn};
use serde::{Deserialize, Serialize};
use shihai_fu::世界结果;

/// 对话意图（界主一句话的归类）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 对话意图 {
    闲聊,
    发布任务,
    追问进度,
    中途干预,
    点名角色,
}

/// 意图判别结果（含发布任务所需的关键字段）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 判别结果 {
    pub 意图: 对话意图,
    /// 发布任务时的任务方向。
    pub 方向: String,
    /// 发布任务时的验收标准（可空，后续对话澄清）。
    pub 验收标准: String,
    /// 发布任务时界主明确提到的涉及路径（可空）。
    pub 涉及路径: Vec<String>,
    /// 点名时的角色名（可空）。
    pub 点名角色: Option<String>,
}

/// LLM 判意图。任何失败回退 闲聊（机械兜底，不阻塞对话）。
pub fn 判别(消息: &str, 配置: &模型配置) -> 判别结果 {
    let 提示 = format!(
        "你是鸿钧，天层主政之神，界主的对话伙伴。界主刚对你说了一句话，请判断这句话的意图并提取关键信息。\n\
意图只能是以下之一：\n\
- 闲聊：问候、闲聊、情绪表达、与任务无关的话题\n\
- 发布任务：界主要世界去做什么（实现功能/写代码/查证/优化等）\n\
- 追问进度：询问某个任务或世界的进展、状态、结果\n\
- 中途干预：要求停止、打回、加急、修改正在进行的任务\n\
- 点名角色：消息中 @ 了某个角色（女娲/老子/元始/通天/后土/多宝/白泽/龟灵/玄天/红云/镇元子/鲲鹏/神农/冥河/轩辕）\n\
输出严格 JSON（不要任何其他文字，不要 markdown 围栏）：\n\
{{\"意图\":\"闲聊|发布任务|追问进度|中途干预|点名角色\",\"方向\":\"发布任务时的任务方向，否则空字符串\",\"验收标准\":\"发布任务时的验收标准，没有则空字符串\",\"涉及路径\":[\"发布任务时界主明确提到的路径，没有则空数组\"],\"点名角色\":\"点名时的角色名，没有则 null\"}}\n\
界主的话：{消息}"
    );
    match 调用模型(配置, &[对话消息::用户(&提示)], 精简上限) {
        Ok((回复, _)) => match 解析(回复) {
            Ok(结果) => {
                info!(意图 = ?结果.意图, "意图判别完成");
                结果
            }
            Err(错误) => {
                warn!(错误 = %错误, "意图判别解析失败，回退闲聊");
                兜底闲聊()
            }
        },
        Err(错误) => {
            warn!(错误 = %错误, "意图判别调用失败，回退闲聊");
            兜底闲聊()
        }
    }
}

/// 兜底结果：判不出就当闲聊。
fn 兜底闲聊() -> 判别结果 {
    判别结果 {
        意图: 对话意图::闲聊,
        方向: String::new(),
        验收标准: String::new(),
        涉及路径: Vec::new(),
        点名角色: None,
    }
}

/// 解析模型回复：取首个完整 JSON 对象 → 映射意图枚举（容错别名）。
fn 解析(回复: String) -> 世界结果<判别结果> {
    let 对象 = 提取首个对象(&回复)?;
    let 意图文本 = 对象.get("意图").and_then(|值| 值.as_str()).unwrap_or("");
    let 意图 = 映射意图(意图文本);
    Ok(判别结果 {
        意图,
        方向: 对象
            .get("方向")
            .and_then(|值| 值.as_str())
            .unwrap_or("")
            .to_string(),
        验收标准: 对象
            .get("验收标准")
            .and_then(|值| 值.as_str())
            .unwrap_or("")
            .to_string(),
        涉及路径: 对象
            .get("涉及路径")
            .and_then(|值| 值.as_array())
            .map(|数组| {
                数组
                    .iter()
                    .filter_map(|值| 值.as_str().map(|文本| 文本.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        点名角色: 对象
            .get("点名角色")
            .and_then(|值| 值.as_str())
            .map(|文本| 文本.to_string()),
    })
}

/// 意图文本 → 枚举（容错：模型可能输出别名或夹杂解释）。
/// 顺序：先判更具体的（干预/追问/点名），再判任务，最后闲聊——「停止任务」不能被误判为发布。
fn 映射意图(文本: &str) -> 对话意图 {
    if 文本.contains("干预")
        || 文本.contains("停止")
        || 文本.contains("中止")
        || 文本.contains("加急")
        || 文本.contains("打回")
    {
        对话意图::中途干预
    } else if 文本.contains("追问")
        || 文本.contains("进度")
        || 文本.contains("进展")
        || 文本.contains("状态")
    {
        对话意图::追问进度
    } else if 文本.contains("点名") || 文本.contains('@') {
        对话意图::点名角色
    } else if 文本.contains("发布") || 文本.contains("任务") {
        对话意图::发布任务
    } else {
        对话意图::闲聊
    }
}

/// 提取首个完整可解析的 JSON 对象：字符串字面量感知的平衡括号扫描
/// （think 块/解释文字中的花括号按字符串与配对正确跳过，取第一个合法对象）。
fn 提取首个对象(文本: &str) -> 世界结果<serde_json::Value> {
    let 字符们: Vec<char> = 文本.chars().collect();
    let mut 深度 = 0i32;
    let mut 起点 = None;
    let mut 在字符串 = false;
    let mut 转义 = false;
    for (索引, &字符) in 字符们.iter().enumerate() {
        if 在字符串 {
            if 转义 {
                转义 = false;
            } else if 字符 == '\\' {
                转义 = true;
            } else if 字符 == '"' {
                在字符串 = false;
            }
            continue;
        }
        match 字符 {
            '"' => 在字符串 = true,
            '{' => {
                if 深度 == 0 {
                    起点 = Some(索引);
                }
                深度 += 1;
            }
            '}' => {
                深度 -= 1;
                if 深度 == 0 {
                    if let Some(起) = 起点 {
                        let 片段: String = 字符们[起..=索引].iter().collect();
                        if let Ok(值) = serde_json::from_str::<serde_json::Value>(&片段) {
                            return Ok(值);
                        }
                    }
                    起点 = None;
                }
            }
            _ => {}
        }
    }
    Err("未找到完整 JSON 对象".into())
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 解析_发布任务_带字段() {
        let 结果 = 解析(r#"{"意图":"发布任务","方向":"新增一个世界 昼夜 命令","验收标准":"cargo test 通过","涉及路径":["乾坤/呈现-域/命令操作-府"],"点名角色":null}"#.to_string()).unwrap();
        assert_eq!(结果.意图, 对话意图::发布任务);
        assert_eq!(结果.方向, "新增一个世界 昼夜 命令");
        assert_eq!(结果.涉及路径.len(), 1);
        assert!(结果.点名角色.is_none());
    }

    #[test]
    fn 解析_点名角色() {
        let 结果 = 解析(
            r#"{"意图":"点名角色","方向":"","验收标准":"","涉及路径":[],"点名角色":"女娲"}"#
                .to_string(),
        )
        .unwrap();
        assert_eq!(结果.意图, 对话意图::点名角色);
        assert_eq!(结果.点名角色.as_deref(), Some("女娲"));
    }

    #[test]
    fn 解析_think块夹杂_取首个对象() {
        let 回复 = "好的，我来判断。<think>界主想新增命令，应该是发布任务。</think>解释文字 {不是JSON} 真正的JSON如下：\n{\"意图\":\"发布任务\",\"方向\":\"写一个测试\",\"验收标准\":\"\",\"涉及路径\":[],\"点名角色\":null}";
        let 结果 = 解析(回复.to_string()).unwrap();
        assert_eq!(结果.意图, 对话意图::发布任务);
        assert_eq!(结果.方向, "写一个测试");
    }

    #[test]
    fn 解析_无_json_报错() {
        assert!(解析("我不知道".to_string()).is_err());
    }

    #[test]
    fn 映射意图_别名容错() {
        assert_eq!(映射意图("发布任务"), 对话意图::发布任务);
        assert_eq!(映射意图("任务"), 对话意图::发布任务);
        assert_eq!(映射意图("询问进度"), 对话意图::追问进度);
        assert_eq!(映射意图("停止任务"), 对话意图::中途干预);
        assert_eq!(映射意图("@女娲"), 对话意图::点名角色);
        assert_eq!(映射意图("今天天气不错"), 对话意图::闲聊);
    }
}
