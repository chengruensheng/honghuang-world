//! 工具 - 定义：全部工具 schema（与 手脚-施展-殿 一一对应，OpenAI 兼容）。

use moxing_fu::工具定义;

/// 全部工具定义（与 手脚-施展-殿 一一对应，OpenAI 兼容 schema）。
pub fn 全部工具定义() -> Vec<工具定义> {
    vec![
        工具定义 {
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
        },
        工具定义 {
            名字: "读文件".to_string(),
            描述: "读取一个文件的内容（相对工作区根），用于了解现状。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "路径": {"type": "string", "description": "相对工作区根的文件路径"}
                },
                "required": ["路径"]
            }),
        },
        工具定义 {
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
        },
        工具定义 {
            名字: "删文件".to_string(),
            描述: "删除一个或多个文件（相对工作区根）。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "路径们": {"type": "array", "items": {"type": "string"}, "description": "要删除的文件路径列表"}
                },
                "required": ["路径们"]
            }),
        },
        工具定义 {
            名字: "列举目录".to_string(),
            描述: "列出一个目录下的条目（名称、是否目录、字节数），用于了解结构。".to_string(),
            参数: serde_json::json!({
                "type": "object",
                "properties": {
                    "路径": {"type": "string", "description": "相对工作区根的目录路径，空串或省略表示工作区根"}
                },
                "required": ["路径"]
            }),
        },
        工具定义 {
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
        },
        工具定义 {
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
        },
        工具定义 {
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
        },
        工具定义 {
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
        },
        工具定义 {
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
        },
    ]
}
