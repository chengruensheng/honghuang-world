use crate::{符号, 符号种类};
use tree_sitter::{Node, Parser};

/// tree-sitter 符号提取器：用 tree-sitter-rust 语法树替换手写行级状态机，
/// 图谱符号携带真实签名（函数参数/返回类型）。语言无关适配器接口不变（扫描契约仍在 Rust扫描器）。
pub struct 树扫描器 {
    解析器: Parser,
}

impl 树扫描器 {
    /// 构造扫描器（内置 Rust 语言语法；设置语言失败不 panic，解析时降级空）
    pub fn 新() -> Self {
        let mut 解析器 = Parser::new();
        if let Err(失败) = 解析器.set_language(&tree_sitter_rust::LANGUAGE.into()) {
            tracing::error!("tree-sitter 设置语言失败: {失败}");
        }
        树扫描器 { 解析器 }
    }

    /// 提取源码内全部顶层 pub 符号（函数/类型/常量），函数符号携带签名（参数+返回类型）
    pub fn 提取符号(&mut self, 源码: &[u8], 包名: &str) -> Vec<符号> {
        let Some(树) = self.解析器.parse(源码, None) else {
            return Vec::new(); // 解析失败降级空（扫描失败回退空图谱，不阻断）
        };
        let 根 = 树.root_node();
        let mut 结果: Vec<符号> = Vec::new();
        收集符号(根, 源码, 包名, &mut 结果);
        结果
    }
}

/// 递归收集带可见性修饰符（pub）的符号项（含 mod 内部的 pub 项，与手写逐行 trim 语义一致）
fn 收集符号(节点: Node, 源码: &[u8], 包名: &str, 结果: &mut Vec<符号>) {
    if let Some(符号) = 解析节点(节点, 源码, 包名) {
        结果.push(符号);
    }
    for i in 0..节点.named_child_count() {
        if let Some(子) = 节点.named_child(i as u32) {
            收集符号(子, 源码, 包名, 结果);
        }
    }
}

/// 单个节点 → 符号（仅 pub 项；非符号项/非 pub 返回 None）
fn 解析节点(节点: Node, 源码: &[u8], 包名: &str) -> Option<符号> {
    if !是发布(&节点) {
        return None; // 非 pub 不提取
    }
    let 名称 = 字段文本(节点, "name", 源码)?;
    let (种类, 签名) = match 节点.kind() {
        "function_item" => (符号种类::函数, Some(函数签名(节点, 源码, 名称))),
        // trait 与手写实现一致归入「函数」类，无签名
        "trait_item" => (符号种类::函数, None),
        "struct_item" | "enum_item" | "type_item" => (符号种类::类型, None),
        "const_item" | "static_item" => (符号种类::常量, None),
        _ => return None, // mod/use/impl 等不提取
    };
    Some(符号 {
        名称: 名称.to_string(),
        种类,
        所属模块: 包名.to_string(),
        签名,
    })
}

/// 判断节点是否 pub（tree-sitter-rust 中 visibility_modifier 是 named 子节点而非 field，需遍历 children）
fn 是发布(节点: &Node) -> bool {
    (0..节点.named_child_count())
        .filter_map(|i| 节点.named_child(i as u32))
        .any(|子| 子.kind() == "visibility_modifier")
}

/// 取节点的具名字段文本（如 name 字段）
fn 字段文本<'a>(节点: Node<'a>, 字段: &str, 源码: &'a [u8]) -> Option<&'a str> {
    节点
        .child_by_field_name(字段)
        .and_then(|n| n.utf8_text(源码).ok())
}

/// 函数签名：fn 名称(参数) -> 返回（截 120 字符，与手写实现一致）
fn 函数签名(节点: Node, 源码: &[u8], 名称: &str) -> String {
    let 参数 = 节点
        .child_by_field_name("parameters")
        .and_then(|p| p.utf8_text(源码).ok())
        .unwrap_or_else(|| "()");
    let 返回 = 节点
        .child_by_field_name("return_type")
        .and_then(|r| r.utf8_text(源码).ok())
        .map(|r| format!(" -> {}", r))
        .unwrap_or_default();
    let 签名 = format!("fn {}{}{}", 名称, 参数, 返回);
    签名.chars().take(120).collect()
}
