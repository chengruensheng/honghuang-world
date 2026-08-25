//! 角色 - 偏好：13 角色默认工具模式（YAML 配置，数据驱动）。
//!
//! 字段语义：
//! - 层级: 顶层 / 中层 / 底层（鸿钧居顶层，6 准圣居底层）
//! - 默认工具: 此角色优先使用的工具名列表（来自 daoshu_fu::工具清单::清单()）
//! - 默认禁写: true=不能写文件（评审/验收），false=可写文件（执行）
//! - 描述: 一句话角色定位
//!
//! 设计：偏好是默认值不是硬约束（多智能体 §1.5.6）。
//! LLM 看工具 manifest schema 决定调哪个工具；偏好表只注入系统提示词作为
//! 「默认使用顺序与权限」。例：女娲偏好禁写，但若设计需要她写代码内文档注释，
//! LLM 可打破偏好。
//!
//! 加载路径：`<工作区根>/config/角色偏好.yaml`。文件不存在返回空 HashMap + warn，
//! 不阻断运行（v1 阶段偏好表是「推荐」而非「必须」）。

use rizhi_fu::{debug, info, warn};
use serde::{Deserialize, Serialize};
use shihai_fu::世界结果;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

/// 角色偏好单条记录。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 角色偏好 {
    /// 角色层级：顶层 / 中层 / 底层。
    pub 层级: String,
    /// 角色默认工具名列表（建议顺序，LLM 可自选）。
    pub 默认工具: Vec<String>,
    /// true=不能写文件（评审/验收），false=可写文件（执行）。
    pub 默认禁写: bool,
    /// 一句话角色定位。
    pub 描述: String,
}

/// 偏好表默认值（文件不存在时返回）。
fn 空偏好表() -> HashMap<String, 角色偏好> {
    HashMap::new()
}

/// 从指定工作区根加载偏好表。
///
/// 文件路径：`<工作区根>/config/角色偏好.yaml`。
/// 文件不存在 → 返回空表 + warn（不报错，项目可无偏好表运行）。
/// 文件存在但 YAML 解析失败 → 返回错误（配置错误必须显式）。
pub fn 加载(工作区根: &Path) -> 世界结果<HashMap<String, 角色偏好>> {
    let 路径 = 工作区根.join("config").join("角色偏好.yaml");
    if !路径.exists() {
        warn!(
            路径 = %路径.display(),
            "角色偏好表不存在，使用空表（项目可无偏好表运行）"
        );
        return Ok(空偏好表());
    }
    let 内容 = match std::fs::read_to_string(&路径) {
        Ok(内容) => 内容,
        Err(错误) => return Err(format!("读取角色偏好表失败：{错误}").into()),
    };
    #[allow(deprecated)] // serde_yaml 0.9 已 deprecated 但功能稳定，0.9 内无替代品
    let 偏好们: HashMap<String, 角色偏好> = match serde_yaml::from_str(&内容) {
        Ok(map) => map,
        Err(错误) => return Err(format!("解析角色偏好表失败：{错误}").into()),
    };
    info!(
        角色数 = 偏好们.len(),
        路径 = %路径.display(),
        "角色偏好表加载完成"
    );
    Ok(偏好们)
}

/// 按角色名取偏好。角色名不在表里返回 None。
pub fn 按角色取<'a>(
    偏好: &'a HashMap<String, 角色偏好>,
    角色: &str,
) -> Option<&'a 角色偏好> {
    偏好.get(角色)
}

/// 渲染偏好段为提示词（注入到 LLM 系统提示末尾，非强制，仅作默认值参考）。
///
/// 格式：
/// ```
/// 【角色偏好·非强制】
/// 角色 X 默认工具：[A, B, C]
/// 禁写：true
/// 描述：……
/// ```
pub fn 渲染偏好段(角色: &str, 偏好: &角色偏好) -> String {
    let 工具列表 = if 偏好.默认工具.is_empty() {
        "（无）".to_string()
    } else {
        偏好.默认工具.join("、")
    };
    debug!(角色 = %角色, 工具数 = 偏好.默认工具.len(), "角色偏好已渲染为提示段");
    format!(
        "\n【角色偏好·非强制】\n角色 {角色} 默认工具：{工具列表}\n禁写：{}\n描述：{}\n",
        if 偏好.默认禁写 { "true" } else { "false" },
        偏好.描述
    )
}

/// 全局偏好表静态缓存：首次调用 加载 工作区根 读盘并缓存，后续直接返回。
/// 工作区根不可变（项目级常量），缓存一次即可。
static 全局偏好: OnceLock<HashMap<String, 角色偏好>> = OnceLock::new();

/// 初始化（懒加载）并取全局偏好表静态引用。
///
/// 首次调用时从指定工作区根读取 YAML；后续调用直接返回静态引用（零 IO）。
/// 文件不存在或解析失败 → 返回空表（不阻断项目运行）。
pub fn 初始化全局(工作区根: &Path) -> &'static HashMap<String, 角色偏好> {
    全局偏好.get_or_init(|| match 加载(工作区根) {
        Ok(表) => 表,
        Err(错误) => {
            // 配置错误走 fallback：空表 + warn，不阻断主流程。
            // 设计：偏好是「推荐」非「必须」，v1 阶段单点失败不应让派遣执行崩溃。
            warn!(错误 = %错误, "角色偏好表初始化失败，使用空表（偏好不可用，LLM 仅看工具清单）");
            空偏好表()
        }
    })
}

#[cfg(test)]
mod 测试 {
    use super::*;

    /// 单角色偏好 YAML 解析：四字段（层级/默认工具/默认禁写/描述）一一对应。
    /// 防 YAML 反序列化漏字段（如「默认工具」被误写为「default_tools」）吞错。
    #[test]
    fn 单角色偏好_yaml四字段解析() {
        let yaml = r#"
层级: 顶层
默认工具: [读文件, 搜索内容]
默认禁写: true
描述: 单元测试用的偏好样例
"#;
        let 偏好: 角色偏好 = serde_yaml::from_str(yaml).expect("四字段 YAML 应解析成功");
        assert_eq!(偏好.层级, "顶层");
        assert_eq!(偏好.默认工具, vec!["读文件", "搜索内容"]);
        assert!(偏好.默认禁写);
        assert_eq!(偏好.描述, "单元测试用的偏好样例");
    }

    /// 多角色 HashMap 解析：与 角色偏好.yaml 顶层结构一致。
    /// 鸿钧/老子两角色作代表，验证中文键 + 多角色并存可正常反序列化。
    #[test]
    fn 多角色偏好_yaml_鸿钧与老子并存() {
        let yaml = r#"
鸿钧:
  层级: 顶层
  默认工具: [读文件, 搜索内容]
  默认禁写: false
  描述: 道祖·主政

老子:
  层级: 中层
  默认工具: [读文件]
  默认禁写: true
  描述: 圣人·道德
"#;
        let 表: HashMap<String, 角色偏好> =
            serde_yaml::from_str(yaml).expect("多角色 YAML 应解析成功");
        assert_eq!(表.len(), 2, "应为 2 个角色");
        let 鸿钧 = 表.get("鸿钧").expect("鸿钧应存在");
        assert_eq!(鸿钧.层级, "顶层");
        assert!(!鸿钧.默认禁写);
        let 老子 = 表.get("老子").expect("老子应存在");
        assert_eq!(老子.层级, "中层");
        assert!(老子.默认禁写);
    }

    /// 渲染偏好段：四要素必须全部出现（角色名/工具列表/禁写/描述）。
    /// 工具列表用顿号拼接，无工具渲染为「（无）」。
    #[test]
    fn 渲染偏好段_含四要素_工具用顿号拼接() {
        let 偏好 = 角色偏好 {
            层级: "底层".to_string(),
            默认工具: vec![
                "读文件".to_string(),
                "搜索内容".to_string(),
                "应用补丁".to_string(),
            ],
            默认禁写: false,
            描述: "测试角色描述".to_string(),
        };
        let 段 = 渲染偏好段("测试角色", &偏好);
        assert!(段.contains("【角色偏好·非强制】"), "应含非强制段头：{段}");
        assert!(段.contains("角色 测试角色"), "应含角色名：{段}");
        assert!(
            段.contains("读文件、搜索内容、应用补丁"),
            "工具列表应用顿号拼接：{段}"
        );
        assert!(段.contains("禁写：false"), "应含禁写标记：{段}");
        assert!(段.contains("测试角色描述"), "应含描述：{段}");
    }

    /// 渲染偏好段：默认工具为空时渲染为「（无）」，禁写为 true 时渲染 true。
    #[test]
    fn 渲染偏好段_空工具渲染无_禁写true渲染true() {
        let 偏好 = 角色偏好 {
            层级: "中层".to_string(),
            默认工具: Vec::new(),
            默认禁写: true,
            描述: "空工具偏好".to_string(),
        };
        let 段 = 渲染偏好段("空工具角色", &偏好);
        assert!(段.contains("（无）"), "空工具列表应渲染为「（无）」：{段}");
        assert!(段.contains("禁写：true"), "禁写 true 应原样渲染：{段}");
    }

    /// 按角色取：表内角色返回 Some，表外角色返回 None（不抛错）。
    #[allow(non_snake_case)]
    #[test]
    fn 按角色取_表内返回_表外返回_some与无() {
        let mut 表 = HashMap::new();
        表.insert(
            "鸿钧".to_string(),
            角色偏好 {
                层级: "顶层".to_string(),
                默认工具: vec!["读文件".to_string()],
                默认禁写: false,
                描述: "道祖".to_string(),
            },
        );
        assert!(按角色取(&表, "鸿钧").is_some(), "表内角色应能取到");
        assert!(
            按角色取(&表, "不存在的角色").is_none(),
            "表外角色应返回 None"
        );
    }

    /// 加载：文件不存在 → 空表 + 不报错（项目可无偏好表运行）。
    /// 用临时目录（保证路径不存在），验证 fallback 行为。
    #[test]
    fn 加载_文件不存在_返回空表不报错() {
        let 临时根 = std::env::temp_dir().join(format!(
            "角色偏好测试-不存在-{}-{}",
            std::process::id(),
            std::sync::atomic::AtomicU64::fetch_add(
                &COUNTER,
                1,
                std::sync::atomic::Ordering::SeqCst,
            )
        ));
        let _ = std::fs::remove_dir_all(&临时根);
        std::fs::create_dir_all(&临时根).unwrap();
        let 偏好们 = 加载(&临时根).expect("文件不存在应返回空表，不应报错");
        assert!(偏好们.is_empty(), "文件不存在应返回空 HashMap");
        let _ = std::fs::remove_dir_all(&临时根);
    }

    /// 加载：文件存在且为合法 YAML → 返回表且字段正确。
    /// 构造一个临时目录并写入合法偏好 YAML，验证端到端加载。
    #[test]
    fn 加载_文件存在且合法_返回完整表() {
        let 序号 = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let 临时根 =
            std::env::temp_dir().join(format!("角色偏好测试-合法-{序号}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&临时根);
        std::fs::create_dir_all(临时根.join("config")).unwrap();
        let yaml = r#"
女娲:
  层级: 中层
  默认工具: [读文件, 搜索内容]
  默认禁写: true
  描述: 圣人·造化
"#;
        std::fs::write(临时根.join("config").join("角色偏好.yaml"), yaml).unwrap();
        let 偏好们 = 加载(&临时根).expect("合法 YAML 应加载成功");
        assert_eq!(偏好们.len(), 1, "应解析出 1 个角色");
        let 女娲 = 偏好们.get("女娲").expect("女娲应存在");
        assert_eq!(女娲.层级, "中层");
        assert!(女娲.默认禁写);
        let _ = std::fs::remove_dir_all(&临时根);
    }

    /// 加载：文件存在但 YAML 不合法 → 返回错误（配置错误必须显式）。
    /// 与「文件不存在」不同——存在但坏掉的配置不应静默走空表 fallback，
    /// 否则偏好表误改后没人发现。
    #[allow(non_snake_case)]
    #[test]
    fn 加载_yaml非法_返回错误() {
        let 序号 = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let 临时根 =
            std::env::temp_dir().join(format!("角色偏好测试-非法-{序号}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&临时根);
        std::fs::create_dir_all(临时根.join("config")).unwrap();
        // 字段类型不匹配：默认工具应为数组，强行写字符串
        std::fs::write(
            临时根.join("config").join("角色偏好.yaml"),
            "鸿钧:\n  默认工具: 不是数组\n",
        )
        .unwrap();
        let 结果 = 加载(&临时根);
        assert!(结果.is_err(), "非法 YAML 应返回错误而非空表");
        let _ = std::fs::remove_dir_all(&临时根);
    }

    /// 初始化全局：重复调用返回同一引用（OnceLock 缓存路径），不重复读盘。
    /// 测试两次调用取到的 HashMap 引用相等（pointer equality）。
    #[test]
    fn 初始化全局_幂等且返回同一引用() {
        let 序号 = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let 临时根 =
            std::env::temp_dir().join(format!("角色偏好测试-全局-{序号}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&临时根);
        std::fs::create_dir_all(&临时根).unwrap();
        // 注意：本测试的全局缓存可能被同进程其他测试污染（OnceLock 跨测试持久），
        // 因此只断言「同一引用」与「非 panic」，不断言具体内容。
        let a = 初始化全局(&临时根) as *const _;
        let b = 初始化全局(&临时根) as *const _;
        assert_eq!(a, b, "二次调用应返回同一静态引用（OnceLock 缓存）");
        let _ = std::fs::remove_dir_all(&临时根);
    }

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
}
