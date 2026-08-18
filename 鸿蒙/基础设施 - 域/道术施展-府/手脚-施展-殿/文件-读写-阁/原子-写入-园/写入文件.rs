//! 原子 - 写入 - 园：写文件全部内容，自动创建父目录。
//! 写前经 回滚垫 备份旧内容：任务失败时可单文件撤销恢复。

use rizhi_fu::{debug, error, info};
use shihai_fu::{回滚垫, 工作区, 当前任务};
use std::path::Path;

/// 写文件全部内容，父目录不存在时自动创建；写前先备份进回滚垫（失败只警告不阻断）。
/// 返回是否真的写入（false = 内容与现状相同，空操作跳过，未写盘）。
/// 空操作优化：目标已存在且内容与写入内容完全相同 → 直接返回 false（不备份不重写，
/// 省轮次，且不产生回滚垫备份/产物记录噪音——执行层常"确认现状"式重写同内容）。
pub fn 写文件(路径: &str, 内容: &str) -> Result<bool, String> {
    // 内容相同检测：文件已存在且内容一致 → 空操作跳过。
    if let Ok(原文) = std::fs::read_to_string(路径) {
        if 原文 == 内容 {
            info!(
                路径,
                字节数 = 内容.len(),
                "写文件跳过：内容与现状相同（空操作）"
            );
            return Ok(false);
        }
    }
    let 垫 = 回滚垫::在工作区(&工作区::定位());
    if let Err(错误) = 垫.备份(&当前任务(), 路径) {
        debug!(路径, "回滚垫备份跳过：{错误}");
    }
    let 目标 = Path::new(路径);
    if let Some(父) = 目标.parent() {
        if !父.as_os_str().is_empty() {
            std::fs::create_dir_all(父)
                .map_err(|错误| format!("创建父目录失败：{}：{错误}", 父.display()))?;
        }
    }
    std::fs::write(目标, 内容).map_err(|错误| {
        error!(路径, "写文件失败：{错误}");
        format!("写文件失败：{路径}：{错误}")
    })?;
    debug!(路径, 字节数 = 内容.len(), "写文件完成");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空操作优化：文件已存在且内容相同 → 成功返回且不改写（mtime 不变）。
    #[test]
    fn 写文件_内容相同跳过() {
        let 临时目录 = std::env::temp_dir().join(format!("写文件测试-跳过-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&临时目录);
        let 临时文件 = 临时目录.join("sample.txt");
        std::fs::write(&临时文件, "相同内容").unwrap();
        let 改前 = std::fs::metadata(&临时文件).unwrap().modified().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(20));
        写文件(临时文件.to_str().unwrap(), "相同内容").unwrap();

        let 改后 = std::fs::metadata(&临时文件).unwrap().modified().unwrap();
        assert_eq!(改前, 改后, "内容相同应跳过写入（mtime 不变）");
        assert_eq!(
            std::fs::read_to_string(&临时文件).unwrap(),
            "相同内容",
            "内容不应变化"
        );
        let _ = std::fs::remove_dir_all(&临时目录);
    }

    /// 内容不同 → 正常写入。
    #[test]
    fn 写文件_内容不同正常写入() {
        let 临时目录 = std::env::temp_dir().join(format!("写文件测试-写入-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&临时目录);
        let 临时文件 = 临时目录.join("sample.txt");
        std::fs::write(&临时文件, "旧内容").unwrap();
        写文件(临时文件.to_str().unwrap(), "新内容").unwrap();
        assert_eq!(
            std::fs::read_to_string(&临时文件).unwrap(),
            "新内容",
            "内容不同应写入"
        );
        let _ = std::fs::remove_dir_all(&临时目录);
    }
}
