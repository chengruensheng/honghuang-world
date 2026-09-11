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

    let 载入配置 = LoadCargoConfig {
        // 不跑 build script：本园不做类型推断之外的用途，省一次 cargo check
        load_out_dirs_from_check: false,
        with_proc_macro_server: ProcMacroServerChoice::None,
        prefill_caches: false,
        num_worker_threads: 0,
        proc_macro_processes: 0,
    };

    // 载入配置：`no_deps = true` 让 cargo 以 `--no-deps` 取元数据，只返回 workspace
    // 成员、不拉 registry 依赖。这是本园耗时的主开关——rust-analyzer 默认会把**整个
    // 依赖图**（本仓库下即 ra_ap_* 等成百个外部 crate）的源码目录都装进 VFS，载入
    // 成本几乎全在这里。本园只做「仓库内符号 → 仓库内符号」的桥接，外部依赖的调用
    // 本就因不在符号表中而被跳过，故关掉依赖不损语义。
    let 载入工作区配置 = CargoConfig {
        no_deps: true,
        sysroot: Some(ra_ap_project_model::RustLibSource::Discover),
        ..CargoConfig::default()
    };
    let (db, vfs, _进程宏) =
        load_workspace_at(项目根, &载入工作区配置, &载入配置, &|_| {})
            .map_err(|e| format!("rust-analyzer 载入工作区失败：{e}"))?;

    let mut 边们: Vec<边> = Vec::new();
    let mut 已扫文件 = 0usize;
    let mut 未匹配 = 0usize;

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
                let Some(宿主id) = 宿主符号(&节点, &相对, &文本, &索引) else {
                    continue;
                };
                let Some(被调) = 被调函数(&语义, &节点) else {
                    continue;
                };
                let Some((目标文件, 目标行)) =
                    函数位置(&db, &vfs, 项目根, &mut 源码缓存, &被调)
                else {
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

    Ok((边们, 诊断))
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
fn 宿主符号(节点: &SyntaxNode, 相对路径: &str, 文本: &str, 索引: &函数索引表) -> Option<String> {
    let 宿主 = 节点.ancestors().find_map(ast::Fn::cast)?;
    let 行 = 行号(文本, usize::from(宿主.fn_token()?.text_range().start()));
    索引.get(相对路径)?.get(&行).cloned()
}

/// 把调用点解析到被调函数；解析不出（动态分发、宏、外部函数）则放弃
fn 被调函数(语义: &Semantics<'_, RootDatabase>, 节点: &SyntaxNode) -> Option<Function> {
    if let Some(方法调用) = ast::MethodCallExpr::cast(节点.clone()) {
        return 语义.resolve_method_call(&方法调用);
    }
    let callee = ast::CallExpr::cast(节点.clone())?.expr()?;
    match 语义.resolve_expr_as_callable(&callee)?.kind() {
        CallableKind::Function(函数) => Some(函数),
        _ => None,
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
