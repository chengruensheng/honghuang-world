#[cfg(test)]
mod tests {
    use super::super::阶段提示::{阶段提示, 任务快照};
    use super::super::解析与净化::{提取json, 剥离杂质标签};
    use hm_cognition::AgentRole;
    use tc_task::Task;

    #[test]
    fn 阶段提示_太乙金仙_含清理纪律() {
        let 任务 = Task::新建(1, "清理纪律任务".to_string(), "描述".to_string(), 0);
        let 快照 = 任务快照(&任务);
        let 提示 = 阶段提示(&AgentRole::太乙金仙, &快照);
        assert!(提示.contains("按名找文件"), "太乙金仙提示应先扫描残留，实际: {提示}");
        assert!(提示.contains("删除文件"), "太乙金仙提示应要求用删除文件工具清理，实际: {提示}");
        assert!(提示.contains("{\"模式\":\"**/*.bak\"}"), "太乙金仙提示应给出显式模式参数示例（防空参连败），实际: {提示}");
        assert!(提示.contains("重新扫描"), "太乙金仙提示应要求删除后复核为空，实际: {提示}");
        assert!(提示.contains("不得在工具失败时直接宣告清理完成"), "太乙金仙提示应禁止失败即宣告，实际: {提示}");
    }

    #[test]
    fn 提取json_无杂质_原样返回() {
        let 输入 = r#"{"a":1}"#;
        assert_eq!(提取json(输入).as_deref(), Some(r#"{"a":1}"#));
    }

    #[test]
    fn 提取json_后附代码块标记_正确截断() {
        let 输入 = "以下是设计文档：\n```json\n{\"边界定义\":{\"输入\":\"n\"},\"输出\":{}}\n```\n完毕";
        let 提取 = 提取json(输入).expect("应提取到 JSON");
        // 截取到配平的 }，不含尾随 ``` 标记
        assert_eq!(提取, r#"{"边界定义":{"输入":"n"},"输出":{}}"#);
    }

    #[test]
    fn 提取json_字符串内含括号_不误判() {
        // 字符串值里含 { 与 }，括号配平应跳过字符串内容
        let 输入 = r#"{"描述":"斐波那契 F(n) 用 {} 表示边界","值":1}"#;
        assert_eq!(
            提取json(输入).as_deref(),
            Some(r#"{"描述":"斐波那契 F(n) 用 {} 表示边界","值":1}"#)
        );
    }

    #[test]
    fn 提取json_嵌套对象_取最外层() {
        let 输入 = r#"{"轮次":[{"通过":true,"问题":["a","b"]}],"最终结果":true}"#;
        let 提取 = 提取json(输入).expect("应提取到嵌套 JSON");
        assert_eq!(提取, r#"{"轮次":[{"通过":true,"问题":["a","b"]}],"最终结果":true}"#);
    }

    #[test]
    fn 提取json_无括号_返回空() {
        assert!(提取json("没有 JSON 内容").is_none());
    }

    #[test]
    fn 提取json_只有左括号未闭合_返回空() {
        assert!(提取json("内容 { 未闭合").is_none());
    }

    #[test]
    fn 剥离杂质_think标签_完整剥离() {
        let 输入 = r#"<think>内部推理{"key":"val"}</think>

{"代码变更":[],"自检":{}}"#;
        let 净化 = 剥离杂质标签(输入);
        assert!(!净化.contains("内部推理"), "think 内容应被剥离");
        assert!(!净化.contains("<think>"), "think 标签应被剥离");
        let json = 提取json(&净化).expect("应提取到正文 JSON");
        assert!(json.contains("代码变更"), "应提取到正文 JSON 而非 think 内碎片");
    }

    #[test]
    fn 剥离杂质_think大小写不敏感() {
        let 输入 = r#"<Think>思考过程</Think>{"结果":true}"#;
        let 净化 = 剥离杂质标签(输入);
        let json = 提取json(&净化).expect("大写 Think 也应剥离");
        assert_eq!(json, r#"{"结果":true}"#);
    }

    #[test]
    fn 剥离杂质_代码块围栏_提取内容() {
        let 输入 = "以下是文档：\n```json\n{\"边界定义\":{\"输入\":\"n\"}}\n```\n完毕";
        let 净化 = 剥离杂质标签(输入);
        let json = 提取json(&净化).expect("应从代码块中提取 JSON");
        assert_eq!(json, r#"{"边界定义":{"输入":"n"}}"#);
    }

    #[test]
    fn 剥离杂质_无杂质_原样保留() {
        let 输入 = r#"{"直接":"json","值":42}"#;
        assert_eq!(剥离杂质标签(输入), 输入);
    }

    #[test]
    fn 剥离杂质_think未闭合_跳过标签保留后续() {
        let 输入 = r#"<think>未闭合的思考{"正文":true}"#;
        let 净化 = 剥离杂质标签(输入);
        // 未闭合时跳过  标记本身，后续内容当正文
        let json = 提取json(&净化).expect("未闭合 think 后仍应提取 JSON");
        assert_eq!(json, r#"{"正文":true}"#);
    }
}
