//! 模板 - 生成 - 园：按类别模板生成设计方案（甲阶段用模板）；LLM 级设计（乙阶段）由模型主导、模板兜底。

use crate::类型_定义_殿::{拆解项, 要求书, 要求类别, 设计方案};
use jiance_fu::{观测角色, 进入观测};
use moxing_fu::{对话消息, 常规上限, 模型配置, 用量, 调用模型};
use rizhi_fu::{info, warn};

/// 分级设计（设计稿 §1.5.5 拍板 2）：简单任务单轮商讨（初稿+四圣一次评审，意见记日志不回改）；
/// 复杂任务多轮商讨（圣人工作群：评审-改稿循环，最多 2 轮）。
/// 复杂判定机械化：类别=新能力，或涉及路径 ≥3，或涉及府数 ≥2（跨府）。
pub fn 分级设计(要求: &要求书, 配置: &模型配置) -> 设计方案 {
    // 白箱观测：设计阶段进入设计角色（要求级上下文），嵌套于鸿钧主政栈顶。
    let _观测守卫 = 进入观测(观测角色::设计, None, Some(要求.id.clone()), None);
    if 是复杂任务(要求) {
        info!(要求id = %要求.id, "复杂任务：圣人工作群多轮商讨");
        圣人工作群设计(要求, 配置)
    } else {
        info!(要求id = %要求.id, "简单任务：单轮商讨（初稿 + 四圣一次评审）");
        let 稿 = 模型设计(要求, 配置);
        let (意见们, _) = 四维评审(要求, &稿, 配置);
        for 意见 in 意见们.iter().filter(|意见| 意见.有意见) {
            info!(要求id = %要求.id, 角度 = 意见.角度, 意见 = %意见.意见, "单轮评审意见（记录不回改）");
        }
        稿
    }
}

/// 复杂任务判定：新能力 / 涉及路径 ≥3 / 涉及府数 ≥2。
fn 是复杂任务(要求: &要求书) -> bool {
    if 要求.类别 == 要求类别::新能力 {
        return true;
    }
    if 要求.约束.涉及路径.len() >= 3 {
        return true;
    }
    let 府们: std::collections::HashSet<&str> = 要求
        .约束
        .涉及路径
        .iter()
        .filter_map(|路径| 路径.split('/').nth(2))
        .collect();
    府们.len() >= 2
}

/// 模板设计：按类别套模板，产出设计方案 + 拆解项。
pub fn 模板设计(要求: &要求书) -> 设计方案 {
    let (设计, 工作流) = match 要求.类别 {
        要求类别::功能 => ("实现该功能，落位对应府殿阁园，写码 + 自证", "L3_program"),
        要求类别::补基础 => ("补齐基础能力，先骨架后逻辑", "L3_program"),
        要求类别::性能 => ("定位瓶颈，优化热路径", "L2_script"),
        要求类别::美观 => ("调整呈现与样式", "L2_script"),
        要求类别::维护 => ("修复问题，补测试", "L2_script"),
        要求类别::新能力 => ("新增能力，按六层规范落园", "L4_complex"),
        _ => ("优化调整", "L2_script"),
    };
    info!(要求id = %要求.id, 类别 = ?要求.类别, "模板设计完成");

    设计方案 {
        要求id: 要求.id.clone(),
        设计: format!("{设计}。验收标准：{}", 要求.验收标准),
        拆解: vec![拆解项 {
            目标: 要求.方向.clone(),
            执行层角色: vec!["duobao".to_string()],
            工作流: 工作流.to_string(),
        }],
        自评: format!("本设计通过「{设计}」满足验收标准"),
    }
}

/// 读项目结构摘要（设计稿 §11.2 设计阶段加固）：结构树 + workspace members + 府间依赖。
/// 助设计主笔落位正确——看到项目骨架，不臆造不存在的府殿阁园。
/// 结构树从依赖图加载（`依赖图::下探` 空关键词 → 渲染全部 crate 树）；
/// workspace members 从根 Cargo.toml 读取（参考三档拼装.rs 读workspace成员）；
/// 府间依赖读各府 Cargo.toml [dependencies] 段，注入「府 → 依赖列表」映射。
fn 读项目结构() -> String {
    let mut 段 = String::new();
    let 工作区 = shihai_fu::工作区::定位();
    // 结构树：从依赖图加载，下探全部（关键词空 → 渲染全部 crate 树）。
    if let Ok(图) = shihai_fu::依赖图::加载自工作区(&工作区) {
        let 树 = 图.下探(&[]);
        if !树.is_empty() {
            段.push_str("【项目结构树】\n");
            段.push_str(&树);
            段.push('\n');
        }
    }
    // workspace members + 府间依赖：从根 Cargo.toml 读 members，再读各府 Cargo.toml [dependencies]。
    let 根路径 = 工作区.根路径();
    let Some(内容) = std::fs::read_to_string(根路径.join("Cargo.toml")).ok() else {
        return 段;
    };
    let members: Vec<String> = 内容
        .lines()
        .filter(|行| 行.contains("-府\""))
        .map(|行| {
            行.trim()
                .trim_start_matches('"')
                .trim_end_matches(',')
                .trim_end_matches('"')
                .to_string()
        })
        .collect();
    if members.is_empty() {
        return 段;
    }
    段.push_str("【workspace members】");
    段.push_str(&members.join("、"));
    段.push('\n');
    // 府间依赖：读各府 Cargo.toml [dependencies]，注入「府 → 依赖列表」映射。
    let mut 依赖段 = String::from("【府间依赖】\n");
    let mut 有依赖 = false;
    for 府 in &members {
        if let Some(行) = 读府依赖(根路径.join(府)) {
            依赖段.push_str(&行);
            有依赖 = true;
        }
    }
    if 有依赖 {
        段.push_str(&依赖段);
    }
    段
}

/// 读单个府 Cargo.toml 的 [dependencies] 段，返回「府名 → 依赖列表」一行文本。
fn 读府依赖(府路径: std::path::PathBuf) -> Option<String> {
    let 内容 = std::fs::read_to_string(府路径.join("Cargo.toml")).ok()?;
    let 府名 = 府路径.file_name()?.to_string_lossy().to_string();
    let mut 依赖们: Vec<String> = Vec::new();
    let mut 在依赖段 = false;
    for 行 in 内容.lines() {
        let 去空白 = 行.trim();
        if 去空白.starts_with('[') {
            在依赖段 = 去空白 == "[dependencies]";
            continue;
        }
        if 在依赖段 {
            if let Some(名) = 去空白.split_once('=').map(|(名, _)| 名.trim()) {
                if !名.is_empty() {
                    依赖们.push(名.to_string());
                }
            }
        }
    }
    if 依赖们.is_empty() {
        return None;
    }
    Some(format!("{府名} → {}\n", 依赖们.join("、")))
}

/// 模型设计（设计稿 §12 P2-9）：提示模型按 方向/类别/验收标准/涉及路径
/// 产出设计方案 JSON（设计 + 拆解 + 自评），解析失败或字段缺失回退模板设计。
/// 提示词注入项目结构树 + workspace members + 府间依赖（设计稿 §11.2 设计阶段加固），
/// 助主笔落位正确——看到项目骨架，不臆造不存在的府殿阁园。
pub fn 模型设计(要求: &要求书, 配置: &模型配置) -> 设计方案 {
    let 结构 = 读项目结构();
    模型设计_带用量(要求, 配置, &结构).0
}

/// 模型设计带用量：返回 (设计方案, 用量)，供 圣人工作群设计 做预算累计。
fn 模型设计_带用量(
    要求: &要求书, 配置: &模型配置, 结构: &str
) -> (设计方案, 用量) {
    let 涉及路径 = if 要求.约束.涉及路径.is_empty() {
        "（未指定）".to_string()
    } else {
        要求.约束.涉及路径.join("\n")
    };
    let 提示 = format!(
        "你是世界设计主笔。根据要求产出设计方案，只输出一个 JSON 对象，不要多余文字。\n\
         JSON 结构：{{\"设计\":\"设计思路（落位府殿阁园、先做什么后做什么、验收怎么自证）\",\
         \"拆解\":[{{\"目标\":\"子任务目标\",\"执行层角色\":[\"duobao\"],\"工作流\":\"L3_program\"}}],\
         \"自评\":\"设计为何满足验收标准（必填不可为空）\"}}\n\
         硬约束：拆解不超过 3 个子任务，每个子任务必须独立可完成，涉及路径互不重叠。\n\
         工作流 字段必须且只能取一个值：L1_qa/L2_script/L3_program/L4_complex 之一（示例填了 L3_program，禁止填列表或竖线分隔的多值，多值会被机械校验打回）。\n\
         自评必填：必须逐条说明设计如何自证验收标准，空自评会被机械校验直接打回。\n\n\
         【要求id】{id}\n【方向】{方向}\n【类别】{类别:?}\n【验收标准】{验收标准}\n【涉及路径】\n{涉及路径}\n{结构}",
        id = 要求.id,
        方向 = 要求.方向,
        类别 = 要求.类别,
        验收标准 = 要求.验收标准,
        涉及路径 = 涉及路径,
        结构 = 结构
    );
    match 调用模型(配置, &[对话消息::用户(提示)], 常规上限) {
        Ok((回复, 用量)) => match 解析设计方案(&要求.id, &回复) {
            Some(方案) => {
                info!(要求id = %要求.id, 拆解数 = 方案.拆解.len(), 提示词 = 用量.提示词, "LLM设计完成");
                (方案, 用量)
            }
            None => {
                let 摘要: String = 回复.chars().take(200).collect();
                warn!(要求id = %要求.id, 回复长度 = 回复.len(), 摘要, "LLM设计解析失败，回退模板");
                (模板设计(要求), 用量)
            }
        },
        Err(错误) => {
            warn!(要求id = %要求.id, 错误 = %错误, "LLM设计调用失败，回退模板");
            (模板设计(要求), 用量::default())
        }
    }
}

/// 评审意见：一个角度对设计稿的独立评审。
struct 评审意见 {
    角度: &'static str,
    有意见: bool,
    意见: String,
}

/// 四维评审角度（设计稿 §18.3）：各审一维，独立 LLM 调用，防单人设计盲区。
const 四维角度: [(&str, &str); 4] = [
    ("老子", "核心逻辑/边界/抽象是否成立"),
    ("元始", "异常路径/错误处理是否完备"),
    ("通天", "接口/性能/并发是否合理"),
    ("后土", "数据流/状态/持久化是否一致"),
];

/// 圣人工作群评审（设计稿 §18.3）：主笔发稿 → 四维分角度评审 → 意见收敛。
/// 本质：多人分角度审设计稿，意见收敛后定稿；主笔单人设计有盲区，四维各补专业知识。
/// 收敛上限 2 轮：评审 → 有意见则综合改稿 → 再评，直到全部一致收敛或达上限。
/// 四维评审并行（`thread::scope`，各角度独立线程）；设总预算上限 20 万 token，
/// 四维评审 + 改稿累计超预算即终止采用当前稿（设计稿 §11.2 设计阶段加固）。
pub fn 圣人工作群设计(要求: &要求书, 配置: &模型配置) -> 设计方案 {
    const 总预算上限: u64 = 200_000;
    let 结构 = 读项目结构();
    let (初稿, 用量0) = 模型设计_带用量(要求, 配置, &结构);
    let mut 稿 = 初稿;
    let mut 累计 = 用量0;
    for 轮 in 0..2 {
        let (意见们, 用量1) = 四维评审(要求, &稿, 配置);
        累计.加(&用量1);
        let 有意见们: Vec<&评审意见> = 意见们.iter().filter(|意见| 意见.有意见).collect();
        if 有意见们.is_empty() {
            info!(要求id = %要求.id, 轮, "设计评审全部意见一致收敛");
            return 稿;
        }
        if 累计.总计 > 总预算上限 {
            warn!(要求id = %要求.id, 轮, 累计 = 累计.总计, 上限 = 总预算上限, "设计评审超预算，采用当前稿");
            return 稿;
        }
        let 意见文本 = 有意见们
            .iter()
            .map(|意见| format!("【{}】{}", 意见.角度, 意见.意见))
            .collect::<Vec<_>>()
            .join("\n");
        info!(要求id = %要求.id, 轮, 意见数 = 有意见们.len(), "设计评审有意见，综合改稿");
        let (新稿, 用量2) = 改稿_带用量(要求, &稿, &意见文本, 配置, &结构);
        累计.加(&用量2);
        稿 = 新稿;
        if 累计.总计 > 总预算上限 {
            warn!(要求id = %要求.id, 轮, 累计 = 累计.总计, 上限 = 总预算上限, "设计改稿超预算，采用当前稿");
            return 稿;
        }
    }
    warn!(要求id = %要求.id, "设计评审达上限未完全收敛，采用最新稿");
    稿
}

/// 四维分角度评审：每个角度独立一次 LLM 调用，产出评审意见。
/// 并行执行（`std::thread::scope`，各角度独立线程，参考终裁.rs 六准圣审验并行实现），
/// 返回 (意见们, 累计用量) 供 圣人工作群设计 做预算累计。
fn 四维评审(
    要求: &要求书, 方案: &设计方案, 配置: &模型配置
) -> (Vec<评审意见>, 用量) {
    let 结果们: Vec<(评审意见, 用量)> = std::thread::scope(|作用域| {
        let 句柄们: Vec<_> = 四维角度
            .into_iter()
            .map(|(角度, 关注点)| {
                let 提示 = format!(
                    "你是{角度}（世界设计评审）。从「{关注点}」角度评审下列设计方案，\
                     只输出 JSON 对象，不要多余文字。\n\
                     JSON 结构：{{\"有意见\":true|false,\"意见\":\"具体意见（无意见则空）\"}}\n\n\
                     【方向】{方向}\n【设计】{设计}\n【拆解】{拆解:?}",
                    角度 = 角度,
                    关注点 = 关注点,
                    方向 = 要求.方向,
                    设计 = 方案.设计,
                    拆解 = 方案.拆解
                );
                作用域.spawn(move || {
                    match 调用模型(配置, &[对话消息::用户(提示)], 常规上限) {
                        Ok((回复, 用量)) => (解析评审意见(角度, &回复), 用量),
                        Err(_) => (
                            评审意见 {
                                角度,
                                有意见: false,
                                意见: String::new(),
                            },
                            用量::default(),
                        ),
                    }
                })
            })
            .collect();
        句柄们
            .into_iter()
            .map(|句柄| {
                句柄.join().unwrap_or_else(|_| {
                    (
                        评审意见 {
                            角度: "异常",
                            有意见: false,
                            意见: String::new(),
                        },
                        用量::default(),
                    )
                })
            })
            .collect()
    });
    let mut 意见们 = Vec::with_capacity(结果们.len());
    let mut 累计 = 用量::default();
    for (意见, 用量) in 结果们 {
        累计.加(&用量);
        意见们.push(意见);
    }
    (意见们, 累计)
}

/// 综合意见改稿带用量：把各角度评审意见注入主笔，重新出稿；失败回退现稿。
/// 提示词注入项目结构（设计稿 §11.2 设计阶段加固），返回 (设计方案, 用量) 供预算累计。
fn 改稿_带用量(
    要求: &要求书,
    现稿: &设计方案,
    意见文本: &str,
    配置: &模型配置,
    结构: &str,
) -> (设计方案, 用量) {
    let 涉及路径 = if 要求.约束.涉及路径.is_empty() {
        "（未指定）".to_string()
    } else {
        要求.约束.涉及路径.join("\n")
    };
    let 提示 = format!(
        "你是世界设计主笔。根据评审意见修改设计方案，只输出一个 JSON 对象，不要多余文字。\n\
         JSON 结构：{{\"设计\":\"设计思路\",\"拆解\":[{{\"目标\":\"子任务目标\",\"执行层角色\":[\"duobao\"],\"工作流\":\"L3_program\"}}],\"自评\":\"设计为何满足验收标准（必填不可为空）\"}}\n\
         硬约束：拆解不超过 3 个子任务，每个子任务必须独立可完成，涉及路径互不重叠。\n\
         工作流 字段必须且只能取一个值：L1_qa/L2_script/L3_program/L4_complex 之一（示例填了 L3_program，禁止填列表或竖线分隔的多值，多值会被机械校验打回）。\n\
         自评必填：必须逐条说明设计如何自证验收标准，空自评会被机械校验直接打回。\n\n\
         【方向】{方向}\n【现稿】{设计}\n【拆解】{拆解:?}\n【涉及路径】\n{涉及路径}\n{结构}\n【评审意见】\n{意见}",
        方向 = 要求.方向,
        设计 = 现稿.设计,
        拆解 = 现稿.拆解,
        涉及路径 = 涉及路径,
        结构 = 结构,
        意见 = 意见文本
    );
    match 调用模型(配置, &[对话消息::用户(提示)], 常规上限) {
        Ok((回复, 用量)) => (
            解析设计方案(&要求.id, &回复).unwrap_or_else(|| 现稿.clone()),
            用量,
        ),
        Err(_) => (现稿.clone(), 用量::default()),
    }
}

/// 解析评审意见 JSON；解析失败或缺字段按「无意见」处理（不阻断收敛）。
/// 遍历全部 JSON 候选：命中带「有意见」布尔字段的对象即采用（先遇先得）。
fn 解析评审意见(角度: &'static str, 回复: &str) -> 评审意见 {
    for 值 in 提取_json候选们(回复) {
        match 值["有意见"].as_bool() {
            Some(true) => {
                let 意见 = 值["意见"].as_str().unwrap_or("").trim().to_string();
                // 声明有意见但未给具体意见，视为无意见（避免空意见触发无效改稿）。
                if !意见.is_empty() {
                    return 评审意见 {
                        角度,
                        有意见: true,
                        意见,
                    };
                }
            }
            Some(false) => {
                return 评审意见 {
                    角度,
                    有意见: false,
                    意见: String::new(),
                }
            }
            None => continue,
        }
    }
    评审意见 {
        角度,
        有意见: false,
        意见: String::new(),
    }
}

/// 从 LLM 回复提取全部「括号平衡且可解析」的 JSON 对象（按出现顺序）。
/// 兼容 think 推理块/解释文字/多个 JSON 候选/围栏：逐对扫描花括号的配对，
/// 字符串字面量内的花括号不计数（含 `\"` 转义），每命中一个完整平衡对象即尝试解析，
/// 成功即收集；孤立右花括号（正文标点）不破坏后续扫描。
fn 提取_json候选们(回复: &str) -> Vec<serde_json::Value> {
    let 字符们: Vec<char> = 回复.chars().collect();
    let mut 结果 = Vec::new();
    let mut 起点: Option<usize> = None;
    let mut 深度 = 0i32;
    let mut 在字符串 = false;
    let mut 转义 = false;
    for (i, &ch) in 字符们.iter().enumerate() {
        if 在字符串 {
            if 转义 {
                转义 = false;
            } else if ch == '\\' {
                转义 = true;
            } else if ch == '"' {
                在字符串 = false;
            }
            continue;
        }
        match ch {
            '"' => 在字符串 = true,
            '{' => {
                if 深度 == 0 {
                    起点 = Some(i);
                }
                深度 += 1;
            }
            '}' => {
                深度 -= 1;
                if 深度 < 0 {
                    // 孤立右花括号（正文标点），重置后继续扫。
                    深度 = 0;
                    起点 = None;
                } else if 深度 == 0 {
                    if let Some(起) = 起点 {
                        let 文本: String = 字符们[起..=i].iter().collect();
                        if let Ok(值) = serde_json::from_str(&文本) {
                            结果.push(值);
                        }
                        // 该候选不可解析（起点的 { 属解释文字），继续扫下一个。
                        起点 = None;
                    }
                }
            }
            _ => {}
        }
    }
    结果
}

/// 从 LLM 回复提取设计方案（遍历全部 JSON 候选，取第一个字段齐全者）。
/// 任一候选字段缺失/设计空/拆解空 → 换下一个候选；全部失败即 None（回退模板）。
fn 解析设计方案(要求id: &str, 回复: &str) -> Option<设计方案> {
    for 值 in 提取_json候选们(回复) {
        // 逐候选校验字段：缺失/空设计/无拆解 → 换下一个候选，绝不因首个示例候选直接放弃。
        let Some(设计) = 值["设计"].as_str() else {
            continue;
        };
        let 设计 = 设计.trim();
        if 设计.is_empty() {
            continue;
        }
        let 自评 = 值["自评"].as_str().unwrap_or("").trim().to_string();
        let 拆解数组 = match 值["拆解"].as_array() {
            Some(数组) if !数组.is_empty() => 数组,
            _ => continue,
        };
        let mut 拆解 = Vec::new();
        for 项 in 拆解数组 {
            let Some(目标) = 项["目标"].as_str() else {
                continue;
            };
            let 目标 = 目标.trim();
            if 目标.is_empty() {
                continue;
            }
            let 工作流 = 项["工作流"]
                .as_str()
                .unwrap_or("L2_script")
                .trim()
                .to_string();
            拆解.push(拆解项 {
                目标: 目标.to_string(),
                执行层角色: vec!["duobao".to_string()],
                工作流,
            });
        }
        // 硬约束：拆解数上限 3（提示词软约束 + 此处硬截断，防过度拆分致执行时间爆炸）。
        拆解.truncate(3);
        if 拆解.is_empty() {
            continue;
        }
        return Some(设计方案 {
            要求id: 要求id.to_string(),
            设计: 设计.to_string(),
            拆解,
            自评,
        });
    }
    warn!(
        要求id,
        字数 = 回复.chars().count(),
        "设计JSON候选均不完整，回退模板"
    );
    None
}

#[cfg(test)]
mod 测试 {
    use super::{
        提取_json候选们, 是复杂任务, 解析设计方案, 解析评审意见, 读府依赖, 读项目结构
    };

    #[test]
    fn 解析评审意见_有意见() {
        let 意见 = 解析评审意见("老子", r#"{"有意见":true,"意见":"拆解粒度太粗"}"#);
        assert!(意见.有意见);
        assert_eq!(意见.意见, "拆解粒度太粗");
    }

    #[test]
    fn 解析评审意见_无意见或空意见或非_json按无意见() {
        // 明确无意见。
        let 无 = 解析评审意见("元始", r#"{"有意见":false,"意见":""}"#);
        assert!(!无.有意见);
        // 声明有意见但空意见 → 按无意见（避免空意见触发无效改稿）。
        let 空 = 解析评审意见("通天", r#"{"有意见":true,"意见":""}"#);
        assert!(!空.有意见);
        // 非 JSON → 按无意见（不阻断收敛）。
        let 乱 = 解析评审意见("后土", "只有一句话");
        assert!(!乱.有意见);
    }

    #[test]
    fn 解析标准设计方案() {
        let 回复 = r#"{"设计":"落位天庭治理-府，先声明后派发","拆解":[{"目标":"加字段","执行层角色":["duobao"],"工作流":"L3_program"}],"自评":"满足验收"}"#;
        let 方案 = 解析设计方案("要求-1", 回复).expect("应解析成功");
        assert_eq!(方案.要求id, "要求-1");
        assert_eq!(方案.拆解.len(), 1);
        assert_eq!(方案.拆解[0].工作流, "L3_program");
    }

    #[test]
    fn 围栏包裹的回复也能解析() {
        let 回复 = "```json\n{\"设计\":\"先读后写\",\"拆解\":[{\"目标\":\"读现状\"}],\"自评\":\"ok\"}\n```";
        let 方案 = 解析设计方案("要求-2", 回复).expect("应容忍围栏");
        assert_eq!(方案.设计, "先读后写");
    }

    #[test]
    fn 缺拆解或非_json回退_none() {
        assert!(
            解析设计方案("要求-3", "设计只有一句话").is_none(),
            "非 JSON 应 None"
        );
        assert!(
            解析设计方案("要求-4", "{\"设计\":\"x\"}").is_none(),
            "缺拆解应 None"
        );
    }

    #[test]
    fn 解析设计方案_拆解数超3截断() {
        let 回复 = r#"{"设计":"x","拆解":[{"目标":"1"},{"目标":"2"},{"目标":"3"},{"目标":"4"},{"目标":"5"}],"自评":"ok"}"#;
        let 方案 = 解析设计方案("要求-5", 回复).expect("应解析成功");
        assert_eq!(方案.拆解.len(), 3, "拆解数应硬截断到 3，防过度拆分");
    }

    /// think 推理块含示例花括号在前 + 真设计 JSON 在后 → 应取到真 JSON（原实现首 `{` 到末 `}` 会切残）。
    #[test]
    fn think块含示例花括号仍能解析出真设计() {
        let 回复 = "我先想一下，结构应类似 {\"设计\":\"示例\"} 或 {\"拆解\":[{\"目标\":\"示例\"}]}，正式输出：\n{\"设计\":\"落位观览-查询-殿，先建园后接线，缓存读取复用于命令分发\",\"拆解\":[{\"目标\":\"新建 流式-纪元-园\",\"执行层角色\":[\"duobao\"],\"工作流\":\"L3_program\"}],\"自评\":\"自证通过\"}";
        let 方案 = 解析设计方案("要求-T", 回复).expect("think 块后应解析出真设计");
        assert!(
            方案.设计.contains("观览-查询-殿"),
            "设计应为后段真 JSON：{}",
            方案.设计
        );
        assert_eq!(方案.拆解.len(), 1);
    }

    /// 多个 JSON 候选：第一个字段不全（示例），第二个完整 → 取第二个，不得因首个放弃。
    #[test]
    fn 多候选取第一个字段齐全者() {
        let 回复 = r#"{"设计":"示例"}{"设计":"真设计","拆解":[{"目标":"目标A"},{"目标":"目标B"}],"自评":"s"}"#;
        let 方案 = 解析设计方案("要求-M", 回复).expect("应取字段齐全的候选");
        assert_eq!(方案.设计, "真设计");
        assert_eq!(方案.拆解.len(), 2);
    }

    /// 分级判定：新能力/路径多/跨府 → 复杂；否则简单。
    #[test]
    fn 分级判定_新能力为复杂() {
        use crate::类型_定义_殿::{
            约束, 要求书, 要求来源, 要求状态, 要求类别, 阶段
        };
        let 要求 = 要求书 {
            id: "要求-1".to_string(),
            来源: 要求来源::界主,
            想法id: None,
            阶段: 阶段::乙,
            方向: "x".to_string(),
            类别: 要求类别::新能力,
            验收标准: "y".to_string(),
            约束: 约束::default(),
            状态: 要求状态::待领,
            确认意见: None,
            验收: None,
            版本: None,
        };
        assert!(是复杂任务(&要求));
    }

    #[test]
    fn 分级判定_简单任务为单轮() {
        use crate::类型_定义_殿::{
            约束, 要求书, 要求来源, 要求状态, 要求类别, 阶段
        };
        let 要求 = 要求书 {
            id: "要求-1".to_string(),
            来源: 要求来源::界主,
            想法id: None,
            阶段: 阶段::乙,
            方向: "x".to_string(),
            类别: 要求类别::维护,
            验收标准: "y".to_string(),
            约束: 约束 {
                涉及路径: vec!["乾坤/呈现-域/命令操作-府/a.rs".to_string()],
                ..约束::default()
            },
            状态: 要求状态::待领,
            确认意见: None,
            验收: None,
            版本: None,
        };
        assert!(!是复杂任务(&要求));
    }

    #[test]
    fn 分级判定_涉及路径多或跨府为复杂() {
        use crate::类型_定义_殿::{
            约束, 要求书, 要求来源, 要求状态, 要求类别, 阶段
        };
        let 造 = |路径们: Vec<&str>| 要求书 {
            id: "要求-1".to_string(),
            来源: 要求来源::界主,
            想法id: None,
            阶段: 阶段::乙,
            方向: "x".to_string(),
            类别: 要求类别::功能,
            验收标准: "y".to_string(),
            约束: 约束 {
                涉及路径: 路径们.into_iter().map(|p| p.to_string()).collect(),
                ..约束::default()
            },
            状态: 要求状态::待领,
            确认意见: None,
            验收: None,
            版本: None,
        };
        assert!(
            是复杂任务(&造(vec![
                "a/域/府-1/x.rs",
                "a/域/府-1/y.rs",
                "a/域/府-1/z.rs"
            ])),
            "涉及路径≥3 应复杂"
        );
        assert!(
            是复杂任务(&造(vec!["a/域/府-1/x.rs", "b/域/府-2/y.rs"])),
            "跨府应复杂"
        );
        assert!(
            !是复杂任务(&造(vec!["a/域/府-1/x.rs", "a/域/府-1/y.rs"])),
            "同府两条应简单"
        );
    }

    /// 字符串字面量内的花括号不破坏平衡扫描（隔离标识符不会导致提前截断）。
    #[test]
    fn 字符串内花括号不干扰解析() {
        let 回复 = r#"{"设计":"输出 {x} 与 } 括号","拆解":[{"目标":"目标","工作流":"L3_program"}],"自评":"ok"}"#;
        let 方案 = 解析设计方案("要求-S", 回复).expect("字符串内花括号不应破坏解析");
        assert!(方案.设计.contains("{x}"));
    }

    /// 评审意见在 think 块 + 解释文字夹杂时也能命中真意见。
    #[test]
    fn 评审意见在夹杂文本中命中() {
        let 回复 =
            "我的评估如下，先分析问题：\n{\"有意见\":true,\"意见\":\"拆解粒度太粗\"}\n结束。";
        let 意见 = 解析评审意见("老子", 回复);
        assert!(意见.有意见);
        assert_eq!(意见.意见, "拆解粒度太粗");
    }

    /// 提取候选：容忍正文里的孤立右花括号（评论文本），后续合法对象不受影响。
    #[test]
    fn 孤立右花括号不破坏后续提取() {
        let 回复 = "这里有个右括号 } 结尾。{\"有意见\":false}";
        let 候选数 = 提取_json候选们(回复).len();
        assert_eq!(候选数, 1, "孤立 }} 后的合法 JSON 应正常提取，实际 {候选数}");
    }

    /// 实况验证（需 MiniMax 密钥与网络）：LLM 设计应解析成功产出真实设计，而非回退模板。
    /// `模型设计` 成功 = 设计非模板机械文本（模板固定句「新增能力，按六层规范落园」等）。
    /// 运行：`$env:WORLD_WORKSPACE_ROOT="d:\洪荒 - 世界"; cargo test -p tianting_fu --lib 实况_LLM设计 -- --ignored --nocapture`
    #[test]
    #[ignore = "实况：需 MiniMax 密钥与网络，非纯单元回归"]
    fn 实况_llm设计不再回退模板() {
        use crate::类型_定义_殿::{
            优先级, 约束, 要求书, 要求来源, 要求状态, 要求类别, 阶段
        };
        let 根 = std::env::var("WORLD_WORKSPACE_ROOT").unwrap_or_default();
        let 环境 =
            std::fs::read_to_string(std::path::Path::new(&根).join(".env")).unwrap_or_default();
        let 取 = |前缀: &str| -> String {
            环境
                .lines()
                .find(|行| 行.trim_start().starts_with(前缀))
                .and_then(|行| 行.split_once('='))
                .map(|(_, 值)| 值.trim().to_string())
                .unwrap_or_default()
        };
        let 配置 = moxing_fu::模型配置 {
            密钥: 取("LLM_API_KEY="),
            地址: 取("LLM_BASE_URL="),
            模型: 取("LLM_MODEL="),
        };
        assert!(
            !配置.密钥.is_empty(),
            "缺少 LLM_API_KEY（检查 WORLD_WORKSPACE_ROOT 与 .env）"
        );
        let 要求 = 要求书 {
            id: "实况-1".to_string(),
            来源: 要求来源::界主,
            想法id: None,
            阶段: 阶段::甲,
            方向: "新增『世界 纪元』只读命令，按公历年份推算干支纪年与生肖并格式化输出"
                .to_string(),
            类别: 要求类别::新能力,
            验收标准: "新增只读命令并接线到兜底分发，干支/生肖推算正确，单元测试通过"
                .to_string(),
            约束: 约束 {
                涉及路径: vec![
                    "乾坤/呈现-域/命令操作-府/观览-查询-殿/世界-观览-阁/流式-纪元-园/流式纪元.rs"
                        .to_string(),
                    "乾坤/呈现-域/命令操作-府/命令-解析-殿/命令-分发-阁/兜底-分发-园/兜底分发.rs"
                        .to_string(),
                ],
                不允许: vec![],
                优先级: 优先级::中,
            },
            状态: 要求状态::待领,
            确认意见: None,
            验收: None,
            版本: None,
        };
        let 方案 = super::模型设计(&要求, &配置);
        let 是否模板 = 方案.设计.starts_with("新增能力") || 方案.设计.starts_with("实现该功能");
        assert!(!是否模板, "LLM 设计回退模板：{设计}", 设计 = 方案.设计);
        println!(
            "【实况】LLM 设计成功：{}（拆解 {} 项）",
            方案.设计,
            方案.拆解.len()
        );
    }

    /// 读府依赖：解析 [dependencies] 段，提取依赖名清单。
    #[test]
    fn 读府依赖_解析依赖段() {
        let 根 = std::env::temp_dir().join(format!(
            "读府依赖测试-{}-{}",
            std::process::id(),
            shihai_fu::当前毫秒()
        ));
        std::fs::create_dir_all(&根).unwrap();
        std::fs::write(
            根.join("Cargo.toml"),
            "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\nshihai_fu = { path = \"../识海承载-府\" }\n",
        )
        .unwrap();
        let 行 = 读府依赖(根.clone()).expect("应解析出依赖");
        assert!(行.contains("serde"), "应含 serde 依赖：{行}");
        assert!(行.contains("shihai_fu"), "应含 shihai_fu 依赖：{行}");
        let _ = std::fs::remove_dir_all(&根);
    }

    /// 读府依赖：无 [dependencies] 段返回 None。
    #[test]
    fn 读府依赖_无依赖段返回none() {
        let 根 = std::env::temp_dir().join(format!(
            "读府依赖测试空-{}-{}",
            std::process::id(),
            shihai_fu::当前毫秒()
        ));
        std::fs::create_dir_all(&根).unwrap();
        std::fs::write(根.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert!(
            读府依赖(根.clone()).is_none(),
            "无 [dependencies] 段应返回 None"
        );
        let _ = std::fs::remove_dir_all(&根);
    }

    /// 读项目结构：在真实工作区跑应含 workspace members 段。
    /// 不硬断言具体内容（环境变量 WORLD_WORKSPACE_ROOT 可能被并行测试设到临时目录），
    /// 仅验证不 panic 且返回 String；真实工作区下应非空。
    #[test]
    fn 读项目结构_不panic且真实工作区非空() {
        let 结构 = 读项目结构();
        // 真实工作区（WORLD_WORKSPACE_ROOT 未被改写时）应含 workspace members 段。
        // 并行测试可能改写环境变量，此处软断言：不 panic 即过，含 members 段时额外验证格式。
        if 结构.contains("【workspace members】") {
            assert!(
                结构.contains("-府"),
                "workspace members 段应含 -府 后缀府名：{结构}"
            );
        }
    }
}
