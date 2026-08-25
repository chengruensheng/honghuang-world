//! 工具 - 清单：manifest 驱动注册表。
//!
//! 工具按 manifest 项登记：每个工具有 schema + tags + side_effects + 限流，
//! 编排器（LLM）看 schema 决定怎么用，**不再写硬编码调度逻辑**。
//! 契约字段语义见 上下文.md §十一 与 多智能体架构设计.md §1.5.6。
//!
//! 内部表示：manifest 是项目侧「工具意图描述」（含 tags / 副作用）。
//! 外部接口：`清单()` 给编排器用；`全部工具定义()` 由 manifest 转换得到
//! OpenAI 兼容 schema（Vec<moxing_fu::工具定义>），签名与历史兼容。
//!
//! 可并行性判定（§11.3）：`tags` 含「可并行」且 `side_effects = 副作用::无`
//! 才允许并发（探查类天然并行；编辑/执行/版本类禁止并行）。

use moxing_fu::工具定义;
use rizhi_fu::info;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 工具副作用分级（与上下文.md §11.2 字段语义一一对应）。
///
/// - `无` = 纯读 / 无副作用（探查类）；
/// - `修改` = 修改文件或本地状态（编辑类）；
/// - `外部` = 调用外部进程 / 外部系统（执行类与编排递归调用类）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum 副作用 {
    无,
    修改,
    外部,
}

impl fmt::Display for 副作用 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            副作用::无 => f.write_str("无"),
            副作用::修改 => f.write_str("修改"),
            副作用::外部 => f.write_str("外部"),
        }
    }
}

/// 工具 manifest 项：项目侧工具意图描述，供编排器查。
///
/// 与 moxing_fu::工具定义 的差异：manifest 多 tags / side_effects / rate_limit
/// 三类编排元数据；OpenAI 兼容 schema 仅看 name / description / parameters。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct 工具清单项 {
    /// 工具唯一标识（与手脚-施展-殿 注册名一致，编排器按此查工具）。
    pub 名字: String,
    /// LLM 看这句决定是否调此工具（一句话讲清用途+输入+输出）。
    pub 描述: String,
    /// 输入参数 JSON-Schema，LLM 据此构造调用参数。
    pub 参数: serde_json::Value,
    /// 输出 JSON-Schema（编排器解析返回值；当前统一以空 object 占位，由执行结果自行产出）。
    pub 输出: serde_json::Value,
    /// 标签：探查/编辑/执行/版本/编排、可并行、只读、无副作用。
    pub 标签: Vec<String>,
    /// 副作用分级（影响是否允许并发）。
    pub 副作用: 副作用,
    /// 每分钟最大调用次数；None = 不限。
    pub 限流每分钟: Option<u32>,
}

impl 工具清单项 {
    /// 是否可并行：tags 含「可并行」且副作用 = 无（上下文.md §11.3）。
    pub fn 可并行(&self) -> bool {
        self.副作用 == 副作用::无 && self.标签.iter().any(|t| t == "可并行")
    }
}

/// 把 manifest 项转为 OpenAI 兼容 schema（moxing_fu::工具定义）。
/// 字段映射：name → 名字、description → 描述、parameters → 参数。
fn manifest_转_openai(项: &工具清单项) -> 工具定义 {
    工具定义 {
        名字: 项.名字.clone(),
        描述: 项.描述.clone(),
        参数: 项.参数.clone(),
    }
}

/// 13 工具 manifest 清单（10 改造 + 3 新增）。
///
/// 顺序与现有 全部工具定义() 一致（前 10 个），新增 3 个附末尾：
/// - 应用补丁、任务规划、查询大模型。
///
/// 调用方按 manifest 直接路由；OpenAI 兼容 schema 见 全部工具定义()。
pub fn 清单() -> Vec<工具清单项> {
    vec![
        // ===== 探查类（可并行 + 无副作用）=====
        工具清单项 {
            名字: "写文件".to_string(),
            描述: "写入或覆盖一个文件。路径相对工作区根，内容为完整文件内容；大文件请一次写全。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "路径": {"type": "string", "description": "相对工作区根的文件路径，如 鸿蒙/基础设施 - 域/道术施展-府/入口.rs"},
                    "内容": {"type": "string", "description": "完整文件内容"}
                },
                "required": ["路径", "内容"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"结果": {"type": "string"}, "字节数": {"type": "integer"}}}),
            标签: vec!["编辑".to_string(), "修改".to_string()],
            副作用: 副作用::修改,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "读文件".to_string(),
            描述: "读取一个文件的内容（相对工作区根），用于了解现状。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "路径": {"type": "string", "description": "相对工作区根的文件路径"}
                },
                "required": ["路径"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"内容": {"type": "string"}}}),
            标签: vec!["探查".to_string(), "可并行".to_string(), "无副作用".to_string()],
            副作用: 副作用::无,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "改文件".to_string(),
            描述: "在文件内把一段旧文本替换为新文本（精确匹配，只替换第一处）。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "路径": {"type": "string", "description": "相对工作区根的文件路径"},
                    "旧文": {"type": "string", "description": "要被替换的原文片段"},
                    "新文": {"type": "string", "description": "替换后的新文本"}
                },
                "required": ["路径", "旧文", "新文"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"结果": {"type": "string"}}}),
            标签: vec!["编辑".to_string(), "修改".to_string()],
            副作用: 副作用::修改,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "删文件".to_string(),
            描述: "删除一个或多个文件（相对工作区根）。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "路径们": {"type": "array", "items": {"type": "string"}, "description": "要删除的文件路径列表"}
                },
                "required": ["路径们"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"删除数": {"type": "integer"}}}),
            标签: vec!["编辑".to_string(), "修改".to_string()],
            副作用: 副作用::修改,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "列举目录".to_string(),
            描述: "列出一个目录下的条目（名称、是否目录、字节数），用于了解结构。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "路径": {"type": "string", "description": "相对工作区根的目录路径，空串或省略表示工作区根"}
                },
                "required": ["路径"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"条目们": {"type": "array", "items": {"type": "object"}}}}),
            标签: vec!["探查".to_string(), "可并行".to_string(), "无副作用".to_string()],
            副作用: 副作用::无,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "寻找文件".to_string(),
            描述: "在目录树下按文件名通配模式寻找文件（如 *.rs）。根必须是目录，禁止填文件路径。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "根": {"type": "string", "description": "检索根目录（只能是目录，禁止文件路径），如 鸿蒙/基础设施 - 域；空串表示工作区根"},
                    "模式": {"type": "string", "description": "文件名通配模式，如 *.rs"}
                },
                "required": ["根", "模式"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"命中们": {"type": "array", "items": {"type": "string"}}}}),
            标签: vec!["探查".to_string(), "可并行".to_string(), "无副作用".to_string()],
            副作用: 副作用::无,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "搜索内容".to_string(),
            描述: "在目录树下按字面串检索文本行（返回文件路径、行号、行内容）。根必须是目录，禁止填文件路径。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "根": {"type": "string", "description": "检索根目录（只能是目录，禁止文件路径），如 鸿蒙/基础设施 - 域；空串表示工作区根"},
                    "字面串": {"type": "string", "description": "要检索的字面文本"}
                },
                "required": ["根", "字面串"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"命中们": {"type": "array", "items": {"type": "object"}}}}),
            标签: vec!["探查".to_string(), "可并行".to_string(), "无副作用".to_string()],
            副作用: 副作用::无,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "运行命令".to_string(),
            描述: "在工作区根执行一条命令（如 cargo build），返回退出码与输出，用于验证。命令在沙箱隔离视图内执行：构建物不落真实盘面，改写源码等越界写入会被自动拦截并回滚（会如实报告）。可指定超时毫秒（默认 600000，上限 600000），超时后子进程被强杀并返回超时错误。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "命令": {"type": "string", "description": "可执行命令名，如 cargo"},
                    "参数们": {"type": "array", "items": {"type": "string"}, "description": "命令参数，如 [build, --workspace, --lib]"},
                    "工作目录": {"type": "string", "description": "可选，相对工作区根的工作目录；省略则用工作区根"},
                    "超时毫秒": {"type": "integer", "description": "可选，超时上限（毫秒），必须在 (0, 600000] 区间；省略则用默认 600000（10 分钟）。超时后子进程被强杀并返回超时错误。"}
                },
                "required": ["命令", "参数们"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"退出码": {"type": "integer"}, "标准输出": {"type": "string"}, "标准错误": {"type": "string"}}}),
            标签: vec!["执行".to_string(), "修改".to_string()],
            副作用: 副作用::外部,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "读格位".to_string(),
            描述: "读取世界记忆体中某个格位的链头集（按实体键分组取最新）。路径相对工作区根时可选，缺省时从工作区 .上下文/格位 读取。返回该格位最新 N 条记录。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "格位名": {"type": "string", "description": "格位名字，如「结构」「铁律·总纲」"},
                    "上限": {"type": "integer", "description": "最多返回的记录条数（默认 20，上限 200）"}
                },
                "required": ["格位名"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"记录们": {"type": "array", "items": {"type": "object"}}}}),
            标签: vec!["探查".to_string(), "可并行".to_string(), "无副作用".to_string()],
            副作用: 副作用::无,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "查格位历史".to_string(),
            描述: "读取世界记忆体中某个格位的全部历史记录（按写入顺序），用于回溯格位链头之外的旧条目。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "格位名": {"type": "string", "description": "格位名字"},
                    "起始": {"type": "integer", "description": "从第几条开始（0 基，默认 0）"},
                    "上限": {"type": "integer", "description": "最多返回的记录条数（默认 50，上限 500）"}
                },
                "required": ["格位名"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"记录们": {"type": "array", "items": {"type": "object"}}}}),
            标签: vec!["探查".to_string(), "可并行".to_string(), "无副作用".to_string()],
            副作用: 副作用::无,
            限流每分钟: None,
        },
        // ===== 新增 3 工具（任务 2 落位）=====
        工具清单项 {
            名字: "应用补丁".to_string(),
            描述: "应用 unified diff 格式补丁到工作区根相对路径。补丁文本须为标准 unified diff 格式（含 --- / +++ / @@ hunk 头）。用于按 diff 落盘改动，避免把整文件灌进 LLM 上下文。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "补丁": {"type": "string", "description": "unified diff 文本（含 --- / +++ / @@ hunk 头）"},
                    "干跑": {"type": "boolean", "description": "可选 true：仅校验补丁合法并返回将改动摘要，不实际写入（默认 false）"}
                },
                "required": ["补丁"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"应用文件们": {"type": "array", "items": {"type": "string"}}, "hunks": {"type": "integer"}}}),
            标签: vec!["编辑".to_string(), "修改".to_string()],
            副作用: 副作用::修改,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "任务规划".to_string(),
            描述: "把当前任务拆成若干子任务（带依赖关系），返回子任务列表。LLM 子规划调用——把单步难以直接完成的任务拆成可串/并行的子步骤，供编排器按依赖调度。子任务们的 依赖 字段写子任务名列表。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "目标": {"type": "string", "description": "本次规划要达成的目标（与原任务目标对齐）"},
                    "上下文摘要": {"type": "string", "description": "可选，把当前已知要点压缩进规划 prompt"},
                    "上限步数": {"type": "integer", "description": "可选，最多拆出多少个子任务（默认 5，上限 20）"}
                },
                "required": ["目标"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"子任务们": {"type": "array", "items": {"type": "object"}}}}),
            标签: vec!["编排".to_string(), "可并行".to_string(), "无副作用".to_string()],
            副作用: 副作用::无,
            限流每分钟: None,
        },
        工具清单项 {
            名字: "查询大模型".to_string(),
            描述: "递归调用大模型——把一段提示词直接发给当前角色 LLM 池，返回纯文本回答。用于子规划/二次询问等场景（如「让模型把这份证据摘要成 3 句话」）。注意：会消耗额外 token，且结果不直接落盘。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "提示词": {"type": "string", "description": "发给模型的完整提示词"},
                    "系统提示": {"type": "string", "description": "可选，覆盖默认系统提示"},
                    "最大输出": {"type": "integer", "description": "可选，最大输出 token 上限（默认 1024）"}
                },
                "required": ["提示词"]
            }),
            输出: serde_json::json!({"type": "object", "properties": {"文本": {"type": "string"}, "用量": {"type": "object"}}}),
            标签: vec!["编排".to_string(), "可并行".to_string(), "外部".to_string()],
            副作用: 副作用::外部,
            限流每分钟: Some(60),
        },
    ]
}

/// 把 manifest 清单转换为 OpenAI 兼容 schema 列表（Vec<moxing_fu::工具定义>）。
///
/// 用途：把项目侧 manifest 投影成 LLM API 兼容的工具定义，供 调用模型带工具 使用。
/// 字段投影规则：name → 名字、description → 描述、parameters → 参数；tags / 副作用 /
/// 限流每分钟 不进 OpenAI schema（仅作编排元数据）。
pub fn 清单_转_openai(项们: &[工具清单项]) -> Vec<工具定义> {
    项们.iter().map(manifest_转_openai).collect()
}

/// 启动期注册占位（工具清单是静态数据，本函数仅埋日志埋点，AGENTS §6）。
/// 设计：v1 期间启动期注册（manifest 在源码里静态给出），v2 再做热重载或扫盘注册。
/// 调用方（daoshu_fu 应用钩子）首次调用 清单() 时会自动触发本函数埋日志点。
pub fn 启动期注册(项们: &[工具清单项]) {
    let 总数 = 项们.len();
    let 可并行数 = 项们.iter().filter(|项| 项.可并行()).count();
    let 修改数 = 项们.iter().filter(|项| 项.副作用 == 副作用::修改).count();
    let 外部数 = 项们.iter().filter(|项| 项.副作用 == 副作用::外部).count();
    info!(
        总数,
        可并行数, 修改数, 外部数, "daoshu_fu 工具清单初始化完成"
    );
}

/// 启动期注册（首次调用 清单() 时自动触发）。
/// 用 OnceLock 守护一次性，避免重复埋点。
static 启动期注册守卫: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// 取工具清单（项目侧 manifest 列表）。
///
/// 首次调用触发 启动期注册 埋日志点，后续调用直接返回静态数据。
pub fn 清单_带启动钩子() -> Vec<工具清单项> {
    let 项们 = 清单();
    let _ = 启动期注册守卫.get_or_init(|| {
        启动期注册(&项们);
    });
    项们
}

#[cfg(test)]
mod 测试 {
    use super::*;

    /// 13 工具清单完整：10 改造 + 3 新增（应用补丁/任务规划/查询大模型）。
    /// 与 多智能体 §1.5.6「10 固定工具 → 15+ 工具统一清单」v1 阶段对齐（13=10+3）。
    #[test]
    fn 清单_完整13项_含新增3工具() {
        let 项们 = 清单();
        assert_eq!(项们.len(), 13, "应为 10+3=13 个工具");
        let 名字们: Vec<&str> = 项们.iter().map(|项| 项.名字.as_str()).collect();
        assert_eq!(
            名字们,
            vec![
                "写文件",
                "读文件",
                "改文件",
                "删文件",
                "列举目录",
                "寻找文件",
                "搜索内容",
                "运行命令",
                "读格位",
                "查格位历史",
                "应用补丁",
                "任务规划",
                "查询大模型",
            ]
        );
    }

    /// manifest 项字段完整：每个工具有 description / input_schema / tags / 副作用 / 限流。
    /// 防「空 description」「空 tags」「未声明 副作用」漏写——manifest 必须自描述。
    #[test]
    fn 清单_每项字段完整() {
        for 项 in 清单() {
            assert!(!项.名字.is_empty(), "name 不应为空");
            assert!(!项.描述.is_empty(), "{} 描述不应为空", 项.名字);
            assert!(
                !项.参数.is_null() && 项.参数.get("type").is_some(),
                "{} 参数 schema 缺 type 字段",
                项.名字
            );
            assert!(!项.标签.is_empty(), "{} 标签不应为空", 项.名字);
        }
    }

    /// 可并行判定：tags 含「可并行」且副作用 = 无 才允许并发（上下文.md §11.3）。
    /// 探查类（6 个）+ 编排类（任务规划）天然可并行；
    /// 编辑/执行/版本/外部 类即使加了「可并行」也不允许并发。
    /// 与上下文.md §11.5「15+ 工具统一清单」一致：plan_task / query_llm 都是「可并行 + 无副作用」，
    /// 但 query_llm 当前 side_effects=外部 故不在可并行集合（虽然 tags 含「可并行」，
    /// 这是符合 §11.3「副作用 = 无」 AND 规则的：tags=可并行 是 OR 不充分条件）。
    #[test]
    fn 可并行_仅探查与编排类允许() {
        let 项们 = 清单();
        let 可并行: Vec<&str> = 项们
            .iter()
            .filter(|项| 项.可并行())
            .map(|项| 项.名字.as_str())
            .collect();
        // 7 个：6 探查类 + 任务规划（编排 + 可并行 + 无副作用）
        assert_eq!(
            可并行,
            vec![
                "读文件",
                "列举目录",
                "寻找文件",
                "搜索内容",
                "读格位",
                "查格位历史",
                "任务规划",
            ],
            "可并行集合应为 6 个探查类 + 任务规划（编排 + 无副作用）"
        );
        // 编辑/执行/版本（外部）/查询大模型（外部）：不允许并行。
        let 不允许 = [
            "写文件",
            "改文件",
            "删文件",
            "应用补丁",
            "运行命令",
            "查询大模型",
        ];
        for 名字 in 不允许 {
            let 项 = 项们.iter().find(|项| 项.名字 == 名字).unwrap();
            assert!(
                !项.可并行(),
                "{名字} 应不允许并行（副作用 = {:?}）",
                项.副作用
            );
        }
        // 不在可并行集合的工具，副作用必须 ≠ 无。
        for 项 in &项们 {
            if !可并行.contains(&项.名字.as_str()) {
                assert_ne!(项.副作用, 副作用::无, "{} 副作用不应为「无」", 项.名字);
            }
        }
    }

    /// 副作用枚举序列化往返：Serialize+Deserialize 兼容（任务数据进事件总线要可读）。
    #[test]
    fn 副作用_序列化往返() {
        for 副作用 in [副作用::无, 副作用::修改, 副作用::外部] {
            let json = serde_json::to_string(&副作用).unwrap();
            let 回: 副作用 = serde_json::from_str(&json).unwrap();
            assert_eq!(回, 副作用, "副作用 序列化-反序列化应恒等");
        }
    }

    /// 副作用 Display：编排器日志用「无/修改/外部」三字显示。
    #[test]
    fn 副作用_显示中文三字() {
        assert_eq!(副作用::无.to_string(), "无");
        assert_eq!(副作用::修改.to_string(), "修改");
        assert_eq!(副作用::外部.to_string(), "外部");
    }

    /// manifest 清单可转 OpenAI 兼容 schema：清单_转_openai 返回同长度 Vec<工具定义>。
    /// 字段投影正确（名字/描述/参数 三字段一一对应）。
    #[test]
    fn 清单_转_openai_字段投影() {
        let 项们 = 清单();
        let openai = 清单_转_openai(&项们);
        assert_eq!(openai.len(), 项们.len());
        for (项, def) in 项们.iter().zip(openai.iter()) {
            assert_eq!(def.名字, 项.名字);
            assert_eq!(def.描述, 项.描述);
            assert_eq!(def.参数, 项.参数, "参数 schema 应原样投射");
        }
    }

    /// 启动期注册埋点：首次调用 清单_带启动钩子() 应触发日志埋点（OnceLock 守护一次性）。
    /// 测试只验证函数可调用、不 panic、不抛错；日志侧由 rizhi_fu 验证层保证。
    #[test]
    fn 启动期注册_幂等且不抛错() {
        let _ = 清单_带启动钩子();
        let _ = 清单_带启动钩子(); // 二次调用走 OnceLock 缓存路径，不重复埋点
    }

    /// 现有 10 工具（不含新增 3 项）的 manifest tags 与 §11.3 一致：
    /// - 编辑类 tags 含「编辑」「修改」；
    /// - 探查类 tags 含「探查」「可并行」「无副作用」；
    /// - 执行类 tags 含「执行」「外部」。
    #[test]
    fn 现有10工具_tags_与契约一致() {
        let 项们 = 清单();
        let 编辑类 = ["写文件", "改文件", "删文件", "应用补丁"];
        for 名字 in 编辑类 {
            let 项 = 项们.iter().find(|项| 项.名字 == 名字).unwrap();
            assert!(项.标签.iter().any(|t| t == "编辑"), "{名字} 应含 编辑 tag");
            assert_eq!(项.副作用, 副作用::修改, "{名字} 应为 修改 副作用");
        }
        let 探查类 = [
            "读文件",
            "列举目录",
            "寻找文件",
            "搜索内容",
            "读格位",
            "查格位历史",
        ];
        for 名字 in 探查类 {
            let 项 = 项们.iter().find(|项| 项.名字 == 名字).unwrap();
            assert!(项.标签.iter().any(|t| t == "探查"), "{名字} 应含 探查 tag");
            assert!(
                项.标签.iter().any(|t| t == "可并行"),
                "{名字} 应含 可并行 tag"
            );
            assert_eq!(项.副作用, 副作用::无, "{名字} 应为 无 副作用");
        }
        let 执行类 = ["运行命令"];
        for 名字 in 执行类 {
            let 项 = 项们.iter().find(|项| 项.名字 == 名字).unwrap();
            assert!(项.标签.iter().any(|t| t == "执行"), "{名字} 应含 执行 tag");
            assert_eq!(项.副作用, 副作用::外部, "{名字} 应为 外部 副作用");
        }
    }
}
