//! 进程 - 驱动 - 园：驱动外部进程执行命令。

use std::process::Command;

/// 命令执行结果。
#[derive(Clone, Debug, PartialEq)]
pub struct 命令结果 {
    pub 退出码: Option<i32>,
    pub 标准输出: String,
    pub 标准错误: String,
}

/// 运行外部命令，可选工作目录，捕获标准输出与错误。
pub fn 运行命令(命令: &str, 参数们: &[&str], 工作目录: Option<&str>) -> Result<命令结果, String> {
    let mut 进程 = Command::new(命令);
    进程.args(参数们);
    if let Some(目录) = 工作目录 {
        进程.current_dir(目录);
    }
    let 输出 = 进程
        .output()
        .map_err(|错误| format!("运行命令失败：{命令}：{错误}"))?;
    Ok(命令结果 {
        退出码: 输出.status.code(),
        标准输出: String::from_utf8_lossy(&输出.stdout).to_string(),
        标准错误: String::from_utf8_lossy(&输出.stderr).to_string(),
    })
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 运行命令取退出码与输出() {
        let 结果 = 运行命令("cargo", &["--version"], None).unwrap();
        assert_eq!(结果.退出码, Some(0));
        assert!(!结果.标准输出.is_empty());
    }

    #[test]
    fn 运行不存在的命令报错() {
        assert!(运行命令("不存在的命令_xyz", &[], None).is_err());
    }
}
