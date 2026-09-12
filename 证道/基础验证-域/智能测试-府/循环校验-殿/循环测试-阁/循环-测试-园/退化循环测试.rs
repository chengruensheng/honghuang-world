#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use hm_agent::智能体;
    use hm_contract::Component;
    use hm_content_contract::{工具对话器, 对话消息, 工具调用, 模型响应};
    use hm_error::{Error, Result};
    use hm_execute_contract::执行器;

    /// 模拟对话器：按预设序列依次返回模型响应，验证契约可插拔
    struct 模拟对话器 {
        响应序列: Mutex<VecDeque<模型响应>>,
    }

    impl 模拟对话器 {
        fn 新(序列: Vec<模型响应>) -> Self {
            模拟对话器 { 响应序列: Mutex::new(序列.into()) }
        }
    }

    impl Component for 模拟对话器 {
        fn name(&self) -> &'static str { "模拟对话器" }
    }

    impl 工具对话器 for 模拟对话器 {
        fn 对话(&self, _消息: Vec<对话消息>, _工具: Vec<serde_json::Value>) -> Result<模型响应> {
            let mut 序列 = self.响应序列.lock().expect("模拟对话器 锁中毒");
            序列.pop_front().ok_or_else(|| Error::Other("对话序列已耗尽".into()))
        }
    }

    /// 模拟执行器：记录每个工具的调用参数并返回预设内容
    struct 模拟执行器 {
        读文件记录: Mutex<Vec<String>>,
        读文件返回: String,
    }

    impl 模拟执行器 {
        fn 新() -> Self {
            模拟执行器 {
                读文件记录: Mutex::new(Vec::new()),
                读文件返回: "文件内容".to_string(),
            }
        }
    }

    impl Component for 模拟执行器 {
        fn name(&self) -> &'static str { "模拟执行器" }
    }

    impl 执行器 for 模拟执行器 {
        fn 读文件(&self, 路径: &str) -> Result<String> {
            self.读文件记录.lock().expect("模拟执行器读记录 锁中毒").push(路径.to_string());
            Ok(self.读文件返回.clone())
        }

        fn 写文件(&self, _路径: &str, _内容: &str) -> Result<()> {
            Ok(())
        }

        fn 运行命令(&self, _命令: &str) -> Result<String> {
            Ok("命令输出".to_string())
        }

        fn 列目录(&self, _路径: &str) -> Result<String> {
            Ok("（空目录）".to_string())
        }

        fn 按名找文件(&self, _模式: &str) -> Result<String> {
            Ok("（无匹配）".to_string())
        }

        fn 搜索内容(&self, _关键词: &str) -> Result<String> {
            Ok("（无匹配）".to_string())
        }

        fn 精确编辑(&self, _路径: &str, _旧: &str, _新: &str) -> Result<String> {
            Ok("替换成功（1 处）".to_string())
        }

        fn 删除文件(&self, _路径: &str) -> Result<String> {
            Ok("已删除".to_string())
        }
    }

    /// 构造工具调用（参数为 JSON 字符串）
    fn 工具调用(名称: &str, 参数: &str) -> 工具调用 {
        工具调用 { id: "call_1".into(), 名称: 名称.into(), 参数: 参数.into() }
    }

    #[test]
    fn 智能体_退化循环_连续同参失败触发熔断() {
        // 回归：LLM 卡死在同一失败调用上（如空参读文件），连续同签名失败达熔断阈值应终止本轮，
        // 而非一路空转到最大轮数（2026-09-11 实测曾空转 60 轮耗尽轮数）
        let 空参读文件 = || 模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{}"#)] };
        let 对话器 = Arc::new(模拟对话器::新(vec![
            空参读文件(), 空参读文件(), 空参读文件(), 空参读文件(), 空参读文件(),
            空参读文件(), 空参读文件(), 空参读文件(),
        ]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        let 结果 = 智能体.运行("陷入退化循环".into());
        let 错误 = 结果.expect_err("应熔断返回错误").to_string();
        assert!(错误.contains("退化循环熔断"), "错误应标识退化循环熔断，实际: {错误}");
    }

    #[test]
    fn 智能体_退化循环_告警后自纠不熔断() {
        // 连续同参失败达告警阈值后，若 LLM 被纠正提示唤醒并改用正确参数，循环应正常收尾（不误杀）。
        // 阈值下调后：告警阈值 2 / 熔断阈值 3，故此处空参失败 2 次即达告警，第 3 步换参后成功归零。
        let 空参读文件 = || 模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{}"#)] };
        let 对话器 = Arc::new(模拟对话器::新(vec![
            空参读文件(), 空参读文件(),
            模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{"路径":"a.txt"}"#)] },
            模型响应 { 思考: None, 内容: Some("已修正完成".into()), 工具调用: vec![] },
        ]));
        let 执行器 = Arc::new(模拟执行器::新());
        let 执行器_记录 = 执行器.clone();
        let 智能体 = 智能体::new(对话器, 执行器, 10);

        let 答复 = 智能体.运行("修正参数".into()).expect("告警后自纠不应熔断");
        assert_eq!(答复, "已修正完成");
        assert_eq!(执行器_记录.读文件记录.lock().expect("锁").len(), 1, "修正参数后应成功读取一次");
    }

    #[test]
    fn 智能体_退化循环_参数仅空白差异_仍累积熔断() {
        // 回归（2026-09-11 真机暴露）：旧实现签名直接拼接原始 JSON 字符串，
        // `{}` 与 `{ }` 因空白差异被判为不同签名致计数归零，熔断永不累积；
        // 规范化后二者同签名，第 3 次同签名失败应熔断。
        let 空参 = |原文: &'static str| 模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", 原文)] };
        let 对话器 = Arc::new(模拟对话器::新(vec![空参(r#"{}"#), 空参(r#"{ }"#), 空参(r#"{}"#), 空参(r#"{ }"#)]));
        let 智能体 = 智能体::new(对话器, Arc::new(模拟执行器::新()), 10);

        let 错误 = 智能体.运行("空白差异".into()).expect_err("应熔断返回错误").to_string();
        assert!(错误.contains("退化循环熔断"), "错误应标识退化循环熔断，实际: {错误}");
    }

    #[test]
    fn 智能体_退化检测器_跨实例共享累积熔断() {
        // 回归（2026-09-11 真机暴露）：驱动器每轮新建智能体，计数随实例创建被重置，
        // 6 次工具失败分散在 5 个实例致熔断永不触发；注入同一共享检测器后应跨实例累积。
        let 检测器 = hm_agent::退化检测器::新();
        let 空参 = || 模型响应 { 思考: None, 内容: None, 工具调用: vec![工具调用("读文件", r#"{}"#)] };
        let 第一 = 智能体::new(
            Arc::new(模拟对话器::新(vec![空参(), 空参(), 模型响应 { 思考: None, 内容: Some("暂止".into()), 工具调用: vec![] }])),
            Arc::new(模拟执行器::新()),
            10,
        )
        .装配退化检测器(检测器.clone());
        let 第二 = 智能体::new(
            Arc::new(模拟对话器::新(vec![空参()])),
            Arc::new(模拟执行器::新()),
            10,
        )
        .装配退化检测器(检测器.clone());

        assert_eq!(第一.运行("首实例两次失败".into()).expect("两次失败未达熔断阈值，应正常收尾"), "暂止");
        let 错误 = 第二.运行("续实例第三次失败".into()).expect_err("跨实例累积第三个同签名失败应熔断").to_string();
        assert!(错误.contains("退化循环熔断"), "错误应标识退化循环熔断，实际: {错误}");
    }
}
