//! Rust 语言解析插件
//!
//! 产出：文件级模块符号 + 其内符号 + `包含` 边。
//!
//! 第一版能力边界（与设计 §七 步 3 对应）：
//! - `包含`：模块 → 其内符号，置信度 **中**（语法级，tree-sitter）
//! - `导入` / `实现` / `调用`：**尚未实现**，故 `能力` 中不声明（`调用` 由 `调用-扫描-园` 独立补出）
//!
//! 下钻范围：`impl` 块、内联 `mod`、函数体都会继续进入，方法、关联项、嵌套模块、
//! 局部函数（nested fn）内的符号都登记。下钻函数体是为了收录局部函数，否则其内部
//! 调用点会因宿主未定位而丢边。
//!
//! ID 规范见设计 §4.4：`rust::<包名>::<模块路径>::<符号名>`，**不含行号**。
//! 嵌套符号的 ID 逐层接在容器后面（`<容器 ID>::<名>`），容器链同时落在 `容器` 字段上。

use hm_symext::{边, 符号, 边种类, 符号种类, 置信度};

/// 解析单个 .rs 源文件，返回该文件的符号与包含边
///
/// `相对路径` 是仓库相对路径（正斜杠分隔），`包名` 是该文件所属 crate 名。
pub fn 解析文件(
    相对路径: &str,
    包名: &str,
    源码: &[u8],
) -> Result<(Vec<符号>, Vec<边>), String> {
    let mut 解析器 = tree_sitter::Parser::new();
    解析器
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| format!("加载 Rust 语法失败：{e}"))?;
    let 树 = 解析器
        .parse(源码, None)
        .ok_or_else(|| "tree-sitter 未能产出语法树".to_string())?;
    let 根 = 树.root_node();

    let 模块路径 = 模块路径(相对路径);
    let 模块id = format!("rust::{包名}::{模块路径}");

    // 文件级模块符号的容器是它的**父模块**，不是一律挂在 crate 根：
    // `a/b/模块.rs` 归为模块 `a::b`，容器应为 `a`。原先一律写 crate 根，
    // 与 `mod` 声明产出的同名符号（容器正是 `a`）对不上，归一化去重时就会打架。
    let 父路径 = 模块路径
        .rsplit_once("::")
        .map(|(父, _)| 父)
        .unwrap_or("");
    let 父容器id = if 父路径.is_empty() {
        format!("rust::{包名}")
    }
    else {
        format!("rust::{包名}::{父路径}")
    };

    let mut 符号们 = vec![符号 {
        id: 模块id.clone(),
        种类: 符号种类::模块,
        名: 模块路径
            .rsplit("::")
            .next()
            .unwrap_or(&模块路径)
            .to_string(),
        全名: 模块路径.clone(),
        文件: 相对路径.to_string(),
        行: 1,
        可见性: "私有".to_string(),
        容器: Some(父容器id),
    }];

    let mut 边们 = Vec::new();

    let mut 游标 = 根.walk();
    for 子 in 根.named_children(&mut 游标) {
        收下(子, 源码, 相对路径, 包名, &模块id, &mut 符号们, &mut 边们);
    }

    Ok((符号们, 边们))
}

/// 收下一个条目：登记符号、连 `包含` 边；`impl` 与内联 `mod` 还要继续下钻
fn 收下(
    节点: tree_sitter::Node,
    源码: &[u8],
    相对路径: &str,
    包名: &str,
    容器id: &str,
    符号们: &mut Vec<符号>,
    边们: &mut Vec<边>,
) {
    let Some(符) = 造符号(节点, 源码, 相对路径, 包名, 容器id) else {
        return;
    };

    边们.push(边 {
        从: 容器id.to_string(),
        到: 符.id.clone(),
        类型: 边种类::包含,
        置信度: 置信度::中,
        位置: Some(hm_symext::位置 {
            文件: 相对路径.to_string(),
            行: 符.行,
            列: None,
        }),
    });

    let 子容器id = 符.id.clone();
    符号们.push(符);

    if !可下钻(节点) {
        return;
    }
    let Some(体) = 节点.child_by_field_name("body") else {
        return;
    };
    let mut 游标 = 体.walk();
    for 子 in 体.named_children(&mut 游标) {
        收下(子, 源码, 相对路径, 包名, &子容器id, 符号们, 边们);
    }
}

/// `impl` 块、内联 `mod`、函数体内还装符号（函数体用于收录局部函数 nested fn）；
/// `mod x;` 声明没有 body，自然止步。
fn 可下钻(节点: tree_sitter::Node) -> bool {
    matches!(节点.kind(), "impl_item" | "mod_item" | "function_item")
}

/// 从单个条目节点造符号；容器链决定 ID，不是顶层项则返回 `None`
fn 造符号(
    节点: tree_sitter::Node,
    源码: &[u8],
    相对路径: &str,
    包名: &str,
    容器id: &str,
) -> Option<符号> {
    let 种类 = match 节点.kind() {
        "function_item" => 符号种类::函数,
        "struct_item" => 符号种类::结构体,
        "enum_item" => 符号种类::枚举,
        "union_item" => 符号种类::联合,
        "trait_item" => 符号种类::特征,
        "impl_item" => 符号种类::实现块,
        "type_item" => 符号种类::类型别名,
        "const_item" => 符号种类::常量,
        "static_item" => 符号种类::静态量,
        "mod_item" => 符号种类::模块,
        _ => return None,
    };

    let 名 = 条目名(节点, 源码)?;
    let id = format!("{容器id}::{名}");
    // 全名 = ID 去掉 `rust::<包名>::` 前缀，与旧版顶层项的全名写法完全一致
    let 全名 = id
        .strip_prefix(&format!("rust::{包名}::"))
        .map(str::to_string)
        .unwrap_or_else(|| id.clone());

    Some(符号 {
        id,
        种类,
        名,
        全名,
        文件: 相对路径.to_string(),
        行: 节点.start_position().row + 1,
        可见性: if 有可见性修饰(节点) { "pub" } else { "私有" }.to_string(),
        容器: Some(容器id.to_string()),
    })
}

/// 条目名。`impl` 块无 `name` 字段，用被实现的类型名；带 trait 时须一并带上，
/// 否则同一类型的多个 trait impl（`impl Display for 错误` 与 `impl Debug for 错误`）
/// 会算出同一个 ID，方法也跟着撞。
fn 条目名(节点: tree_sitter::Node, 源码: &[u8]) -> Option<String> {
    if 节点.kind() == "impl_item" {
        let 类型 = 文本(节点.child_by_field_name("type")?, 源码)?;
        let 头 = match 节点.child_by_field_name("trait") {
            Some(特征) => format!("{} for {}", 文本(特征, 源码)?, 类型),
            None => 类型,
        };
        return Some(format!("impl {头}"));
    }

    let 名节点 = 节点.child_by_field_name("name")?;
    文本(名节点, 源码)
}

fn 文本(节点: tree_sitter::Node, 源码: &[u8]) -> Option<String> {
    节点
        .utf8_text(源码)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn 有可见性修饰(节点: tree_sitter::Node) -> bool {
    let mut 游标 = 节点.walk();
    for 子 in 节点.children(&mut 游标) {
        if 子.kind() == "visibility_modifier" {
            return true;
        }
    }
    false
}

/// 文件相对路径 → 模块路径
///
/// `a/b/模块.rs` 与 `a/b/mod.rs` 归为 `a::b`；`a/main.rs` 与 `a/lib.rs` 归为 `a`。
fn 模块路径(相对路径: &str) -> String {
    let 无后缀 = 相对路径.strip_suffix(".rs").unwrap_or(相对路径);
    let 去模块 = 无后缀.strip_suffix("/模块").unwrap_or(无后缀);
    let 卷起的 = 去模块
        .strip_suffix("/main")
        .or_else(|| 去模块.strip_suffix("/lib"))
        .unwrap_or(去模块);
    let 结果 = 卷起的.replace('/', "::");
    if 结果.is_empty() {
        "crate".to_string()
    }
    else {
        结果
    }
}
