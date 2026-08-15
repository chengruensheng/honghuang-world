// 分块解析 —— 日志配置分块解析：级别 / 去向
#![allow(non_snake_case)]

use std::path::PathBuf;

/// 日志级别（从轻到重）
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum 日志级别 {
    追踪,
    调试,
    信息,
    警告,
    错误,
}

/// 日志去向
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum 日志去向 {
    仅控制台,
    仅文件(PathBuf),
    双写(PathBuf),
}

/// 日志配置
#[derive(Clone, Debug)]
pub struct 日志配置 {
    pub 级别: 日志级别,
    pub 去向: 日志去向,
}

impl Default for 日志配置 {
    fn default() -> Self {
        日志配置 {
            级别: 日志级别::信息,
            去向: 日志去向::仅控制台,
        }
    }
}

/// 按级别名解析（追踪/调试/信息/警告/错误），不识别时回退信息级
pub fn 解析级别(名: &str) -> 日志级别 {
    match 名 {
        "追踪" | "trace" => 日志级别::追踪,
        "调试" | "debug" => 日志级别::调试,
        "信息" | "info" => 日志级别::信息,
        "警告" | "warn" => 日志级别::警告,
        "错误" | "error" => 日志级别::错误,
        _ => 日志级别::信息,
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    #[test]
    fn 默认配置为信息级控制台() {
        let 配置 = 日志配置::default();
        assert_eq!(配置.级别, 日志级别::信息);
        assert_eq!(配置.去向, 日志去向::仅控制台);
    }

    #[test]
    fn 解析级别回退() {
        assert_eq!(解析级别("错误"), 日志级别::错误);
        assert_eq!(解析级别("debug"), 日志级别::调试);
        assert_eq!(解析级别("未知"), 日志级别::信息);
    }
}
