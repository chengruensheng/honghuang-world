//! 快照 - 落库 - 园：源码快照、版本记录与世界状态读写。
//!
//! 版本记录追加到 `.上下文/状态/版本.jsonl`（一行一条，append 模式）。
//! 世界状态是单一对象，写入 `.上下文/状态/世界状态.jsonl`（原子覆盖：临时文件 + rename）。
//! 「版本 存档」完成后标记 `v1已存档=true`（甲→乙阶段唯一切换点）。

use crate::类型_定义_殿::{阶段, 世界状态, 版本记录, 要求书};
use rizhi_fu::{debug, error, info, warn};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 复制源码到快照目录，排除构建产物/版本库/依赖，返回复制文件数。
pub fn 源码快照(源目录: &Path, 目标目录: &Path) -> Result<usize, String> {
    let 排除项 = shihai_fu::扫描排除项(源目录);
    let mut 计数 = 0;
    复制目录(源目录, 目标目录, &排除项, &mut 计数)?;
    Ok(计数)
}

/// 增量源码快照（设计稿 §5「版本递增与增量快照」）：
/// 相对上一版基线只复制「新增/修改」文件（同名且字节相同跳过），变动文件按相对路径落目标目录；
/// 基目录不存在时退化为全量复制（即 v1 场景）。
/// 返回 (复制文件数, 变更清单[(相对路径, 字节数)])；全量退化时清单为空。
pub fn 增量快照(
    源目录: &Path,
    基目录: Option<&Path>,
    目标目录: &Path,
) -> Result<(usize, Vec<(String, u64)>), String> {
    let 排除项 = shihai_fu::扫描排除项(源目录);
    let 有基 = 基目录.map(|路径| 路径.exists()).unwrap_or(false);
    if !有基 {
        let mut 计数 = 0;
        复制目录(源目录, 目标目录, &排除项, &mut 计数)?;
        return Ok((计数, Vec::new()));
    }
    let 基 = 基目录.expect("有基即 Some");
    let mut 计数 = 0;
    let mut 清单 = Vec::new();
    复制变更目录(源目录, 基, 目标目录, &排除项, &mut 计数, &mut 清单)?;
    Ok((计数, 清单))
}

/// 递归复制「相对基目录不存在或字节不同」的文件；目录结构按需创建，仅目录不变更（空目录不落盘）。
fn 复制变更目录(
    源: &Path,
    基: &Path,
    目标: &Path,
    排除项: &[String],
    计数: &mut usize,
    清单: &mut Vec<(String, u64)>,
) -> Result<(), String> {
    fs::create_dir_all(目标).map_err(|错误| format!("建目录失败: {错误}"))?;
    for 条目 in fs::read_dir(源).map_err(|错误| format!("读目录失败: {错误}"))? {
        let 条目 = 条目.map_err(|错误| format!("读条目失败: {错误}"))?;
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 排除项.iter().any(|项| 项 == &名) {
            continue;
        }
        let 源路径 = 条目.path();
        let 相对 = match 源路径.strip_prefix(源) {
            Ok(相对) => 相对,
            Err(_) => continue,
        };
        let 基路径 = 基.join(相对);
        let 目标路径 = 目标.join(相对);
        if 源路径.is_dir() {
            复制变更目录(&源路径, &基路径, &目标路径, 排除项, 计数, 清单)?;
        } else {
            let 源码大小 = 条目
                .metadata()
                .map_err(|错误| format!("读元数据失败: {错误}"))?
                .len();
            let 需复制 = match fs::metadata(&基路径) {
                Ok(元) => 元.len() != 源码大小, // 同名同字节跳过外，其余视为修改
                Err(_) => true,                  // 基目录无此文件 → 新增
            };
            if 需复制 {
                fs::copy(&源路径, &目标路径).map_err(|错误| format!("复制文件失败: {错误}"))?;
                *计数 += 1;
                清单.push((相对.to_string_lossy().to_string(), 源码大小));
            }
        }
    }
    Ok(())
}

fn 复制目录(源: &Path, 目标: &Path, 排除项: &[String], 计数: &mut usize) -> Result<(), String> {
    fs::create_dir_all(目标).map_err(|错误| format!("建目录失败: {错误}"))?;
    for 条目 in fs::read_dir(源).map_err(|错误| format!("读目录失败: {错误}"))? {
        let 条目 = 条目.map_err(|错误| format!("读条目失败: {错误}"))?;
        let 名 = 条目.file_name().to_string_lossy().to_string();
        if 排除项.iter().any(|项| 项 == &名) {
            continue;
        }
        let 源路径 = 条目.path();
        let 目标路径 = 目标.join(&名);
        if 源路径.is_dir() {
            复制目录(&源路径, &目标路径, 排除项, 计数)?;
        } else {
            fs::copy(&源路径, &目标路径).map_err(|错误| format!("复制文件失败: {错误}"))?;
            *计数 += 1;
        }
    }
    Ok(())
}

/// 生成一条版本记录。
pub fn 生成版本记录(
    版本号: &str,
    时间: u64,
    阶段: 阶段,
    改了什么: &str,
    源码快照路径: &str,
    构建产物路径: &str,
    验收结论: Vec<String>,
    对比上一版: &str,
) -> 版本记录 {
    版本记录 {
        版本号: 版本号.to_string(),
        时间,
        阶段,
        改了什么: 改了什么.to_string(),
        源码快照路径: 源码快照路径.to_string(),
        构建产物路径: 构建产物路径.to_string(),
        验收结论,
        对比上一版: 对比上一版.to_string(),
    }
}

/// 回退版本：清空目标目录，从快照恢复。
pub fn 回退版本(快照目录: &Path, 目标目录: &Path) -> Result<usize, String> {
    if 目标目录.exists() {
        fs::remove_dir_all(目标目录).map_err(|错误| {
            error!(目标 = %目标目录.display(), "清目标失败：{错误}");
            format!("清目标失败: {错误}")
        })?;
    }
    let 排除项 = shihai_fu::扫描排除项(快照目录);
    let mut 计数 = 0;
    复制目录(快照目录, 目标目录, &排除项, &mut 计数)?;
    warn!(目标 = %目标目录.display(), 文件数 = 计数, "版本已回退");
    Ok(计数)
}

/// 落盘一条版本记录到 `.上下文/状态/版本.jsonl`（**追加**，一行一条，原子：旧内容+新行 → 临时文件 → rename）。
/// 防半写：任一行必为完整 JSON（序列化失败则该次写入整体失败，绝不留半截行）。
pub fn 落盘版本记录(状态目录: &Path, 记录: &版本记录) -> Result<(), String> {
    if !状态目录.exists() {
        fs::create_dir_all(状态目录).map_err(|错误| format!("建状态目录失败: {错误}"))?;
    }
    let 文件 = 状态目录.join("版本.jsonl");
    let 行 = serde_json::to_string(记录).map_err(|错误| format!("序列化版本记录失败: {错误}"))?;
    let 旧内容 = fs::read_to_string(&文件).unwrap_or_default();
    let 新内容 = if 旧内容.is_empty() {
        format!("{行}\n")
    } else {
        format!("{旧内容}{行}\n")
    };
    let 临时 = 文件.with_extension("jsonl.tmp");
    {
        let mut 句柄 = fs::File::create(&临时).map_err(|错误| format!("建临时文件失败: {错误}"))?;
        句柄
            .write_all(新内容.as_bytes())
            .map_err(|错误| format!("写临时文件失败: {错误}"))?;
        句柄.flush().map_err(|错误| format!("刷盘失败: {错误}"))?;
        句柄.sync_all().map_err(|错误| format!("fsync 失败: {错误}"))?;
    }
    fs::rename(&临时, &文件).map_err(|错误| format!("rename 失败: {错误}"))?;
    debug!(版本号 = %记录.版本号, "版本记录已落盘");
    Ok(())
}

/// 读全部版本记录（按写入顺序）。
pub fn 读版本历史(状态目录: &Path) -> Result<Vec<版本记录>, String> {
    let 文件 = 状态目录.join("版本.jsonl");
    if !文件.exists() {
        return Ok(Vec::new());
    }
    let 内容 = fs::read_to_string(&文件).map_err(|错误| format!("读版本历史失败: {错误}"))?;
    let mut 项们 = Vec::new();
    for 行 in 内容.lines().filter(|行| !行.trim().is_empty()) {
        let 项 = serde_json::from_str::<版本记录>(行).map_err(|错误| format!("解析版本记录失败: {错误}"))?;
        项们.push(项);
    }
    Ok(项们)
}

/// 读世界状态：`.上下文/状态/世界状态.jsonl`（实际只取最新一行）。
/// 文件不存在返回 None；解析失败返回错误（避免被默认覆盖）。
/// 读时聚合（生产化 2.1）：想法池/在途要求/验收历史 的权威事实源是各自 jsonl
/// （想法.jsonl / 要求.jsonl / 验收.jsonl，由 投递/入池/验收 追加维护），
/// 世界状态内嵌字段经常滞后（实测 验收历史=0 而验收.jsonl 有 55 条）——
/// 读时用 jsonl 全量聚合覆盖内嵌字段，保证状态可视真实一致。
pub fn 读世界状态(状态目录: &Path) -> Result<Option<世界状态>, String> {
    let 文件 = 状态目录.join("世界状态.jsonl");
    if !文件.exists() {
        return Ok(None);
    }
    let 内容 = fs::read_to_string(&文件).map_err(|错误| format!("读世界状态失败: {错误}"))?;
    let 末行 = 内容
        .lines()
        .filter(|行| !行.trim().is_empty())
        .next_back()
        .ok_or_else(|| "世界状态文件无有效行".to_string())?;
    let mut 状态 = serde_json::from_str::<世界状态>(末行).map_err(|错误| format!("解析世界状态失败: {错误}"))?;
    // 读时聚合：想法池 / 在途要求（含全状态） / 验收历史 从各自 jsonl 重读覆盖。
    状态.界主想法池 = crate::落盘队列::<crate::类型_定义_殿::想法>::打开(状态目录.join("想法.jsonl"))
        .读全部()
        .unwrap_or_default();
    状态.在途要求 = crate::落盘队列::<要求书>::打开(状态目录.join("要求.jsonl"))
        .读全部()
        .unwrap_or_default();
    状态.验收历史 = crate::落盘队列::<crate::终裁回执>::打开(状态目录.join("验收.jsonl"))
        .读全部()
        .unwrap_or_default()
        .into_iter()
        .map(|回执| 回执.验收)
        .collect();
    Ok(Some(状态))
}

/// 原子写世界状态：临时文件 → fsync → rename 替换。防半写：任一次写入要么全成功、要么旧文件保持完整。
pub fn 写世界状态(状态目录: &Path, 状态: &世界状态) -> Result<(), String> {
    if !状态目录.exists() {
        fs::create_dir_all(状态目录).map_err(|错误| format!("建状态目录失败: {错误}"))?;
    }
    let 文件 = 状态目录.join("世界状态.jsonl");
    let 行 = serde_json::to_string(状态).map_err(|错误| format!("序列化世界状态失败: {错误}"))?;
    let 临时 = 文件.with_extension("jsonl.tmp");
    {
        let mut 句柄 = fs::File::create(&临时).map_err(|错误| format!("建临时文件失败: {错误}"))?;
        句柄.write_all(行.as_bytes()).map_err(|错误| format!("写临时文件失败: {错误}"))?;
        句柄.write_all(b"\n").map_err(|错误| format!("写换行失败: {错误}"))?;
        句柄.flush().map_err(|错误| format!("刷盘失败: {错误}"))?;
        句柄.sync_all().map_err(|错误| format!("fsync 失败: {错误}"))?;
    }
    fs::rename(&临时, &文件).map_err(|错误| format!("rename 失败: {错误}"))?;
    info!(阶段 = ?状态.阶段, v1已存档 = 状态.v1已存档, "世界状态已原子写入");
    Ok(())
}

/// 首次启动初始化：若世界状态不存在则写入默认（阶段=甲、v1已存档=false）。
/// 已存在则原样返回现有状态，避免覆盖既有进度。
pub fn 确保世界状态初始化(状态目录: &Path) -> Result<世界状态, String> {
    if let Some(已有) = 读世界状态(状态目录)? {
        debug!(阶段 = ?已有.阶段, v1已存档 = 已有.v1已存档, "世界状态已存在，跳过初始化");
        return Ok(已有);
    }
    let 初始 = 世界状态 {
        阶段: 阶段::甲,
        v1已存档: false,
        进入路径: crate::类型_定义_殿::进入路径::从零创建,
        长期记忆: String::new(),
        界主想法池: Vec::new(),
        在途要求: Vec::new(),
        验收历史: Vec::new(),
        失败模式: Vec::new(),
        版本历史: Vec::new(),
        巡世候选池: Vec::new(),
        项目档案: None,
        天道报告库: Vec::new(),
    };
    写世界状态(状态目录, &初始)?;
    info!("世界状态已初始化：阶段=甲，v1已存档=false");
    Ok(初始)
}

/// 标记 v1 已存档：读世界状态 → 改 v1已存档=true → 原子写回；同时把版本记录追加到世界状态内嵌字段。
/// 步骤：先落盘版本记录，再升级世界状态。任一步失败则版本记录保持已写但世界状态未升级，便于人工排查。
pub fn 标记v1已存档(状态目录: &Path, 记录: 版本记录) -> Result<世界状态, String> {
    落盘版本记录(状态目录, &记录)?;
    let mut 状态 = 读世界状态(状态目录)?
        .ok_or_else(|| "世界状态未初始化，请先调用 确保世界状态初始化".to_string())?;
    状态.v1已存档 = true;
    状态.版本历史.push(记录);
    写世界状态(状态目录, &状态)?;
    info!("v1 已存档：阶段切换点已落盘");
    Ok(状态)
}

#[cfg(test)]
mod 测试 {
    use super::*;

    /// 设计稿 §5：版本记录追加不覆盖旧版本，历史按写入顺序保留。
    #[test]
    fn 落盘版本记录_追加不覆盖() {
        let 状态目录 = std::env::temp_dir().join(format!("版本落盘测试-{}", std::process::id()));
        let r1 = 生成版本记录("v1", 1, 阶段::甲, "v1 内容", "", "", vec![], "基线");
        let r2 = 生成版本记录("v2", 2, 阶段::乙, "v2 内容", "", "", vec![], "增量");
        assert!(落盘版本记录(&状态目录, &r1).is_ok());
        assert!(落盘版本记录(&状态目录, &r2).is_ok());
        let 历史 = 读版本历史(&状态目录).expect("应可读历史");
        assert_eq!(历史.len(), 2, "两次落盘应保留两条记录");
        assert_eq!(历史[0].版本号, "v1");
        assert_eq!(历史[1].版本号, "v2");
        let _ = std::fs::remove_dir_all(&状态目录);
    }
}

/// 版本库根目录：`.上下文/版本库/`（用于源码快照）。
pub fn 版本库目录(工作区根: &Path) -> PathBuf {
    工作区根.join(".上下文").join("版本库")
}