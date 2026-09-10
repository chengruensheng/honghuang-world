use std::io::Write;
use std::path::{Component as 路径组件, Path, PathBuf};
use hm_error::{Error, Result};

/// 回收站目录名：安全删除的兜底落点（位于工作区内，同盘原子移动，可人工恢复）
pub const 回收站名: &str = ".回收站";
/// 删除名册文件名：回收站内 JSONL 追加式，一行 = 一次删除的完整审计记录
const 删除名册: &str = "删除名册.jsonl";

/// 安全删除四步闭环（执行器「删除文件」的唯一实现路径）：
/// 1) 预审——沙箱校验（拒绝绝对路径与 `..`）+ 存在性检查 + 保护回收站与名册自身；
/// 2) 名册——删除完成后向 `.回收站/删除名册.jsonl` 追加一行完整审计记录（时间戳/路径/落点/复核结果）；
/// 3) 安全删除——rename 到 `工作区/.回收站/<原相对路径>`（同盘原子移动可恢复，落点冲突加毫秒时间戳）；
/// 4) 复核——确认原路径已不存在且回收站落点存在，否则回滚原位并报错。
///
/// 结果文本会向任务智能体说明落点与可恢复性，防止模型把「安全删除」误读为「已彻底销毁」。
pub fn 安全删除(工作区: &Path, 路径: &str) -> Result<String> {
    // ── 第一步：预审 ──
    let 相对 = Path::new(路径);
    if 相对.is_absolute() || 相对.components().any(|c| matches!(c, 路径组件::ParentDir)) {
        return Err(Error::Config(format!("删除预审失败（路径越出工作区）: {路径}")));
    }
    if 是回收站自身或名册(相对) {
        return Err(Error::Config(format!(
            "删除预审失败：{回收站名} 与删除名册受保护，禁止删除: {路径}"
        )));
    }
    let 目标 = 工作区.join(相对);
    if !目标.is_file() {
        return Err(Error::Config(format!(
            "删除预审失败：{路径} 不存在或不是文件（先用「按名找文件」确认真实相对路径再删）"
        )));
    }
    // ── 第三步：移入回收站（同盘 rename，原子可恢复） ──
    let 回收站 = 工作区.join(回收站名);
    let 落点 = 避让同名(&回收站.join(相对));
    if let Some(父) = 落点.parent() {
        std::fs::create_dir_all(父).map_err(Error::Io)?;
    }
    std::fs::rename(&目标, &落点).map_err(Error::Io)?;
    // ── 第四步：复核真实删除，失败则尽力回滚 ──
    if 目标.exists() || !落点.exists() {
        let _ = std::fs::rename(&落点, &目标);
        return Err(Error::Config(format!(
            "删除复核失败：{路径} 移入回收站后未确认真实移除，已回滚原位，请重试或人工检查"
        )));
    }
    // ── 第二步：名册（成功才入册；失败场景未发生删除，不入册） ──
    let 落点显示 = 相对显示(&落点, 工作区);
    记入名册(&回收站, 路径, &落点显示)?;
    Ok(format!("已安全删除: {路径} → {落点显示}（已移入回收站可恢复，名册已记录）"))
}

/// 相对路径（或路径首组件）是否位于回收站内。
///
/// 两种用途：① 删除预审——回收站自身与名册禁止删除（防止清理逻辑自我销毁兜底能力）；
/// ② 感知扫描——回收站内是「已删除文件」，不属于工作区可见产物，按名找文件/搜索内容/
/// 清理核验必须排除，否则已删除文件被当残留 → 核验门永远驳回 → 清理死循环。
pub fn 是回收站条目(相对: &Path) -> bool {
    相对
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .map(|s| s.eq_ignore_ascii_case(回收站名))
        .unwrap_or(false)
}

/// 路径是否指向回收站内条目或删除名册（删除预审用，一律拒绝）
fn 是回收站自身或名册(相对: &Path) -> bool {
    是回收站条目(相对) || 相对 == Path::new(删除名册)
}

/// 回收站内同名冲突避让：落点已存在时在文件名后追加毫秒时间戳，两份都保留
fn 避让同名(落点: &Path) -> PathBuf {
    if !落点.exists() {
        return 落点.to_path_buf();
    }
    let 毫秒 = 当前毫秒();
    // 落点已存在故必有文件名；个别畸形路径（如以 .. 结尾）取不到时按未名处理
    let 原名 = match 落点.file_name().and_then(|n| n.to_str()) {
        Some(名) => 名.to_string(),
        None => "未名".to_string(),
    };
    落点.with_file_name(format!("{原名}.{毫秒}"))
}

/// 路径的工作区相对显示形式（统一正斜杠，供结果文本与名册记录）
fn 相对显示(路径: &Path, 工作区: &Path) -> String {
    // 剥前缀失败（如路径不在工作区内）时原样显示，属预期回退而非错误
    match 路径.strip_prefix(工作区) {
        Ok(相对) => 相对.to_string_lossy().replace('\\', "/"),
        Err(_) => 路径.to_string_lossy().replace('\\', "/"),
    }
}

/// 向删除名册追加一行 JSONL 审计记录：时间戳 + 路径 + 落点 + 四步结论
fn 记入名册(回收站: &Path, 路径: &str, 落点显示: &str) -> Result<()> {
    let 行 = serde_json::json!({
        "时间戳毫秒": 当前毫秒(),
        "路径": 路径,
        "落点": 落点显示,
        "预审": "通过",
        "移入回收站": "成功",
        "复核": "原路径已不存在且落点存在",
        "结果": "已删除可恢复",
    })
    .to_string();
    let 名册 = 回收站.join(删除名册);
    let mut 文件 = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(名册)
        .map_err(Error::Io)?;
    文件.write_all(format!("{行}\n").as_bytes()).map_err(Error::Io)
}

/// 当前 UNIX 时间毫秒（名册时间戳与同名避让用）
fn 当前毫秒() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 临时工作区(名: &str) -> PathBuf {
        let 根 = std::env::temp_dir().join(format!("zd_delete_test_{名}"));
        let _ = std::fs::remove_dir_all(&根);
        std::fs::create_dir_all(&根).expect("创建工作区");
        根
    }

    #[test]
    fn 安全删除_四步闭环全通过() {
        let 根 = 临时工作区("闭环");
        std::fs::write(根.join("临时.txt"), "内容").expect("写文件");
        let 结果 = 安全删除(&根, "临时.txt").expect("删除应成功");
        assert!(结果.contains(".回收站"), "结果应说明回收站落点: {结果}");
        assert!(!根.join("临时.txt").exists(), "原路径应已真实移除（复核）");
        assert!(根.join(".回收站").join("临时.txt").exists(), "回收站应有落点可恢复");
        let 名册 = std::fs::read_to_string(根.join(".回收站").join("删除名册.jsonl")).expect("名册应存在");
        assert!(名册.contains("临时.txt") && 名册.contains("已删除可恢复"), "名册应记录本次删除: {名册}");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 安全删除_预审拒绝不存在与越界() {
        let 根 = 临时工作区("预审");
        assert!(安全删除(&根, "不存在.bak").is_err(), "不存在应预审失败");
        assert!(安全删除(&根, "../越界.txt").is_err(), "越界应被拒绝");
        assert!(安全删除(&根, "C:/外部.txt").is_err(), "绝对路径应被拒绝");
        assert!(!根.join(".回收站").exists(), "预审失败不得创建回收站");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 安全删除_回收站与名册自身受保护() {
        let 根 = 临时工作区("保护");
        std::fs::create_dir_all(根.join(".回收站")).expect("建回收站");
        std::fs::write(根.join(".回收站").join("删除名册.jsonl"), "{}").expect("写名册");
        assert!(安全删除(&根, ".回收站/删除名册.jsonl").is_err(), "名册禁止删除");
        assert!(安全删除(&根, ".回收站/内含文件.bak").is_err(), "回收站内文件禁止删除");
        assert!(根.join(".回收站").join("删除名册.jsonl").exists(), "名册应原样保留");
        let _ = std::fs::remove_dir_all(&根);
    }

    #[test]
    fn 安全删除_同名落点避让不覆盖() {
        let 根 = 临时工作区("避让");
        std::fs::write(根.join("旧.bak"), "第一份").expect("写第一份");
        assert!(安全删除(&根, "旧.bak").is_ok());
        std::fs::write(根.join("旧.bak"), "第二份").expect("再写同名");
        assert!(安全删除(&根, "旧.bak").is_ok());
        let 回收站 = 根.join(".回收站");
        assert_eq!(
            std::fs::read_dir(&回收站).expect("列回收站").count(),
            3, // 两份落点 + 名册
            "同名落点应避让保留两份，实际条目: {:?}",
            std::fs::read_dir(&回收站).unwrap().collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&根);
    }
}
