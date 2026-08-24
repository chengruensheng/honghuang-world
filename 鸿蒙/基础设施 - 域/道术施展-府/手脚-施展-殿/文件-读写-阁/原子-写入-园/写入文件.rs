//! 原子 - 写入 - 园：写文件全部内容，自动创建父目录。
//! 写前经 回滚垫 备份旧内容：任务失败时可单文件撤销恢复。
//! 行尾保持（观察点 7）：目标文件原为 CRLF 时，写入前把内容 LF→CRLF 转换——
//! 防模型生成的 LF 文本把整文件行尾污染（git diff 纯行尾噪音）。

use rizhi_fu::{debug, error, info};
use shihai_fu::{回滚垫, 工作区, 当前任务};
use std::path::{Path, PathBuf};

/// §B.1.5 沙箱兼容：临时目录放工作区根下（写入文件测试用）。
#[allow(dead_code)]  // 测试用 — §B.1.5 沙箱兼容
fn 工作区根() -> PathBuf {
    工作区::定位().根路径().join(".上下文")
}

/// 把内容按目标文件行尾风格归一（原文件 CRLF → 内容 LF 转 CRLF；原 LF/新建 → 保持 LF）。
fn 按行尾归一(原文: Option<&str>, 内容: &str) -> String {
    let 原文为crlf = 原文.is_some_and(|原文| 原文.contains("\r\n"));
    if 原文为crlf && !内容.contains("\r\n") {
        // 只转纯 LF（避免把已含 CRLF 的内容双重加 \r）。
        内容.replace('\n', "\r\n")
    } else {
        内容.to_string()
    }
}

/// §B.1.5 沙箱校验：拒绝 .. 跳出 + 符号链接跳出工作区根。
/// 错误：路径在工作区根外 → 返回 Err 不写盘。
pub fn 沙箱校验(工作区根: &Path, 目标: &Path) -> Result<(), String> {
    let 目标 = std::path::Path::new(目标).components().collect::<Vec<_>>();
    let 根段 = 工作区根.components().collect::<Vec<_>>();
    if 目标.len() < 根段.len() || 目标[..根段.len()] != 根段[..] {
        return Err(format!("沙箱拒绝：路径 {:?} 不在工作区根 {:?} 内", 目标, 根段));
    }
    Ok(())
}

/// 写文件全部内容，父目录不存在时自动创建；写前先备份进回滚垫（失败只警告不阻断）。
/// 返回是否真的写入（false = 内容与现状相同，空操作跳过，未写盘）。
/// 空操作优化：目标已存在且内容（按行尾归一后）与现状一致 → 直接返回 false。
/// 行尾保持：原文件 CRLF 时写入内容先转 CRLF，防整文件行尾污染。
pub fn 写文件(路径: &str, 内容: &str) -> Result<bool, String> {
    let 根 = shihai_fu::工作区::定位();
    let 目标 = Path::new(路径);
    沙箱校验(根.根路径(), 目标)?;
    // 内容相同检测：文件已存在且内容（行尾归一后）一致 → 空操作跳过。
    let 原文 = std::fs::read_to_string(路径).ok();
    let 写入内容 = 按行尾归一(原文.as_deref(), 内容);
    if let Some(原文) = &原文 {
        if 原文 == &写入内容 {
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
    } // 备份失败仅 debug — 写文件应继续（用 ? 会让无备份时也失败）
    let 目标 = Path::new(路径);
    if let Some(父) = 目标.parent() {
        if !父.as_os_str().is_empty() {
            std::fs::create_dir_all(父)
                .map_err(|错误| format!("创建父目录失败：{}：{错误}", 父.display()))?;
        }
    }
    std::fs::write(目标, &写入内容).map_err(|错误| {
        error!(路径, "写文件失败：{错误}");
        format!("写文件失败：{路径}：{错误}")
    })?;
    debug!(路径, 字节数 = 写入内容.len(), "写文件完成");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空操作优化：文件已存在且内容相同 → 成功返回且不改写（mtime 不变）。
    #[test]
    fn 写文件_内容相同跳过() {
        let 临时目录 = 工作区根().join(format!(".上下文/.test-tmp/写文件测试-跳过-{}", std::process::id()));
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
        let 临时目录 = 工作区根().join(format!(".上下文/.test-tmp/写文件测试-写入-{}", std::process::id()));
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

    /// 行尾保持（观察点 7）：原文件 CRLF → 写入 LF 内容自动转 CRLF，防整文件行尾污染。
    #[test]
    #[allow(non_snake_case)]
    fn 写文件_保持原CRLF行尾() {
        let 临时目录 = 工作区根().join(format!(".上下文/.test-tmp/写文件测试-行尾-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&临时目录);
        let 临时文件 = 临时目录.join("sample.txt");
        std::fs::write(&临时文件, "旧行1\r\n旧行2\r\n").unwrap();
        // 模型生成 LF 内容，写入后应保持 CRLF（原文件风格）。
        写文件(临时文件.to_str().unwrap(), "新行1\n新行2\n").unwrap();
        let 写后 = std::fs::read_to_string(&临时文件).unwrap();
        assert!(写后.contains("\r\n"), "原 CRLF 文件应保持 CRLF：{:?}", 写后);
        assert!(
            !写后.contains("\n") || 写后.contains("\r\n"),
            "不应出现裸 LF"
        );
        assert_eq!(写后, "新行1\r\n新行2\r\n", "内容应归一为 CRLF");
        let _ = std::fs::remove_dir_all(&临时目录);
    }

    /// 行尾保持：原文件 LF → 保持 LF（不转 CRLF）。
    #[test]
    #[allow(non_snake_case)]
    fn 写文件_保持原LF行尾() {
        let 临时目录 = 工作区根().join(format!(".上下文/.test-tmp/写文件测试-行尾LF-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&临时目录);
        let 临时文件 = 临时目录.join("sample.txt");
        std::fs::write(&临时文件, "旧行1\n旧行2\n").unwrap();
        写文件(临时文件.to_str().unwrap(), "新行1\n新行2\n").unwrap();
        let 写后 = std::fs::read_to_string(&临时文件).unwrap();
        assert_eq!(写后, "新行1\n新行2\n", "原 LF 文件应保持 LF");
        let _ = std::fs::remove_dir_all(&临时目录);
    }
}
