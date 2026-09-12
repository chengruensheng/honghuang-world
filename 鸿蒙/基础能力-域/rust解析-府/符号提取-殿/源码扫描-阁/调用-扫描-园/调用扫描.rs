//! 调用扫描：用 rust-analyzer 语义层提取函数级「调用」边（设计 §七 步 5）
//!
//! 与 `crate-扫描-园` 的分工：那边看 **crate → crate** 的清单依赖（读 Cargo 清单，
//! 零解析成本）；本园看 **函数 → 函数** 的实际调用（需载入语义库，成本高）。
//! 两者都落在既有边词表内，未新增边类型。
//!
//! 为什么不引 `ra_ap_ide`：本园只问"这个表达式解析成哪个函数"，
//! `ra_ap_hir::Semantics` 已经够用，少一层依赖、少一份编译时间。

use hm_symext::{边, 边种类, 符号, 符号种类, 置信度, 插件诊断, 位置};
use ra_ap_hir::{CallableKind, Function, HasSource, Semantics};
use ra_ap_ide_db::RootDatabase;
use ra_ap_load_cargo::{load_workspace_at, LoadCargoConfig, ProcMacroServerChoice};
use ra_ap_project_model::CargoConfig;
use ra_ap_syntax::ast;
use ra_ap_syntax::{AstNode, SyntaxNode};
use ra_ap_vfs::{FileId, Vfs};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::rc::Rc;

/// 「文件 → 行 → 符号 ID」索引，用于把语义层的定义位置桥接回语法层的符号 ID
type 函数索引表 = BTreeMap<String, BTreeMap<usize, String>>;

/// 载入工作区，产出函数级 `调用` 边（置信度 **高**）
///
/// `已有符号` 是语法层已提取的全部符号；本园只借用其中的 `函数` 符号做位置桥接，
/// **不新增符号**——符号的判定权仍归语法层，避免两套提取器互相打架。
pub fn 扫描调用(
    项目根: &Path,
    已有符号: &[符号],
) -> Result<(Vec<边>, Vec<插件诊断>), String> {
    let 索引 = 函数位置索引(已有符号);
    if 索引.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let 函数名表 = 仓库函数名(已有符号);

    let 载入配置 = LoadCargoConfig {
        // 不跑 build script：本园不做类型推断之外的用途，省一次 cargo check
        load_out_dirs_from_check: false,
        with_proc_macro_server: ProcMacroServerChoice::None,
        prefill_caches: false,
        num_worker_threads: 0,
        proc_macro_processes: 0,
    };

    // 载入配置：`no_deps = false`（关键修正）。此前用 `true` 想省去外部依赖的 VFS 装载，
    // 但实测发现 rust-analyzer 在 no_deps 模式下连 **workspace 内部依赖边**也不建
    // （探针：hm_http 的依赖只剩 5 个 sysroot crate，hm_agent/tc_task 等全丢），导致
    // 跨 crate 的 `use hm_agent::…` 解析不出、类型推断连锁失效、方法调用大面积漏收。
    // 代价是外部依赖（axum/tokio…）也会进 crate 图与 VFS，载入变慢；但「仓库内→仓库内」
    // 的桥接只看两端是否落在仓库内，外部依赖的调用仍会因目标在仓库外而被跳过，不损语义。
    let 载入工作区配置 = CargoConfig {
        no_deps: false,
        sysroot: Some(ra_ap_project_model::RustLibSource::Discover),
        // 开启 `cfg(test)`：否则 `#[cfg(test)] mod tests` 里的函数（临时路径/造任务等测试辅助）
        // 在语义层被当作「未激活」，语法层却完整遍历到调用点，造成「取不到可调用体」大面积累加。
        set_test: true,
        ..CargoConfig::default()
    };
    let (db, vfs, _进程宏) =
        load_workspace_at(项目根, &载入工作区配置, &载入配置, &|_| {})
            .map_err(|e| format!("rust-analyzer 载入工作区失败：{e}"))?;

    let mut 边们: Vec<边> = Vec::new();
    let mut 已扫文件 = 0usize;
    let mut 未匹配 = 0usize;
    let mut 计数 = 解析计数::default();

    // 新版解算器的类型驻留表是 **thread-local** 的：出了 `attach_db` 作用域就解绑，
    // 再问类型就 panic "Try to use attached db, but not db is attached"。
    // 因此凡是要做推断的调用（解析调用点、查被调函数定义）都必须在这个作用域内跑。
    ra_ap_hir::attach_db(&db, || {
        let 语义 = Semantics::new(&db);
        let mut 源码缓存: HashMap<FileId, Rc<String>> = HashMap::new();
        let mut 已建: BTreeSet<(String, String)> = BTreeSet::new();

        for (文件id, 虚拟路径) in vfs.iter() {
            let Some(绝对) = 虚拟路径.as_path() else {
                continue;
            };
            let Some(相对) = 仓库相对(项目根, 绝对.as_str()) else {
                continue;
            };
            if !相对.ends_with(".rs") || !索引.contains_key(&相对) {
                continue;
            }
            已扫文件 += 1;

            let 源文件 = 语义.parse_guess_edition(文件id);
            let 文本 = 源码(&vfs, 文件id, &mut 源码缓存);

            for 节点 in 源文件.syntax().descendants() {
                let Some((宿主id, 宿主形态)) = 宿主符号(&节点, &相对, &文本, &索引) else {
                    // 宿主未定位：调用点无法归属到任何函数，边必然丢失。但绝大多数语法节点
                    // 本就不在函数内（use/struct/const），必须先用「是不是调用表达式」过滤，
                    // 否则这一项会被海量非调用节点淹没而失去诊断价值。
                    if ast::MethodCallExpr::can_cast(节点.kind())
                        || ast::CallExpr::can_cast(节点.kind())
                    {
                        *计数.调用点_宿主未定位.entry(相对.clone()).or_insert(0) += 1;
                    }
                    continue;
                };
                let Some(被调) = 被调函数(&语义, &节点, 宿主形态, &函数名表, &mut 计数) else {
                    continue;
                };
                let Some((目标文件, 目标行)) =
                    函数位置(&db, &vfs, 项目根, &mut 源码缓存, &被调)
                else {
                    // 解析出了函数却拿不到仓库内位置：目标是 std / 第三方库（`函数位置`
                    // 里的 `仓库相对` 前缀判定会拒掉仓库外路径）。这一支此前静默丢弃。
                    计数.解析成功_目标在仓库外 += 1;
                    continue;
                };
                let Some(目标id) = 索引.get(&目标文件).and_then(|行表| 行表.get(&目标行)) else {
                    未匹配 += 1;
                    continue;
                };
                // 自递归不成边（否则"死代码"判定会被自己撑住入度）
                if 宿主id.as_str() == 目标id.as_str()
                    || !已建.insert((宿主id.clone(), 目标id.clone()))
                {
                    continue;
                }
                边们.push(边 {
                    从: 宿主id,
                    到: 目标id.clone(),
                    类型: 边种类::调用,
                    置信度: 置信度::高,
                    位置: Some(位置 {
                        文件: 相对.clone(),
                        行: 行号(&文本, usize::from(节点.text_range().start())),
                        列: None,
                    }),
                });
            }
        }
    });

    let mut 诊断 = vec![插件诊断 {
        级别: "提示".into(),
        文件: "rust-analyzer".into(),
        信息: format!("语义层扫描 {已扫文件} 个文件，产出调用边 {} 条", 边们.len()),
    }];
    if 未匹配 > 0 {
        诊断.push(插件诊断 {
            级别: "提示".into(),
            文件: "rust-analyzer".into(),
            信息: format!(
                "{未匹配} 处调用的目标落在仓库内却不在符号表中（宏展开产物、trait 默认方法、未覆盖的符号种类），已跳过，未计入悬空"
            ),
        });
    }

    // 解析损失分类：这些计数此前全部落在调用链的 `continue` 里，从未被任何诊断反映，
    // 是「调用:高」与事实不符的根源。先量化，再决定回退链怎么补。
    诊断.push(插件诊断 {
        级别: "提示".into(),
        文件: "rust-analyzer".into(),
        信息: format!(
            "调用解析分类：方法调用 成功 {} / 解析不出 {}；函数调用 成功 {} / 取不到可调用体 {} / 可调用体非函数 {} / 缺被调表达式 {}；离开函数体的调用点 {}",
            计数.方法调用_解析成功,
            计数.方法调用_解析不出,
            计数.函数调用_解析成功,
            计数.函数调用_取不到可调用体,
            计数.函数调用_可调用体非函数,
            计数.函数调用_缺被调表达式,
            计数.调用点_宿主未定位.values().sum::<usize>(),
        ),
    });
    let 未解析方法 = 前若干(&计数.未解析方法, 20);
    if !未解析方法.is_empty() {
        诊断.push(插件诊断 {
            级别: "提示".into(),
            文件: "rust-analyzer".into(),
            信息: format!("未解析方法（接收者形态::方法名×次数）Top20：{未解析方法}"),
        });
    }
    let 未解析函数 = 前若干(&计数.未解析函数, 20);
    if !未解析函数.is_empty() {
        诊断.push(插件诊断 {
            级别: "提示".into(),
            文件: "rust-analyzer".into(),
            信息: format!("未解析函数调用（接收者形态::被调表达式×次数）Top20：{未解析函数}"),
        });
    }
    let 宿主未定位 = 前若干(&计数.调用点_宿主未定位, 10);
    if !宿主未定位.is_empty() {
        诊断.push(插件诊断 {
            级别: "提示".into(),
            文件: "rust-analyzer".into(),
            信息: format!("调用点离开函数体（文件×次数）Top10：{宿主未定位}"),
        });
    }
    诊断.push(插件诊断 {
        级别: "提示".into(),
        文件: "rust-analyzer".into(),
        信息: format!(
            "解析成功但目标在仓库外（std/第三方，本就不该成边）：{} 处",
            计数.解析成功_目标在仓库外
        ),
    });
    let 仓库内方法 = 前若干(&计数.仓库内方法_解析不出, 30);
    if !仓库内方法.is_empty() {
        诊断.push(插件诊断 {
            级别: "警告".into(),
            文件: "rust-analyzer".into(),
            信息: format!(
                "真损失·仓库内方法解析不出（形态::方法名×次数）Top30：{仓库内方法}｜二分：接收者类型可推 {} / 类型也推不出 {}",
                计数.仓库内方法_接收者类型可推, 计数.仓库内方法_接收者类型推断失败,
            ),
        });
    }
    let 仓库内函数 = 前若干(&计数.仓库内函数_解析不出, 30);
    if !仓库内函数.is_empty() {
        诊断.push(插件诊断 {
            级别: "警告".into(),
            文件: "rust-analyzer".into(),
            信息: format!(
                "真损失·仓库内函数调用解析不出（形态::被调表达式×次数）Top30：{仓库内函数}"
            ),
        });
    }

    Ok((边们, 诊断))
}

/// 解析损失分类计数；只统计「确实是调用表达式」的节点，其余语法节点不计
#[derive(Default)]
struct 解析计数 {
    方法调用_解析成功: usize,
    方法调用_解析不出: usize,
    函数调用_解析成功: usize,
    函数调用_缺被调表达式: usize,
    函数调用_取不到可调用体: usize,
    函数调用_可调用体非函数: usize,
    /// 调用点向上找不到任何 `fn`（如文件级 `const X: Y = f();`），边必然丢失
    调用点_宿主未定位: BTreeMap<String, usize>,
    /// `{接收者形态}::{方法名}` → 次数。用接收者形态分组是本次排查的关键：
    /// 显式接收者 `self: &Arc<Self>` 与简写 `&self` 的解析成功率是否真有差异，靠这个分布说话。
    未解析方法: BTreeMap<String, usize>,
    /// `{接收者形态}::{被调表达式文本}` → 次数
    未解析函数: BTreeMap<String, usize>,
    /// 解析出了函数、但目标是 std / 第三方（拿不到仓库内位置）——此前静默丢弃
    解析成功_目标在仓库外: usize,
    /// **真损失**：解析失败，且名字命中仓库函数表。按 `形态::名字` 分组。
    /// 与 `未解析方法` 的区别：`未解析方法` 含大量 `clone`/`into` 这类 std 方法，
    /// 它们的定义在仓库外、符号表本就只收仓库内符号，解析不出属正常，不能算账；
    /// 只有命中仓库函数名的才是本该成边而没成边的。
    仓库内方法_解析不出: BTreeMap<String, usize>,
    仓库内函数_解析不出: BTreeMap<String, usize>,
    /// 真损失的二分证据：接收者类型推得出、但方法仍解析不出
    仓库内方法_接收者类型可推: usize,
    /// 真损失的二分证据：接收者类型本身就推断不出（类型推断失效）
    仓库内方法_接收者类型推断失败: usize,
}

/// 仓库内全部函数名（去重）。用于把「解析失败」二分为真损失与外部分支：
/// `clone`/`into` 这类 std 方法的定义在仓库外、符号表本就只收仓库内符号，
/// `就绪`/`启动驱动` 这类仓库方法解析不出才是缺陷。
fn 仓库函数名(符号们: &[符号]) -> BTreeSet<String> {
    符号们
        .iter()
        .filter(|符| 符.种类 == 符号种类::函数)
        .map(|符| 符.名.clone())
        .collect()
}

/// 计数表按次数降序取前 n 条，渲染成 `键×次数` 串
fn 前若干(表: &BTreeMap<String, usize>, n: usize) -> String {
    let mut 项: Vec<(&String, &usize)> = 表.iter().collect();
    项.sort_by(|甲, 乙| 乙.1.cmp(甲.1).then_with(|| 甲.0.cmp(乙.0)));
    项.iter()
        .take(n)
        .map(|(键, 次)| format!("{键}×{次}"))
        .collect::<Vec<_>>()
        .join("；")
}

/// 只收 `函数` 种类，避免与行 1 上的模块符号抢同一格子
fn 函数位置索引(符号们: &[符号]) -> 函数索引表 {
    let mut 索引: 函数索引表 = BTreeMap::new();
    for 符 in 符号们 {
        if 符.种类 != 符号种类::函数 {
            continue;
        }
        索引
            .entry(符.文件.clone())
            .or_default()
            .entry(符.行)
            .or_insert_with(|| 符.id.clone());
    }
    索引
}

/// 从调用点向上找最近的 `fn`，用其 `fn` 关键字所在行定位符号
///
/// 用 `fn_token` 而非节点起点：语法树里 `#[属性]` 可能被算进函数节点范围，
/// 而 tree-sitter 侧的行号取自 `fn` 行，两边必须对齐才能桥接上。
///
/// 一并返回宿主的接收者形态（供解析损失按形态分组统计），形态判定不额外解析，只看语法。
fn 宿主符号(
    节点: &SyntaxNode,
    相对路径: &str,
    文本: &str,
    索引: &函数索引表,
) -> Option<(String, &'static str)> {
    let 宿主 = 节点.ancestors().find_map(ast::Fn::cast)?;
    let 行 = 行号(文本, usize::from(宿主.fn_token()?.text_range().start()));
    let id = 索引.get(相对路径)?.get(&行).cloned()?;
    Some((id, 接收者形态(&宿主)))
}

/// 接收者形态：`&self` 与 `self: &Arc<Self>` 在语法上泾渭分明，分开统计
/// 便于在诊断里按「自由函数 / 简写接收者 / 显式接收者」观察解析失败的分布。
fn 接收者形态(函数: &ast::Fn) -> &'static str {
    match 函数.param_list().and_then(|表| 表.self_param()) {
        None => "(自由函数)",
        // `ty()` 有值 = 显式接收者语法 `self: 类型`；无值 = `self` / `&self` / `&mut self`
        Some(自身) if 自身.ty().is_some() => "(显式接收者)",
        Some(_) => "(简写接收者)",
    }
}

/// 把调用点解析到被调函数；解析不出（动态分发、宏、外部函数）则放弃
///
/// 失败一律留痕到 `计数`：此前这些失败在调用处直接 `continue`，既不产生边也不产生诊断，
/// 使调用召回的真实水平无从观测。
fn 被调函数(
    语义: &Semantics<'_, RootDatabase>,
    节点: &SyntaxNode,
    宿主形态: &str,
    函数名表: &BTreeSet<String>,
    计数: &mut 解析计数,
) -> Option<Function> {
    if let Some(方法调用) = ast::MethodCallExpr::cast(节点.clone()) {
        if let Some(函数) = 语义.resolve_method_call(&方法调用) {
            计数.方法调用_解析成功 += 1;
            return Some(函数);
        }
        计数.方法调用_解析不出 += 1;
        let 名 = 方法调用
            .name_ref()
            .map(|引用| 引用.text().to_string())
            .unwrap_or_else(|| "(无名)".to_string());
        *计数
            .未解析方法
            .entry(format!("{宿主形态}::{名}"))
            .or_insert(0) += 1;
        // 名字命中仓库函数表 = 本该成边却漏掉的真损失；未命中则是 std 方法（clone/into…），
        // 它们的定义在仓库外、符号表本就只收仓库内符号，解析不出属正常，不能混进同一个数字里。
        if 函数名表.contains(&名) {
            *计数
                .仓库内方法_解析不出
                .entry(format!("{宿主形态}::{名}"))
                .or_insert(0) += 1;
            // 二分：接收者类型能否推断。推不出说明类型推断失效（要动载入配置）；
            // 推得出却仍解析不出，说明卡在方法/名称解析环节（要动解析调用方式）。
            let 类型可推 = 方法调用
                .receiver()
                .and_then(|接收者| 语义.type_of_expr(&接收者))
                .is_some();
            if 类型可推 {
                计数.仓库内方法_接收者类型可推 += 1;
            } else {
                计数.仓库内方法_接收者类型推断失败 += 1;
            }
        }
        return None;
    }
    // 既非方法调用也非函数调用：绝大多数语法节点走这里，不计数
    let Some(调用) = ast::CallExpr::cast(节点.clone()) else {
        return None;
    };
    let Some(被调表达式) = 调用.expr() else {
        计数.函数调用_缺被调表达式 += 1;
        return None;
    };
    let Some(可调用) = 语义.resolve_expr_as_callable(&被调表达式) else {
        计数.函数调用_取不到可调用体 += 1;
        let 全名 = 被调表达式.syntax().text().to_string();
        let 文: String = 全名.chars().take(40).collect();
        *计数
            .未解析函数
            .entry(format!("{宿主形态}::{文}"))
            .or_insert(0) += 1;
        // 真损失只认「单段路径」的自由函数调用（如 `临时路径(...)`）。
        // `Arc::new` / `TaskBoard::新建` 这类「类型::关联函数」无法只凭末段判断
        // 类型是否落在仓库内——`Arc` 是 std 的、`TaskBoard` 是仓库的，末段同名会误判，
        // 故一律不在此计真损失（宁可漏报，不可误报）。
        if !全名.contains("::") && 函数名表.contains(&全名) {
            *计数
                .仓库内函数_解析不出
                .entry(format!("{宿主形态}::{文}"))
                .or_insert(0) += 1;
        }
        return None;
    };
    match 可调用.kind() {
        CallableKind::Function(函数) => {
            计数.函数调用_解析成功 += 1;
            Some(函数)
        }
        // 元组结构体构造器 / 元组字段等：不是函数，不构成函数级调用边
        _ => {
            计数.函数调用_可调用体非函数 += 1;
            None
        }
    }
}

/// 被调函数的定义位置（仓库相对路径 + 行）
fn 函数位置(
    db: &RootDatabase,
    vfs: &Vfs,
    项目根: &Path,
    缓存: &mut HashMap<FileId, Rc<String>>,
    函数: &Function,
) -> Option<(String, usize)> {
    let 源 = 函数.source(db)?;
    // HirFileId → EditionedFileId → Vfs::FileId 两级下钻：语义层带着 edition 信息，
    // 而 vfs 只认物理文件 id，中间必须过一遍 db 才能剥掉 edition 包装。
    let 文件id = 源.file_id.file_id()?.file_id(db);
    let 偏移 = 源.value.fn_token()?.text_range().start();
    let 相对 = 仓库相对(项目根, vfs.file_path(文件id).as_path()?.as_str())?;
    let 文本 = 源码(vfs, 文件id, 缓存);
    Some((相对, 行号(&文本, usize::from(偏移))))
}

/// 取文件文本；同一文件多次问只读一次盘
fn 源码(vfs: &Vfs, 文件id: FileId, 缓存: &mut HashMap<FileId, Rc<String>>) -> Rc<String> {
    if let Some(文) = 缓存.get(&文件id) {
        return 文.clone();
    }
    let 文本 = vfs
        .file_path(文件id)
        .as_path()
        .and_then(|路径| std::fs::read_to_string(路径.as_str()).ok())
        .unwrap_or_default();
    let 文 = Rc::new(文本);
    缓存.insert(文件id, 文.clone());
    文
}

/// 绝对路径 → 仓库相对路径（正斜杠）。Windows 的 `\\?\` 前缀与盘符大小写都要抹平
fn 仓库相对(项目根: &Path, 绝对: &str) -> Option<String> {
    let 根 = 规整(&项目根.to_string_lossy());
    let 根 = 根.trim_end_matches('/');
    let 剥 = 绝对
        .strip_prefix(r"\\?\")
        .unwrap_or(绝对)
        .replace('\\', "/");
    if !规整(&剥).starts_with(根) {
        return None;
    }
    Some(剥[根.len()..].trim_start_matches('/').to_string())
}

fn 规整(路径: &str) -> String {
    let 剥 = 路径.strip_prefix(r"\\?\").unwrap_or(路径);
    剥.replace('\\', "/").to_lowercase()
}

/// 字节偏移 → 1-based 行号（与 tree-sitter 的 `row + 1` 对齐）
fn 行号(文本: &str, 偏移: usize) -> usize {
    let 截止 = 偏移.min(文本.len());
    文本.as_bytes()[..截止]
        .iter()
        .filter(|字节| **字节 == b'\n')
        .count()
        + 1
}
