//! 增量 - 改写 - 园：把文件里第一次出现的旧文替换为新文。
//! 写前经 回滚垫 备份旧内容：任务失败时可单文件撤销恢复。

use rizhi_fu::{debug, error, info, warn};
use shihai_fu::{回滚垫, 工作区, 当前任务};
use crate::原子_写入_园::沙箱校验;
use std::path::PathBuf;

fn 工作区根() -> PathBuf {
    工作区::定位().根路径().join(".上下文")
}

/// 把文件里第一次出现的旧文替换为新文，旧文不存在则报错；写前先备份进回滚垫。
/// 返回是否真的改写（false = 替换结果与原文相同，空操作跳过，未写盘）。
/// 空操作优化：替换后内容与原文相同（旧文==新文 等）→ 返回 false，不备份不重写。
pub fn 改文件(路径: &str, 旧文: &str, 新文: &str) -> Result<bool, String> {
    let 根 = shihai_fu::工作区::定位();
    if let Err(错误) = 沙箱校验(根.根路径(), std::path::Path::new(路径)) {
        return Err(错误);
    }
    let 原文 = std::fs::read_to_string(路径).map_err(|错误| {
        error!(路径, "改文件读失败：{错误}");
        format!("读文件失败：{路径}：{错误}")
    })?;
    if !原文.contains(旧文) {
        // 错误容错（2026-08-18）：附「文件总长度 + 旧文长度 + 旧文前 80 字符 + 文件前 200 字符」，
        // 让 LLM 能看到真实内容（避免「未找到待替换内容」撞同错空转烧 token）。
        // 实测：让世界产出复杂任务（要求-36/31）时改文件撞「未找到」连撞 2-3 轮。
        warn!(路径, "改文件失败：未找到待替换内容（附原文片段便于诊断）");
        let 旧文预览: String = 旧文.chars().take(80).collect();
        let 原文预览: String = 原文.chars().take(200).collect();
        return Err(format!(
            "改文件失败：{路径}：未找到待替换内容（文件长度 {} 字节，旧文长度 {} 字节，旧文前 80 字符 = {:?}，文件前 200 字符 = {:?}）",
            原文.len(), 旧文.len(), 旧文预览, 原文预览
        ));
    }
    let 改后 = if 原文.contains("\r\n") && !新文.contains("\r\n") {
        // 行尾保持（观察点 7）：原文 CRLF 时，替换片段的新文 LF→CRLF 归一，防混合行尾。
        原文.replacen(旧文, &新文.replace('\n', "\r\n"), 1)
    } else {
        原文.replacen(旧文, 新文, 1)
    };
    // 空操作检测：替换结果与原文相同（旧文==新文）→ 跳过，不备份不重写。
    if 改后 == 原文 {
        info!(路径, "改文件跳过：替换结果与现状相同（空操作）");
        return Ok(false);
    }
    let 垫 = 回滚垫::在工作区(&工作区::定位());
    if let Err(错误) = 垫.备份(&当前任务(), 路径) {
        debug!(路径, "回滚垫备份跳过：{错误}");
    }
    std::fs::write(路径, 改后).map_err(|错误| {
        error!(路径, "改文件写失败：{错误}");
        format!("写文件失败：{路径}：{错误}")
    })?;
    debug!(路径, "文件已改写");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 「未找到待替换内容」必须附文件长度 + 旧文长度 + 旧文预览 + 文件前 200 字符，
    /// 让 LLM 看到真实内容（2026-08-18 容错补齐）。
    #[test]
    fn 改文件_未找到时_附原文片段便于诊断() {
        let 临时目录 = 工作区根().join(format!(".上下文/.test-tmp/改文件测试-未找到-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&临时目录);
        let 临时文件 = 临时目录.join("sample.txt");
        std::fs::write(&临时文件, "实际内容是 ABCDEF, 不含要找的子串").unwrap();

        // 旧文 = 「目标字符串」不在文件里
        let 结果 = 改文件(临时文件.to_str().unwrap(), "目标字符串", "新内容");
        assert!(结果.is_err());
        let 错误 = 结果.unwrap_err();
        assert!(
            错误.contains("未找到待替换内容"),
            "错误信息应含'未找到'：{}",
            错误
        );
        assert!(
            错误.contains("文件长度"),
            "错误信息应含文件长度便于 LLM 诊断：{}",
            错误
        );
        assert!(错误.contains("旧文长度"), "错误信息应含旧文长度：{}", 错误);
        assert!(
            错误.contains("旧文前 80 字符"),
            "错误信息应含旧文前 80 字符：{}",
            错误
        );
        assert!(
            错误.contains("文件前 200 字符"),
            "错误信息应含文件前 200 字符：{}",
            错误
        );
        let _ = std::fs::remove_file(&临时文件);
    }

    /// 「找到并替换」应正常工作。
    #[test]
    fn 改文件_找到时正常替换() {
        let 临时目录 = 工作区根().join(format!(".上下文/.test-tmp/改文件测试-找到-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&临时目录);
        let 临时文件 = 临时目录.join("sample.txt");
        std::fs::write(&临时文件, "old content here, more").unwrap();
        改文件(临时文件.to_str().unwrap(), "old content", "new content").unwrap();
        let 新内容 = std::fs::read_to_string(&临时文件).unwrap();
        assert_eq!(新内容, "new content here, more");
        let _ = std::fs::remove_file(&临时文件);
    }

    /// 空操作优化：旧文==新文 → 替换结果与原文相同，跳过改写（mtime 不变）。
    #[test]
    fn 改文件_替换结果相同跳过() {
        let 临时目录 = 工作区根().join(format!(".上下文/.test-tmp/改文件测试-跳过-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&临时目录);
        let 临时文件 = 临时目录.join("sample.txt");
        std::fs::write(&临时文件, "内容包含目标词").unwrap();
        let 改前 = std::fs::metadata(&临时文件).unwrap().modified().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(20));
        改文件(临时文件.to_str().unwrap(), "目标词", "目标词").unwrap();

        let 改后 = std::fs::metadata(&临时文件).unwrap().modified().unwrap();
        assert_eq!(改前, 改后, "替换结果相同应跳过改写（mtime 不变）");
        assert_eq!(
            std::fs::read_to_string(&临时文件).unwrap(),
            "内容包含目标词",
            "内容不应变化"
        );
        let _ = std::fs::remove_file(&临时文件);
    }

    /// 行尾保持（观察点 7）：原文 CRLF、新文 LF → 替换片段转 CRLF，防混合行尾。
    #[test]
    #[allow(non_snake_case)]
    fn 改文件_新文LF转CRLF防混合行尾() {
        let 临时目录 = 工作区根().join(format!(".上下文/.test-tmp/改文件测试-行尾-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&临时目录);
        let 临时文件 = 临时目录.join("sample.txt");
        std::fs::write(&临时文件, "第1行\r\n目标词\r\n第3行\r\n").unwrap();
        改文件(临时文件.to_str().unwrap(), "目标词", "新词甲\n新词乙\n").unwrap();
        let 写后 = std::fs::read_to_string(&临时文件).unwrap();
        assert!(
            写后.contains("新词甲\r\n新词乙\r\n"),
            "新文应转 CRLF：{:?}",
            写后
        );
        assert!(!写后.contains("新词甲\n"), "不应出现裸 LF");
        let _ = std::fs::remove_file(&临时文件);
    }
}
