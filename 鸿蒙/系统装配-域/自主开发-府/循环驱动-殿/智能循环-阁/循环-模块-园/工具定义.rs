use super::super::任务_清单_园::{清单项键_内容, 清单项键_状态, 状态_待办, 状态_进行中, 状态_已完成};
use super::循环模块::{
    对外工具名, 读文件, 写文件, 运行命令, 列目录, 按名找文件, 搜索内容, 精确编辑, 删除文件,
    任务清单, 参数键_路径, 参数键_内容, 参数键_命令, 参数键_模式, 参数键_关键词, 参数键_旧,
    参数键_新, 参数键_清单,
};

/// 九个工具的函数定义（OpenAI function calling 的 tools 数组元素）
pub(crate) fn 工具定义() -> Vec<serde_json::Value> {
    vec![
        单参函数(对外工具名(读文件), "读取文件完整文本。普通相对路径 = 工作区内；`~/路径` = 项目根（只读探索本体代码用）", 参数键_路径, "相对工作区的路径，或 `~/` 开头的项目根路径（只读）"),
        双参函数(对外工具名(写文件), "把内容写入工作区文件（覆盖）。只能写工作区，不接受 ~/ 与绝对路径", 参数键_路径, "相对工作区的文件路径", 参数键_内容, "要写入的完整文本"),
        单参函数(对外工具名(运行命令), "在工作区目录下运行命令，返回标准输出；`~ 命令` 则在项目根下运行（仅只读命令）", 参数键_命令, "要执行的命令；`~ ` 前缀 = 在项目根只读执行"),
        单参函数_可选(对外工具名(列目录), "列出目录下条目（一层，区分目录/文件）；路径可缺省列工作区根，`~/路径` 列项目根", 参数键_路径, "相对工作区的目录路径（缺省为根目录），或 `~/` 开头的项目根路径（只读）"),
        单参函数(对外工具名(按名找文件), "按 glob 模式（*、**、?）递归匹配文件；`~/` 开头则在项目根匹配（只读）", 参数键_模式, "glob 模式，如 **/*.rs；`~/鸿蒙/**/*.rs` 搜项目根"),
        单参函数(对外工具名(搜索内容), "递归搜索文本文件内容，返回匹配行；`~/关键词` 则搜项目根（只读）", 参数键_关键词, "要搜索的关键词；`~/` 前缀 = 搜项目根"),
        三参函数(对外工具名(精确编辑), "把文件中唯一匹配的旧串替换为新串（多处匹配会报错）。只能改工作区", 参数键_路径, "相对工作区的文件路径", 参数键_旧, "要被替换的旧文本（须唯一）", 参数键_新, "替换后的新文本"),
        单参函数(对外工具名(删除文件), "删除工作区内的单个文件（清理临时/备份产物，如 .bak）", 参数键_路径, "相对工作区的文件路径"),
        任务清单函数(),
    ]
}

/// 构造单字符串参数的函数定义
fn 单参函数(名: &str, 描述: &str, 键: &str, 键说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键.to_string(), serde_json::json!({"type": "string", "description": 键说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": [键]
            }
        }
    })
}

/// 构造可选单字符串参数的函数定义（参数可缺省，调用侧给默认值，如「列目录」缺省列工作区根）
fn 单参函数_可选(名: &str, 描述: &str, 键: &str, 键说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键.to_string(), serde_json::json!({"type": "string", "description": 键说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": []
            }
        }
    })
}

/// 构造双字符串参数的函数定义
fn 双参函数(名: &str, 描述: &str, 键一: &str, 键一说明: &str, 键二: &str, 键二说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键一.to_string(), serde_json::json!({"type": "string", "description": 键一说明}));
    属性.insert(键二.to_string(), serde_json::json!({"type": "string", "description": 键二说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": [键一, 键二]
            }
        }
    })
}

/// 构造三字符串参数的函数定义
fn 三参函数(名: &str, 描述: &str, 键一: &str, 键一说明: &str, 键二: &str, 键二说明: &str, 键三: &str, 键三说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键一.to_string(), serde_json::json!({"type": "string", "description": 键一说明}));
    属性.insert(键二.to_string(), serde_json::json!({"type": "string", "description": 键二说明}));
    属性.insert(键三.to_string(), serde_json::json!({"type": "string", "description": 键三说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": [键一, 键二, 键三]
            }
        }
    })
}

/// 构造任务清单工具的函数定义（参数为「清单」数组，每项含「内容」「状态」）
fn 任务清单函数() -> serde_json::Value {
    let mut 清单属性 = serde_json::Map::new();
    清单属性.insert(
        参数键_清单.to_string(),
        serde_json::json!({
            "type": "array",
            "description": "任务条目数组",
            "items": {
                "type": "object",
                "properties": {
                    (清单项键_内容): {"type": "string", "description": "任务内容"},
                    (清单项键_状态): {"type": "string", "enum": [状态_待办, 状态_进行中, 状态_已完成]}
                },
                "required": [清单项键_内容]
            }
        }),
    );
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 任务清单,
            "description": "覆盖式更新多步任务清单（待办/进行中/已完成）",
            "parameters": {
                "type": "object",
                "properties": 清单属性,
                "required": [参数键_清单]
            }
        }
    })
}
