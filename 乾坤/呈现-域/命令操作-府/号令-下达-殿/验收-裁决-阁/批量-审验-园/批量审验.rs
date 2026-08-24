//! 验收裁决：构造验收回执 → 入验收历史

use crate::状态目录;
use rizhi_fu::{error, info, warn};

pub fn 裁决验收(要求id: &str, 结论: &str, 意见: &str) -> String {
    let 结论值 = match 结论 {
        "通过" => tianting_fu::验收结论::通过,
        "打回" => tianting_fu::验收结论::打回,
        _ => {
            warn!(要求id, 结论, "验收结论非法");
            return format!("结论需为 通过|打回，当前：{结论}");
        }
    };
    let 回执 = tianting_fu::验收回执 {
        要求id: 要求id.to_string(),
        结论: 结论值,
        验收意见: Some(意见.to_string()),
        产物: Vec::new(),
        耗时秒: 0.0,
    };
    let 队列 = tianting_fu::落盘队列::<tianting_fu::验收回执>::打开(
        状态目录().join("验收.jsonl"),
    );
    match 队列.入队(&回执) {
        Ok(_) => {
            info!(要求id, 结论, "验收已裁决入历史");
            format!("验收已裁决\n要求id：{要求id}\n结论：{结论}\n意见：{意见}\n已入验收历史")
        }
        Err(错误) => {
            error!(要求id, "验收入队失败：{错误}");
            format!("入队失败：{错误}")
        }
    }
}

#[cfg(test)]
mod 测试_裁决验收 {
    //! 验收裁决函数单测。
    //!
    //! 约束：`裁决验收` 写死路径 `状态目录/验收.jsonl`，跨 crate 并行测试时
    //! 其他测试可能同时追加写入，导致读文件遇到 UTF-8 边界错误。
    //! 因此本套用例**仅**断言函数返回值文本，不回读文件、不验证落盘可读回。

    use super::*;
    use crate::测试设施::工作区测试锁;

    #[test]
    fn 合法通过_返回内容含裁决字段() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 结果 = 裁决验收("R-001", "通过", "无意见");
        assert!(结果.contains("R-001"), "应含要求id：{结果}");
        assert!(结果.contains("通过"), "应含结论：通过：{结果}");
        assert!(结果.contains("无意见"), "应含验收意见：{结果}");
        assert!(结果.contains("已入验收历史"), "应提示已入验收历史：{结果}");
    }

    #[test]
    fn 合法打回_返回内容含裁决字段() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 结果 = 裁决验收("R-002", "打回", "需修正");
        assert!(结果.contains("R-002"), "应含要求id：{结果}");
        assert!(结果.contains("打回"), "应含结论：打回：{结果}");
        assert!(结果.contains("需修正"), "应含验收意见：{结果}");
        assert!(结果.contains("已入验收历史"), "应提示已入验收历史：{结果}");
    }

    #[test]
    fn 非法结论_返回错误且不入队() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 结果 = 裁决验收("R-003", "待定", "非法");
        assert!(结果.contains("结论需为"), "应指明结论非法：{结果}");
        assert!(结果.contains("待定"), "应回显非法结论值：{结果}");
        assert!(
            !结果.contains("已入验收历史"),
            "非法结论不应入验收历史：{结果}"
        );
    }

    #[test]
    fn 空字符串结论_视为非法() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 结果 = 裁决验收("R-004", "", "空结论");
        assert!(结果.contains("结论需为"), "空结论应视为非法：{结果}");
        assert!(结果.contains("当前："), "应回显非法结论值：{结果}");
        assert!(!结果.contains("已入验收历史"), "空结论不应入队");
    }

    #[test]
    fn 空意见_仍入队() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 结果 = 裁决验收("R-005", "通过", "");
        assert!(结果.contains("已入验收历史"), "空意见应允许入队：{结果}");
        assert!(结果.contains("R-005"));
    }

    #[test]
    fn 超长意见_仍入队() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 意见 = "a".repeat(8192);
        let 结果 = 裁决验收("R-006", "通过", &意见);
        assert!(结果.contains("已入验收历史"), "超长意见应允许入队：{结果}");
    }

    #[test]
    fn 特殊字符意见_仍入队() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 意见 = "引号\"反斜杠\\换行\n制表符\t中文标点，。，！？";
        let 结果 = 裁决验收("R-007", "通过", 意见);
        assert!(
            结果.contains("已入验收历史"),
            "特殊字符意见应允许入队：{结果}"
        );
    }

    #[test]
    fn utf8要求id_仍入队() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let id = "要求-测试-αβγ-中文";
        let 结果 = 裁决验收(id, "通过", "意见");
        assert!(结果.contains(id), "应回显UTF-8要求id：{结果}");
        assert!(结果.contains("已入验收历史"));
    }

    #[test]
    fn 同id重复裁决_两次均返回入队() {
        let _锁 = 工作区测试锁.lock().unwrap_or_else(|e| e.into_inner());
        let 结果1 = 裁决验收("R-008", "通过", "第一次");
        let 结果2 = 裁决验收("R-008", "打回", "第二次");
        assert!(结果1.contains("已入验收历史"), "第一次应入队：{结果1}");
        assert!(
            结果2.contains("已入验收历史"),
            "第二次同id亦应入队：{结果2}"
        );
    }
}
