use std::sync::Arc;
use hm_content_contract::{工具对话器, 对话消息};
use hm_error::{Error, Result};
use hm_execute_contract::执行器;

/// 免审后缀：写文件/精确编辑自动产生的备份与常见临时文件，属确定安全类别，走快速通道免审
const 免审后缀: &[&str] = &[".bak", ".tmp", ".orig", ".rej", ".swp"];
/// 审查对话携带的文件内容摘要上限（字符），足够判断文件性质并控制 token
const 摘要上限: usize = 600;

/// 删除审查提示词：独立单轮会话，强制只输出一行 JSON 结论（拿不准一律判不可删）
const 审查提示: &str = "你是删除安全审查员。任务智能体请求删除一个工作区文件，你依据路径与内容摘要判断该删除是否安全。\n\
判为可删（true）：.tmp/.bak/.orig/.rej/.swp 等临时备份、构建产物（如 target/ 内文件）、内容明显是废稿或重复副本。\n\
判为不可删（false）：源代码、配置、文档、测试、清单索引等有保留价值的文件；路径或内容拿不准时一律判 false。\n\
只输出一行 JSON：{\"可删\": true, \"理由\": \"一句话\"}，禁止输出 JSON 以外的任何文字。";

/// 删除防护门（LLM 审查）：任务智能体的删除请求先过这一关，再交执行器安全删除（四步闭环在执行器层）。
///
/// `.bak` 等自动备份后缀走快速通道免审；其余文件发起独立审查对话——审查员是单轮独立会话，
/// 不受主循环上下文压力影响，且能读到文件内容摘要做「是否真实无用」的智商判断。
/// 审查拒绝或结论不可解析时保守不放行，把理由回传给任务智能体重定方案（工具失败必须重试或改道，不许谎报完成）。
pub fn 审查并删除(
    对话器: &Arc<dyn 工具对话器>,
    执行器: &Arc<dyn 执行器>,
    路径: &str,
) -> Result<String> {
    // 快速通道：自动备份/临时后缀确定安全，免审
    let 小写 = 路径.to_lowercase();
    if 免审后缀.iter().any(|后缀| 小写.ends_with(后缀)) {
        return 执行器.删除文件(路径);
    }
    // 取内容摘要给审查员（读取失败不影响审查，给占位说明）
    let 摘要: String = 执行器
        .读文件(路径)
        .unwrap_or_else(|_| "（无法读取内容）".to_string())
        .chars()
        .take(摘要上限)
        .collect();
    let 请求 = format!(
        "待删除文件：{路径}\n内容摘要（前 {摘要上限} 字符）：\n{摘要}\n\n请审查该删除是否安全，只输出一行 JSON 结论。"
    );
    let 响应 = 对话器.对话(
        vec![对话消息::系统(审查提示.to_string()), 对话消息::用户(请求)],
        Vec::new(),
    )?;
    let 文本 = 响应.内容.unwrap_or_default();
    let (可删, 理由) = 解析结论(&文本).ok_or_else(|| {
        Error::Config(format!(
            "删除审查未给出可解析结论（模型输出：{}），保守拒绝删除；如确需删除请提供更明确依据重试",
            截短(&文本, 120)
        ))
    })?;
    if !可删 {
        return Err(Error::Config(format!(
            "删除审查拒绝删除 {路径}：{理由}。请勿删除该文件，如属误判请补充依据后重试"
        )));
    }
    执行器.删除文件(路径)
}

/// 从模型输出解析审查结论：定位首个 `{` 到最后一个 `}` 的 JSON，取「可删」与「理由」
fn 解析结论(文本: &str) -> Option<(bool, String)> {
    let 头 = 文本.find('{')?;
    let 尾 = 文本.rfind('}')?;
    if 尾 < 头 {
        return None;
    }
    let 值: serde_json::Value = serde_json::from_str(&文本[头..=尾]).ok()?;
    let 可删 = 值.get("可删").and_then(|v| v.as_bool())?;
    let 理由 = match 值.get("理由").and_then(|v| v.as_str()) {
        Some(文) => 文.to_string(),
        // 模型未给理由属正常情况，非错误，无需告警
        None => "未说明理由".to_string(),
    };
    Some((可删, 理由))
}

/// 按字符数截短文本（审查失败回显模型输出摘要用），超长以省略号结尾
fn 截短(文本: &str, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        文本.to_string()
    } else {
        let 头部: String = 文本.chars().take(上限).collect();
        format!("{头部}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use hm_contract::Component;
    use hm_content_contract::{工具调用, 模型响应};

    /// 模拟对话器：按预设序列依次返回模型响应（审查对话与主循环共用同一实例）
    struct 序列对话器 {
        序列: Mutex<VecDeque<模型响应>>,
        对话次数: Mutex<usize>,
    }

    impl 序列对话器 {
        fn 新(序列: Vec<模型响应>) -> Self {
            序列对话器 { 序列: Mutex::new(序列.into()), 对话次数: Mutex::new(0) }
        }
    }

    impl Component for 序列对话器 {
        fn name(&self) -> &'static str { "序列对话器" }
    }

    impl 工具对话器 for 序列对话器 {
        fn 对话(&self, _消息: Vec<对话消息>, _工具: Vec<serde_json::Value>) -> hm_error::Result<模型响应> {
            *self.对话次数.lock().expect("锁") += 1;
            self.序列.lock().expect("锁").pop_front().ok_or_else(|| hm_error::Error::Other("序列耗尽".into()))
        }
    }

    /// 模拟执行器：记录删除调用，删除即真实移除临时文件（供存在性断言）
    struct 删录执行器 {
        删除记录: Mutex<Vec<String>>,
    }

    impl 删录执行器 {
        fn 新() -> Self {
            删录执行器 { 删除记录: Mutex::new(Vec::new()) }
        }
    }

    impl Component for 删录执行器 {
        fn name(&self) -> &'static str { "删录执行器" }
    }

    impl 执行器 for 删录执行器 {
        fn 读文件(&self, _路径: &str) -> hm_error::Result<String> {
            Ok("fn 主要() {}".to_string())
        }
        fn 写文件(&self, 路径: &str, _内容: &str) -> hm_error::Result<()> {
            let 全 = std::env::temp_dir().join(format!("zd_review_{路径}").replace('/', "_"));
            std::fs::write(全, "").map_err(hm_error::Error::Io)
        }
        fn 运行命令(&self, _命令: &str) -> hm_error::Result<String> { Ok(String::new()) }
        fn 列目录(&self, _路径: &str) -> hm_error::Result<String> { Ok("（空）".into()) }
        fn 按名找文件(&self, _模式: &str) -> hm_error::Result<String> { Ok("（无匹配）".into()) }
        fn 搜索内容(&self, _关键词: &str) -> hm_error::Result<String> { Ok("（无匹配）".into()) }
        fn 精确编辑(&self, _路径: &str, _旧: &str, _新: &str) -> hm_error::Result<String> { Ok("替换成功".into()) }
        fn 删除文件(&self, 路径: &str) -> hm_error::Result<String> {
            self.删除记录.lock().expect("锁").push(路径.to_string());
            Ok(format!("已安全删除: {路径}"))
        }
    }

    fn 响应(内容: &str) -> 模型响应 {
        模型响应 { 内容: Some(内容.into()), 工具调用: vec![], 思考: None }
    }

    #[test]
    fn 审查并删除_bak后缀走快速通道免审() {
        // 对话器序列为空：若审查发起对话会因序列耗尽而报错，从而证明走了免审通道
        let 对话器具体 = Arc::new(序列对话器::新(vec![]));
        let 对话器: Arc<dyn 工具对话器> = 对话器具体.clone();
        let 执行器具体 = Arc::new(删录执行器::新());
        let 执行器: Arc<dyn 执行器> = 执行器具体.clone();
        let 结果 = 审查并删除(&对话器, &执行器, "src/lib.rs.bak").expect("免审应直接删除");
        assert!(结果.contains("src/lib.rs.bak"));
        assert_eq!(执行器具体.删除记录.lock().expect("锁").len(), 1, "应执行一次删除");
        assert_eq!(*对话器具体.对话次数.lock().expect("锁"), 0, "免审不得发起审查对话");
    }

    #[test]
    fn 审查并删除_审查拒绝时不删除() {
        let 对话器: Arc<dyn 工具对话器> = Arc::new(序列对话器::新(vec![
            响应(r#"{"可删": false, "理由": "源代码有保留价值"}"#),
        ]));
        let 执行器具体 = Arc::new(删录执行器::新());
        let 执行器: Arc<dyn 执行器> = 执行器具体.clone();
        let 错 = 审查并删除(&对话器, &执行器, "src/main.rs").expect_err("拒绝应报错");
        assert!(format!("{错}").contains("拒绝"), "错误应说明审查拒绝: {错}");
        assert!(执行器具体.删除记录.lock().expect("锁").is_empty(), "拒绝后不得执行删除");
    }

    #[test]
    fn 审查并删除_审查通过才执行() {
        let 对话器: Arc<dyn 工具对话器> = Arc::new(序列对话器::新(vec![
            // 结论裹在自然语言里：解析器应能定位 JSON 并放行
            响应(r#"结论：{"可删": true, "理由": "构建产物"}"#),
        ]));
        let 执行器具体 = Arc::new(删录执行器::新());
        let 执行器: Arc<dyn 执行器> = 执行器具体.clone();
        let 结果 = 审查并删除(&对话器, &执行器, "target/调试产物.tmp2").expect("通过应执行删除");
        assert!(结果.contains("target/调试产物.tmp2"));
        assert_eq!(执行器具体.删除记录.lock().expect("锁").len(), 1, "通过后应执行一次删除");
    }

    #[test]
    fn 审查并删除_结论不可解析时保守拒绝() {
        let 对话器: Arc<dyn 工具对话器> = Arc::new(序列对话器::新(vec![
            响应("我觉得可以删，没问题。"),
        ]));
        let 执行器具体 = Arc::new(删录执行器::新());
        let 执行器: Arc<dyn 执行器> = 执行器具体.clone();
        let 错 = 审查并删除(&对话器, &执行器, "doc/说明.md").expect_err("无 JSON 结论应拒绝");
        assert!(format!("{错}").contains("保守拒绝"), "应保守拒绝: {错}");
        assert!(执行器具体.删除记录.lock().expect("锁").is_empty(), "不得执行删除");
    }
}
