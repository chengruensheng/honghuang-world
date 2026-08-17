//! 流式-回放-园 · 流式回放测试：
//! 列表关键词 / 会话过滤 / 流程状态汇总 / 三池计数。

#[cfg(test)]
mod 测试 {
    use std::fs;
    use std::sync::Mutex;
    use super::super::流式回放::{流水列表, 流水跟踪, 流水跟随, 全流程总览};

    /// 串行互斥：四个用例都会改 WORLD_WORKSPACE_ROOT，必须串行跑。
    static 串行: Mutex<()> = Mutex::new(());

    fn 准备根() -> std::path::PathBuf {
        let 唯一 = format!(
            "流水测试-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let 临时根 = std::env::temp_dir().join(唯一);
        let 状态 = 临时根.join(".上下文").join("状态");
        let 格位 = 临时根.join(".上下文").join("格位");
        fs::create_dir_all(&状态).expect("建状态目录");
        fs::create_dir_all(&格位).expect("建格位目录");
        std::env::set_var("WORLD_WORKSPACE_ROOT", &临时根);
        临时根
    }

    fn 清理(根: &std::path::Path) {
        std::env::remove_var("WORLD_WORKSPACE_ROOT");
        let _ = fs::remove_dir_all(根);
    }

    /// 用例 1 · 列表关键词：返回文本须含「流水列表」标题与事件条数标注。
    #[test]
    fn 流水列表_含列表关键词与条数() {
        let _锁 = 串行.lock().unwrap();
        let 根 = 准备根();

        let 输出 = 流水列表();
        assert!(输出.contains("流水列表"), "列表标题缺失：{输出}");
        assert!(
            输出.contains("条事件") || 输出.contains("（空）"),
            "列表应有条数或空态标记：{输出}"
        );

        清理(&根);
    }

    /// 用例 2 · 会话过滤：传入会话 id 应被原样回填到「会话 {会话id}」中。
    #[test]
    fn 流水跟踪_会话id进入标题() {
        let _锁 = 串行.lock().unwrap();
        let 根 = 准备根();

        let 输出 = 流水跟踪("测试会话-A", false);
        assert!(输出.contains("测试会话-A"), "会话 id 应回填到标题：{输出}");
        assert!(输出.contains("执行过程"), "应含「执行过程」流程标识：{输出}");
        assert!(输出.contains("条事件") || 输出.contains("（空）"), "应含事件条数：{输出}");

        清理(&根);
    }

    /// 用例 3 · 流程状态汇总：全流程总览应在标题中给出三池计数行。
    #[test]
    fn 全流程总览_三池计数齐备() {
        let _锁 = 串行.lock().unwrap();
        let 根 = 准备根();

        let 输出 = 全流程总览();
        assert!(输出.contains("全流程总览"), "总览标题缺失：{输出}");
        assert!(输出.contains("想法池["), "想法池计数缺失：{输出}");
        assert!(输出.contains("要求队列["), "要求队列计数缺失：{输出}");
        assert!(输出.contains("验收历史["), "验收历史计数缺失：{输出}");

        清理(&根);
    }

    /// 用例 4 · 流水跟随：空会话应走「全流程总览」分支。
    #[test]
    fn 流水跟随_空会话走总览分支() {
        let _锁 = 串行.lock().unwrap();
        let 根 = 准备根();

        let 输出 = 流水跟随("");
        assert!(输出.contains("全流程总览"), "空会话应触发总览分支：{输出}");
        assert!(输出.contains("想法池["), "总览分支应含想法池计数：{输出}");

        清理(&根);
    }
}