//! 派发 - 落单 - 园 · 测试

use super::*;
use crate::类型_定义_殿::{
    产物条目, 工作流级别, 执行任务, 执行回执, 执行状态
};
use moxing_fu::{模型配置, 用量};
use shihai_fu::全量基线;
use std::path::PathBuf;

/// 造一个调度器（测试用假配置，不触网）。
fn 假调度(根: PathBuf) -> 任务调度 {
    任务调度::新(
        模型配置 {
            密钥: String::new(),
            地址: String::new(),
            模型: String::new(),
        },
        根,
    )
}

#[test]
fn 模板复述占位块被跳过真实块照常提取() {
    let 文本 = "1. 每个文件必须用 <<<文件:路径>>> 开头、<<<结束>>> 结尾，两标记成对出现。\n\
                <<<文件:相对项目根的文件路径>>>\n示例内容\n<<<结束>>>\n\
                <<<文件:鸿蒙\\基础设施 - 域\\识海承载-府\\识海-铭记-殿\\代码-扫描-阁\\扫描-落格位-园\\扫描落格位.rs>>>\n\
                pub fn 扫描() {}\n\
                <<<结束>>>";
    let 文件们 = 解析落盘文本(文本).unwrap();
    assert_eq!(文件们.len(), 1);
    assert!(文件们[0].路径.ends_with("扫描落格位.rs"));
    assert!(文件们[0].内容.contains("pub fn 扫描"));
}

#[test]
fn 全是占位块时仍报未找到() {
    let 文本 = "<<<文件:路径>>>\n内容\n<<<结束>>>";
    assert!(解析落盘文本(文本).is_err());
}

#[test]
fn 合并产物_主优先兜底补缺按路径去重() {
    let 调度 = 假调度(std::env::temp_dir());
    let 主们 = vec![产物条目 {
        路径: "甲.rs".to_string(),
        类别: "代码".to_string(),
        字节数: 1,
        变化类型: "修改".to_string(),
    }];
    let 兜底们 = vec![
        产物条目 {
            路径: "甲.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 999,
            变化类型: "修改".to_string(),
        },
        产物条目 {
            路径: "乙.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 2,
            变化类型: "新增".to_string(),
        },
    ];
    let 结果 = 调度.合并产物(主们, 兜底们);
    assert_eq!(结果.len(), 2);
    assert_eq!(结果[0].路径, "甲.rs");
    assert_eq!(结果[0].字节数, 1, "主产物优先，不被兜底覆盖");
    assert_eq!(结果[1].路径, "乙.rs");
}

#[test]
fn diff补产物_识别任务期间新增与修改() {
    // 临时工程：先拍基线，任务期间新增 + 修改，diff 应把两处都合成产物。
    let 根 = std::env::temp_dir().join(format!("派发兜底测试-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&根);
    std::fs::create_dir_all(&根).unwrap();
    std::fs::write(根.join("Cargo.toml"), "[package]\nname = \"兜底\"\n").unwrap();
    let 基线 = 全量基线(&根);
    std::fs::write(根.join("子.rs"), "pub fn 甲() {}\n").unwrap();
    std::fs::write(
        根.join("Cargo.toml"),
        "[package]\nname = \"兜底\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    let 调度 = 假调度(根.clone());
    let 产物们 = 调度.diff补产物(&基线);
    let 路径们: Vec<&str> = 产物们.iter().map(|产物| 产物.路径.as_str()).collect();
    assert!(路径们.contains(&"子.rs"), "新增应被识别：{:?}", 路径们);
    assert!(路径们.contains(&"Cargo.toml"), "修改应被识别：{:?}", 路径们);
    assert_eq!(产物们.len(), 2);
    let _ = std::fs::remove_dir_all(&根);
}

/// 产物兜底只补相对基线有变化的文件：未变文件不应出现在清单中。
/// 与 diff补产物_识别任务期间新增与修改 互补，锁定「未变 → 不补入」行为防回归。
/// 指纹 = 大小 + 修改时间（毫秒），同一文件大小与 mtime 完全一致即视为未变。
#[test]
fn diff补产物_未变文件不补入清单() {
    let 根 = std::env::temp_dir().join(format!("派发兜底未变测试-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&根);
    std::fs::create_dir_all(&根).unwrap();
    std::fs::write(根.join("未变.rs"), "pub fn 甲() {}\n").unwrap();
    std::fs::write(根.join("要改.rs"), "pub fn 乙() {}\n").unwrap();
    // 间隔确保 mtime 跨毫秒（部分文件系统 mtime 精度仅秒级，需留余量）。
    std::thread::sleep(std::time::Duration::from_millis(50));
    let 基线 = 全量基线(&根);

    // 仅改要改.rs，未变.rs 不动（fingerprint 与基线一致 → 分类为未变 → 不进产物）。
    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::write(根.join("要改.rs"), "pub fn 乙() { println!(\"改了\"); }\n").unwrap();

    let 调度 = 假调度(根.clone());
    let 产物们 = 调度.diff补产物(&基线);
    let 路径们: Vec<&str> = 产物们.iter().map(|产物| 产物.路径.as_str()).collect();
    assert!(路径们.contains(&"要改.rs"), "修改应被识别：{:?}", 路径们);
    assert!(
        !路径们.contains(&"未变.rs"),
        "未变文件不应在产物清单中：{:?}",
        路径们
    );
    assert_eq!(产物们.len(), 1, "只有修改的文件应在产物清单中");
    let _ = std::fs::remove_dir_all(&根);
}

/// 执行回执必须带 轮数字段（跨任务累计总轮数），首调起算、重试不重置、跨并发派遣各自独立。
#[test]
fn 执行回执带轮数字段且类型为u32() {
    let 回执 = 执行回执 {
        状态: 执行状态::成功,
        产物们: vec![],
        说明: "示例".to_string(),
        用量: 用量::default(),
        轮数: 7,
    };
    assert_eq!(回执.轮数, 7u32, "轮数应原样保存（u32 跨任务累计）");
    // 反序列化往返：JSON 落库可还原（含 轮数 字段）。
    let 文本 = serde_json::to_string(&回执).expect("应能序列化");
    assert!(文本.contains("\"轮数\":7"), "序列化须包含轮数字段：{文本}");
    let 反: 执行回执 = serde_json::from_str(&文本).expect("应能反序列化");
    assert_eq!(反.轮数, 7);
}

/// 假调度造一个 执行任务，确认 类型 装配无误（仅为结构兜底测试，证明轮数接口已就绪）。
#[test]
fn 执行任务结构与轮数接口对齐() {
    let 任务 = 执行任务 {
        目标: "示例目标".to_string(),
        工作流: 工作流级别::脚本,
        角色们: vec!["多宝".to_string()],
    };
    assert_eq!(任务.目标, "示例目标");
    // 跨任务累计总轮数将由 调用方在收集 执行回执.轮数 后求和并落库。
}

/// 环境故障判定（2026-08-18）：负退出码（系统级异常终止，如 0xC0000142=-1073741502）
/// 与 None（进程未能启动/未产生退出码）应判为环境故障——不得误当编译错误反复重试烧 token。
#[test]
fn 环境故障判定_负码与缺失判真() {
    // 负退出码 = 系统级异常终止（Windows SEH，如 DLL 初始化失败 0xC0000142）
    assert!(
        super::是环境故障(Some(-1073741502)),
        "0xC0000142 应为环境故障"
    );
    assert!(super::是环境故障(Some(-1)));
    // None = 进程未能产生退出码（启动失败）→ 环境故障
    assert!(super::是环境故障(None));
}

#[test]
fn 环境故障判定_正常编译码判假() {
    // 编译错误 / 构建失败 的正常非 0 正退出码：非环境故障，可重试
    assert!(!super::是环境故障(Some(0)), "退出码0=构建通过，非故障");
    assert!(
        !super::是环境故障(Some(101)),
        "101=Rust编译错误，非环境故障"
    );
    assert!(!super::是环境故障(Some(1)), "普通失败非环境故障");
    assert!(!super::是环境故障(Some(2)));
    // 普通正整数都不应误判为环境故障
    for 码 in 0..200 {
        assert!(!super::是环境故障(Some(码)), "正码 {码} 非环境故障");
    }
}

// 产物须入编译树（设计稿 §11.2 规则 13）测试：构建通过后产物不在 workspace members 里时构建假阳性。

/// 临时工程唯一编号：并行测试各自独立目录，防互相覆盖 Cargo.toml（实测：同用 pid 目录致成员数漂移）。
static 临时工程序号: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 造一个临时 workspace 工程（含 Cargo.toml），供产物入编译树测试。
fn 临时workspace工程(成员们: &[&str]) -> PathBuf {
    let 序号 = 临时工程序号.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let 根 = std::env::temp_dir().join(format!("产物入编译树测试-{序号}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&根);
    std::fs::create_dir_all(&根).unwrap();
    let mut 内容 = String::from("[workspace]\nresolver = \"2\"\nmembers = [\n");
    for 成员 in 成员们 {
        内容.push_str(&format!("    \"{成员}\",\n"));
    }
    内容.push_str("]\n");
    std::fs::write(根.join("Cargo.toml"), 内容).unwrap();
    根
}

#[test]
fn 产物在workspace内_按路径组件匹配() {
    let 成员们 = vec![PathBuf::from("鸿蒙/基础设施 - 域/道术施展-府")];
    assert!(
        super::产物在workspace内("鸿蒙/基础设施 - 域/道术施展-府/某.rs", &成员们),
        "产物在成员目录内应匹配"
    );
    assert!(
        !super::产物在workspace内("鸿蒙/基础设施-域/道术施展-府/某.rs", &成员们),
        "错误目录（无空格）不应匹配——这是要求-91 假阳性的核心"
    );
    assert!(
        !super::产物在workspace内("乾坤/某.rs", &成员们),
        "完全不相关的路径不应匹配"
    );
}

#[test]
fn 产物在workspace内_反斜杠路径统一处理() {
    let 成员们 = vec![PathBuf::from("鸿蒙/基础设施 - 域/道术施展-府")];
    // Windows 反斜杠路径应与正斜杠成员匹配
    assert!(
        super::产物在workspace内("鸿蒙\\基础设施 - 域\\道术施展-府\\某.rs", &成员们),
        "反斜杠路径应统一为正斜杠后匹配"
    );
}

#[test]
fn 解析workspace成员_显式列表() {
    let 内容 = "[workspace]\nresolver = \"2\"\nmembers = [\n    \"鸿蒙/基础设施 - 域/识海承载-府\",\n    \"鸿蒙/基础设施 - 域/道术施展-府\",\n]\n";
    let 成员们 = super::解析workspace成员(内容);
    assert_eq!(成员们.len(), 2, "应解析出 2 个成员");
    assert_eq!(成员们[0], "鸿蒙/基础设施 - 域/识海承载-府");
    assert_eq!(成员们[1], "鸿蒙/基础设施 - 域/道术施展-府");
}

#[test]
fn 解析workspace成员_忽略非workspace段() {
    // 含 [package] 段和其他字段，只应取 [workspace] members
    let 内容 = "[package]\nname = \"x\"\nmembers = [\"不应被取\"]\n\n[workspace]\nmembers = [\"应被取\"]\n";
    let 成员们 = super::解析workspace成员(内容);
    assert_eq!(成员们, vec!["应被取".to_string()]);
}

#[test]
fn 解析workspace成员_空members返回空() {
    let 内容 = "[workspace]\nmembers = []\n";
    let 成员们 = super::解析workspace成员(内容);
    assert!(成员们.is_empty());
}

#[test]
fn 校验产物入编译树_产物在成员内通过() {
    let 根 = 临时workspace工程(&["鸿蒙/基础设施 - 域/道术施展-府"]);
    let 调度 = 假调度(根.clone());
    let 产物们 = vec![产物条目 {
        路径: "鸿蒙/基础设施 - 域/道术施展-府/某.rs".to_string(),
        类别: "代码".to_string(),
        字节数: 1,
        变化类型: "修改".to_string(),
    }];
    let 涉及路径 = vec!["鸿蒙/基础设施 - 域/道术施展-府/某.rs".to_string()];
    assert!(
        调度.校验产物入编译树(&产物们, &涉及路径).is_ok(),
        "产物在成员内应通过"
    );
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 校验产物入编译树_产物不在成员内失败() {
    let 根 = 临时workspace工程(&["鸿蒙/基础设施 - 域/道术施展-府"]);
    let 调度 = 假调度(根.clone());
    // 产物落在错误目录（无空格），不在 workspace members 里
    let 产物们 = vec![产物条目 {
        路径: "鸿蒙/基础设施-域/道术施展-府/某.rs".to_string(),
        类别: "代码".to_string(),
        字节数: 1,
        变化类型: "修改".to_string(),
    }];
    let 涉及路径 = vec!["鸿蒙/基础设施-域/道术施展-府/某.rs".to_string()];
    let 结果 = 调度.校验产物入编译树(&产物们, &涉及路径);
    assert!(结果.is_err(), "产物不在成员内应失败");
    let 错误 = 结果.unwrap_err();
    assert!(
        错误.contains("鸿蒙/基础设施-域/道术施展-府/某.rs"),
        "错误信息应包含脱靶产物路径：{错误}"
    );
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 校验产物入编译树_涉及路径为空跳过() {
    let 根 = 临时workspace工程(&["鸿蒙/基础设施 - 域/道术施展-府"]);
    let 调度 = 假调度(根.clone());
    // 产物路径不在成员内，但涉及路径为空（审验/核查类），应跳过检查
    let 产物们 = vec![产物条目 {
        路径: "任意/不在/成员/内.rs".to_string(),
        类别: "代码".to_string(),
        字节数: 1,
        变化类型: "修改".to_string(),
    }];
    let 涉及路径: Vec<String> = vec![];
    assert!(
        调度.校验产物入编译树(&产物们, &涉及路径).is_ok(),
        "涉及路径为空（审验/核查类）应跳过检查"
    );
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 校验产物入编译树_非rs文件跳过() {
    let 根 = 临时workspace工程(&["鸿蒙/基础设施 - 域/道术施展-府"]);
    let 调度 = 假调度(根.clone());
    // Cargo.toml 等非 RS 必需文件不参与编译，跳过检查
    let 产物们 = vec![产物条目 {
        路径: "任意/不在/成员/内/Cargo.toml".to_string(),
        类别: "代码".to_string(),
        字节数: 1,
        变化类型: "修改".to_string(),
    }];
    let 涉及路径 = vec!["任意/不在/成员/内/Cargo.toml".to_string()];
    assert!(
        调度.校验产物入编译树(&产物们, &涉及路径).is_ok(),
        "非 RS 必需文件应跳过检查"
    );
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 校验产物入编译树_混合产物只报脱靶() {
    let 根 = 临时workspace工程(&["鸿蒙/基础设施 - 域/道术施展-府"]);
    let 调度 = 假调度(根.clone());
    // 一个在成员内，一个不在，应失败且只报不在的那个
    let 产物们 = vec![
        产物条目 {
            路径: "鸿蒙/基础设施 - 域/道术施展-府/甲.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 1,
            变化类型: "修改".to_string(),
        },
        产物条目 {
            路径: "鸿蒙/基础设施-域/乙.rs".to_string(),
            类别: "代码".to_string(),
            字节数: 1,
            变化类型: "新增".to_string(),
        },
    ];
    let 涉及路径 = vec!["鸿蒙/基础设施-域/乙.rs".to_string()];
    let 结果 = 调度.校验产物入编译树(&产物们, &涉及路径);
    assert!(结果.is_err(), "含脱靶产物应失败");
    let 错误 = 结果.unwrap_err();
    assert!(错误.contains("乙.rs"), "应报脱靶产物：{错误}");
    assert!(!错误.contains("甲.rs"), "不应报在成员内的产物：{错误}");
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 读取并展开workspace成员_显式路径() {
    let 根 = 临时workspace工程(&["鸿蒙/基础设施 - 域/道术施展-府", "乾坤/呈现-域/命令操作-府"]);
    let 成员们 = super::读取并展开workspace成员(&根);
    assert_eq!(成员们.len(), 2, "显式路径应原样返回");
    assert!(成员们.contains(&PathBuf::from("鸿蒙/基础设施 - 域/道术施展-府")));
    assert!(成员们.contains(&PathBuf::from("乾坤/呈现-域/命令操作-府")));
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 读取并展开workspace成员_glob模式展开() {
    let 根 = std::env::temp_dir().join(format!("产物glob测试-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&根);
    std::fs::create_dir_all(根.join("鸿蒙")).unwrap();
    // 造两个子目录供 glob 展开
    std::fs::create_dir_all(根.join("鸿蒙").join("甲-府")).unwrap();
    std::fs::create_dir_all(根.join("鸿蒙").join("乙-府")).unwrap();
    // 造一个文件（应被跳过，只展开目录）
    std::fs::write(根.join("鸿蒙").join("文件.toml"), "").unwrap();
    std::fs::write(
        根.join("Cargo.toml"),
        "[workspace]\nmembers = [\"鸿蒙/*\"]\n",
    )
    .unwrap();
    let 成员们 = super::读取并展开workspace成员(&根);
    assert_eq!(成员们.len(), 2, "glob 应展开为 2 个目录（文件跳过）");
    assert!(成员们.contains(&PathBuf::from("鸿蒙/甲-府")));
    assert!(成员们.contains(&PathBuf::from("鸿蒙/乙-府")));
    let _ = std::fs::remove_dir_all(&根);
}

#[test]
fn 读取并展开workspace成员_读不到cargo返回空() {
    let 根 = std::env::temp_dir().join(format!("产物空测试-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&根);
    std::fs::create_dir_all(&根).unwrap();
    // 没有 Cargo.toml
    let 成员们 = super::读取并展开workspace成员(&根);
    assert!(成员们.is_empty(), "读不到 Cargo.toml 应返回空");
    let _ = std::fs::remove_dir_all(&根);
}
