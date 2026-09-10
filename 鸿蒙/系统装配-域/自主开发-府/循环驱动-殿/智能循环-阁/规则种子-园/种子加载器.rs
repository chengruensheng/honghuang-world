use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use hm_cognition::{心智地图, 格位, 维度, 维度载荷, 规则载荷, 规则层级, 严重度};

/// 规则种子文件所在目录（项目根/rules，与规则单源决策一致）
const 规则目录: &str = "rules";
/// 规则种子文件扩展名
const 规则扩展名: &str = "md";

/// 规则种子：从 .md 文件解析出的结构化规则条目
#[derive(Debug, Clone)]
pub struct 规则种子 {
    pub 名称: String,
    pub 层级: 规则层级,
    pub 摘要: String,
}

/// 解析单个 .md 规则种子文件：提取 frontmatter 中的层级 + 正文摘要。
/// frontmatter 格式：`---\n层级: 大道|天道|临时\n---`
/// 摘要取正文前 200 字符（去除 frontmatter 后）。
fn 解析种子文件(路径: &Path) -> Option<规则种子> {
    let 内容 = fs::read_to_string(路径).ok()?;
    let 文件名 = 路径.file_stem()?.to_string_lossy().to_string();

    // 解析 frontmatter
    let (层级, 正文) = if 内容.starts_with("---") {
        if let Some(结束位置) = 内容[3..].find("---") {
            let frontmatter = &内容[3..3 + 结束位置];
            let 正文起始 = 3 + 结束位置 + 3;
            let 正文 = 内容[正文起始..].trim_start();
            let 层级 = 解析层级(frontmatter);
            (层级, 正文.to_string())
        } else {
            (规则层级::天道, 内容.clone())
        }
    } else {
        (规则层级::天道, 内容.clone())
    };

    // 摘要：取正文前 200 字符（按 char 计数，避免 UTF-8 截断），截断到完整行
    let 摘要 = if 正文.chars().count() > 200 {
        let 截断: String = 正文.chars().take(200).collect();
        if let Some(换行) = 截断.rfind('\n') {
            截断[..换行].to_string()
        } else {
            截断
        }
    } else {
        正文.clone()
    };

    Some(规则种子 {
        名称: 文件名,
        层级,
        摘要: 摘要.trim().to_string(),
    })
}

/// 从 frontmatter 文本中解析层级字段
fn 解析层级(frontmatter: &str) -> 规则层级 {
    for 行 in frontmatter.lines() {
        let 行 = 行.trim();
        if 行.starts_with("层级:") || 行.starts_with("层级：") {
            let 值 = 行.split_once(':').or_else(|| 行.split_once('：'))
                .map(|(_, v)| v.trim())
                .filter(|v| !v.is_empty());
            return match 值 {
                Some("大道") => 规则层级::大道,
                Some("天道") => 规则层级::天道,
                Some("临时") => 规则层级::临时,
                _ => {
                    tracing::warn!("规则种子层级标注缺失或未知，按天道处理: {行}");
                    规则层级::天道
                }
            };
        }
    }
    规则层级::天道
}

/// 加载规则种子目录下的所有 .md 文件，返回解析成功的种子列表。
/// 目录不存在或为空时返回空列表（不报错，静默跳过）。
pub fn 加载规则种子(目录: &str) -> Vec<规则种子> {
    let 路径 = Path::new(目录);
    if !路径.is_dir() {
        tracing::warn!("规则种子目录不存在: {目录}");
        return Vec::new();
    }
    let mut 种子们 = Vec::new();
    let entries = match fs::read_dir(路径) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!("读取规则种子目录失败: {e}");
            return Vec::new();
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(种子) = 解析种子文件(&path) {
                tracing::info!(
                    "规则种子已加载: {} (层级={:?}, 摘要长度={})",
                    种子.名称, 种子.层级, 种子.摘要.len()
                );
                种子们.push(种子);
            }
        }
    }
    tracing::info!("规则种子加载完成: {} 个文件", 种子们.len());
    种子们
}

/// 把规则种子写入心智地图的规则维度格位。
/// 每个种子以文件名为格位名（保证唯一），大道层级置信度 1.0，天道 0.9，临时 0.8。
pub fn 注入种子到格位(心智: &Arc<Mutex<心智地图>>, 种子们: &[规则种子]) {
    let mut 心智 = 心智.lock().expect("心智锁中毒");

    for 种子 in 种子们.iter() {
        let 格位名 = &种子.名称;
        let 可信度 = match 种子.层级 {
            规则层级::大道 => 1.0,
            规则层级::天道 => 0.9,
            规则层级::临时 => 0.8,
        };
        let 载荷 = 规则载荷 {
            层级: 种子.层级.clone(),
            触发条件: format!("种子规则:{}", 种子.名称),
            严重度: if 种子.层级 == 规则层级::大道 { 严重度::红线 } else { 严重度::警告 },
            例外条款: Vec::new(),
            历史违反次数: 0,
        };
        // upsert：已有同名格位则更新，否则新增
        let 新格位 = 格位::新(
            维度::规则,
            格位名,
            &种子.摘要,
            可信度,
            vec![format!("{规则目录}/{}.{}", 种子.名称, 规则扩展名)],
            Some(维度载荷::规则(载荷)),
        );
        match 心智.更新格位(新格位.clone()) {
            Ok(()) => {},
            Err(_) => {
                心智.添加格位(新格位).unwrap_or_else(|e| {
                    tracing::warn!("注入规则种子到格位失败: {} -> {e}", 种子.名称);
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 解析层级_大道() {
        assert_eq!(解析层级("层级: 大道"), 规则层级::大道);
    }

    #[test]
    fn 解析层级_天道() {
        assert_eq!(解析层级("层级: 天道"), 规则层级::天道);
    }

    #[test]
    fn 解析层级_中文冒号() {
        assert_eq!(解析层级("层级：临时"), 规则层级::临时);
    }

    #[test]
    fn 解析层级_缺失默认天道() {
        assert_eq!(解析层级("其他字段: 值"), 规则层级::天道);
    }

    #[test]
    fn 加载规则种子_实际目录() {
        // cargo test 工作目录为 crate 根，需向上两级到 workspace 根
        let 候选路径 = ["rules", "../../rules", "../../../rules"];
        let mut 种子们 = Vec::new();
        for 路径 in &候选路径 {
            种子们 = 加载规则种子(路径);
            if !种子们.is_empty() {
                break;
            }
        }
        // rules/ 目录下至少有 7 个大道 + 2 个天道 = 9 个文件
        assert!(种子们.len() >= 9, "应加载至少 9 个种子，实际: {}", 种子们.len());
        let 大道数 = 种子们.iter().filter(|s| s.层级 == 规则层级::大道).count();
        let 天道数 = 种子们.iter().filter(|s| s.层级 == 规则层级::天道).count();
        assert!(大道数 >= 7, "大道规则应 ≥7，实际: {大道数}");
        assert!(天道数 >= 2, "天道规则应 ≥2，实际: {天道数}");
    }
}
