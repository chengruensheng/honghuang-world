use hm_execute_contract::执行器;

/// 机器核验默认超时（秒）：编译与全量测试显著长于普通工具命令，
/// 独立于执行器默认命令超时（30 秒），否则必然被误判为超时失败。
pub(crate) const 核验默认超时秒: u64 = 900;
/// 核验原始输出留痕上限（字符）：够定位真因即可，避免撑大任务证据链
const 留痕上限: usize = 1200;
/// 失败摘要中保留的错误行上限
const 摘要行上限: usize = 12;
/// 构建清单文件名（数据契约明文名，提为常量以过门禁检测 6）
const 清单文件名: &str = "Cargo.toml";
/// 递归查找构建清单的 glob 模式
const 清单匹配模式: &str = "**/Cargo.toml";

/// 机器核验结论：**由系统真实执行编译与测试得出**，不采信模型自述。
///
/// 存在意义：五层协作中「验收通过」原由模型在 JSON 里自报、系统不独立复核，
/// 构成「自证闭环」（生成者同时是验证者）。本结论把判定权交还机器：退出码与
/// `test result: ok.` 行是唯一依据，缺证据即判不通过（fail-closed）。
#[derive(Debug, Clone)]
pub(crate) struct 核验结论 {
    pub(crate) 通过: bool,
    /// 结论摘要：含真实证据要点（失败时为首批错误行），直接作为打回实现层的修正依据
    pub(crate) 摘要: String,
    /// 原始输出留痕（截断后）：编译与测试的真实 stdout/stderr
    pub(crate) 原始输出: String,
}

impl 核验结论 {
    fn 不通过(摘要: String, 原始输出: String) -> Self {
        核验结论 { 通过: false, 摘要: 截断(&摘要, 留痕上限), 原始输出: 截断(&原始输出, 留痕上限) }
    }

    fn 通过(摘要: String, 原始输出: String) -> Self {
        核验结论 { 通过: true, 摘要, 原始输出: 截断(&原始输出, 留痕上限) }
    }
}

/// 机器核验门：在模型宣告「验收通过 / 交付通过」之前，真实执行编译与测试，
/// 以**退出码** + **`test result: ok.` 行**判定，模型自述一律不作为依据。
///
/// 判定规则（全部为硬证据，无宽免）：
/// 1. 编译必须成功（执行器对非零退出码返回 Err）；
/// 2. 测试必须成功且**确有测试运行**——输出须出现 `test result: ok. N passed`，且不含失败标记；
///    两者皆无（未跑测试 / 输出被截断）按不通过处理（fail-closed）。
pub(crate) fn 机器核验(执行器: &dyn 执行器, 超时秒: u64) -> 核验结论 {
    let 目标 = 定位构建目标(执行器);
    let mut 全部输出 = String::new();

    let (编译输出, 编译错误) = 跑(执行器, "build", &目标, 超时秒);
    全部输出.push_str(&编译输出);
    if let Some(错误) = 编译错误 {
        return 核验结论::不通过(
            format!("机器核验未通过：编译失败（非零退出）。{}", 错误要点(&错误)),
            format!("{全部输出}\n{错误}"),
        );
    }

    let (测试输出, 测试错误) = 跑(执行器, "test", &目标, 超时秒);
    全部输出.push_str(&测试输出);
    if let Some(错误) = 测试错误 {
        return 核验结论::不通过(
            format!("机器核验未通过：测试失败（非零退出）。{}", 错误要点(&错误)),
            format!("{全部输出}\n{错误}"),
        );
    }

    let 通过数 = 统计通过数(&测试输出);
    let 有失败标记 = 测试输出.contains("test result: FAILED")
        || 测试输出.contains("test failed")
        || 测试输出.contains("panicked at");
    if 通过数 == 0 {
        return 核验结论::不通过(
            format!(
                "机器核验未通过：测试未产出可采信的通过证据（输出中无 `test result: ok.` 行，或输出被截断）。{}",
                错误要点(&测试输出)
            ),
            全部输出,
        );
    }
    if 有失败标记 {
        return 核验结论::不通过(
            format!("机器核验未通过：测试存在失败标记。{}", 错误要点(&测试输出)),
            全部输出,
        );
    }

    核验结论::通过(
        format!("机器核验通过：编译成功；测试通过 {通过数} 例、0 失败（系统实跑，非模型自述）。"),
        全部输出,
    )
}

/// 构建目标形态：决定 cargo 命令如何指定作用域
#[derive(Debug, Clone, PartialEq, Eq)]
enum 构建目标 {
    /// 工作区根即含清单：用 `--workspace` 覆盖全部成员
    工作区,
    /// 清单在子目录（产物为独立 crate）：用 `--manifest-path` 指定
    指定清单(String),
    /// 未找到任何清单：无从核验，直接判不通过
    缺失,
}

/// 定位构建目标：优先「工作区根有清单」，否则递归找最浅层的 Cargo.toml 作为产物清单。
///
/// 为什么不能只用 `--workspace`：自主开发的产物常直接落在工作区子目录（如
/// `工作区/数列演算-域/斐波那契-府/Cargo.toml`），工作区根并无清单，
/// 此时 `cargo build --workspace` 只会报「找不到清单」，把合法产物误判为编译失败。
fn 定位构建目标(执行器: &dyn 执行器) -> 构建目标 {
    if 有清单(执行器, 清单文件名) {
        return 构建目标::工作区;
    }
    match 执行器.按名找文件(清单匹配模式) {
        Ok(输出) => match 最浅清单(&输出) {
            Some(路径) => 构建目标::指定清单(路径),
            None => 构建目标::缺失,
        },
        Err(_) => 构建目标::缺失,
    }
}

fn 有清单(执行器: &dyn 执行器, 路径: &str) -> bool {
    matches!(执行器.读文件(路径), Ok(内容) if !内容.trim().is_empty())
}

/// 从 `按名找文件` 输出（逐行相对路径）中取层级最浅的清单；
/// 排除构建产物与回收站内的清单，避免核验到依赖缓存而非产物本身。
fn 最浅清单(输出: &str) -> Option<String> {
    let mut 候选: Vec<(usize, String)> = Vec::new();
    for 行 in 输出.lines() {
        let 行 = 行.trim();
        if 行.is_empty() || 行 == "（无匹配）" {
            continue;
        }
        let 键 = 行.replace('\\', "/");
        if 键.split('/').any(|段| 段 == "target" || 段 == ".git" || 段 == ".回收站") {
            continue;
        }
        if !键.eq_ignore_ascii_case(清单文件名) && !键.ends_with(&format!("/{清单文件名}")) {
            continue;
        }
        候选.push((键.matches('/').count(), 键));
    }
    候选.sort();
    候选.first().map(|(_, 路径)| 路径.clone())
}

/// 执行一条 cargo 子命令，返回（输出, 失败原因）。
///
/// 参数严格落在白名单内（cargo + build/test + --workspace/--manifest-path），
/// 路径一律用相对路径并加引号，避免含空格路径被拆成多个参数。
fn 跑(执行器: &dyn 执行器, 子命令: &str, 目标: &构建目标, 超时秒: u64) -> (String, Option<String>) {
    match 目标 {
        构建目标::缺失 => (
            String::new(),
            Some("工作区内未找到任何 Cargo.toml 清单，无法进行机器核验".to_string()),
        ),
        _ => {
            let 命令 = 组装命令(子命令, 目标);
            match 执行器.运行命令_限时(&命令, 超时秒) {
                Ok(输出) => (输出, None),
                Err(错误) => (String::new(), Some(错误.to_string())),
            }
        }
    }
}

fn 组装命令(子命令: &str, 目标: &构建目标) -> String {
    // `--quiet`：抑制逐条用例与进度输出，把「test result: ok. N passed」结论行留在
    // 执行器输出上限之内。否则大工作区的测试输出会把尾部结论行截断，
    // 核验门只能看到头部而误判为「无通过证据」（缺陷 11-2 / 12-4 的同一根因）。
    match 目标 {
        构建目标::工作区 => format!("cargo {子命令} --workspace --quiet"),
        构建目标::指定清单(路径) => format!("cargo {子命令} --manifest-path \"{路径}\" --quiet"),
        构建目标::缺失 => String::new(),
    }
}

/// 汇总 `test result: ok. N passed` 的通过例数（多测试二进制逐行累加）
fn 统计通过数(输出: &str) -> usize {
    let mut 合计 = 0usize;
    for 行 in 输出.lines() {
        let 行 = 行.trim();
        if let Some(位置) = 行.find("test result: ok.") {
            let 余 = &行[位置 + "test result: ok.".len()..];
            if let Some(数) = 余.split_whitespace().next().and_then(|s| s.parse::<usize>().ok()) {
                合计 += 数;
            }
        }
    }
    合计
}

/// 从原始输出抽取错误要点行（error/FAILED/panic/退出码），供摘要携带真实证据
fn 错误要点(输出: &str) -> String {
    let mut 要点: Vec<&str> = Vec::new();
    for 行 in 输出.lines() {
        let 行 = 行.trim();
        if 行.is_empty() {
            continue;
        }
        if 行.contains("error[")
            || 行.starts_with("error")
            || 行.contains("test result: FAILED")
            || 行.contains("panicked at")
            || 行.contains("命令失败")
            || 行.contains("命令不在白名单")
            || 行.contains("命令超时")
        {
            要点.push(行);
            if 要点.len() >= 摘要行上限 {
                break;
            }
        }
    }
    if 要点.is_empty() {
        String::new()
    } else {
        format!("真实错误：{}", 要点.join(" / "))
    }
}

fn 截断(文本: &str, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        文本.to_string()
    } else {
        let 头部: String = 文本.chars().take(上限).collect();
        format!("{头部}…")
    }
}
