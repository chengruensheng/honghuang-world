use std::collections::HashMap;

/// 可插入的文本嵌入器：把文本映射为语义向量。
///
/// 可插拔原则：默认用本地 `字符嵌入器`（零依赖、零模型文件、离线可用），
/// 可替换为 ONNX（bge-small-zh）或已接入供应商 embeddings API（仅需实现本 trait）。
pub trait 嵌入器: Send + Sync {
    /// 文本 → 语义向量（空文本返回空向量）
    fn 嵌入(&self, 文本: &str) -> Vec<f32>;
}

/// 语义向量维度：本地字符嵌入器的哈希槽位总数
const 语义维度: usize = 512;

/// 本地字符嵌入器：中文 2-gram + 英文 token 稳定哈希到固定维度并 L2 归一化。
///
/// 词序不敏感（「任务看板」与「看板任务」哈希到相同中文 2-gram 集合→高相似度），
/// 对中文短串语义去序有效，保证语义检索离线可用（验收「同义不同词可命中」）。
pub struct 字符嵌入器;

impl 字符嵌入器 {
    pub fn 新() -> Self {
        字符嵌入器
    }
}

impl 嵌入器 for 字符嵌入器 {
    fn 嵌入(&self, 文本: &str) -> Vec<f32> {
        if 文本.trim().is_empty() {
            return Vec::new();
        }
        let mut 桶: HashMap<String, usize> = HashMap::new();
        // 中文 2-gram
        for 中文段 in 文本.split(|c: char| !(c >= '\u{4e00}' && c <= '\u{9fff}')) {
            let 字: Vec<char> = 中文段.chars().collect();
            if 字.len() < 2 {
                continue;
            }
            for i in 0..字.len() - 1 {
                let 片段: String = 字[i..i + 2].iter().collect();
                *桶.entry(片段).or_insert(0) += 1;
            }
        }
        // 英文字符 2-gram（大小写不敏感）：支持词形变化（authenticate/authentication）与词序变体的语义命中
        for 词 in 文本.split(|c: char| !c.is_ascii_alphanumeric()) {
            let 小写: Vec<char> = 词.to_lowercase().chars().collect();
            if 小写.len() < 2 {
                continue;
            }
            for i in 0..小写.len() - 1 {
                let 片段: String = 小写[i..i + 2].iter().collect();
                *桶.entry(片段).or_insert(0) += 1;
            }
        }
        if 桶.is_empty() {
            return Vec::new();
        }
        let mut 向量 = vec![0f32; 语义维度];
        for (片段, 计数) in 桶 {
            let 索引 = (稳定哈希(&片段) % 语义维度 as u64) as usize;
            向量[索引] += 计数 as f32;
        }
        归一化(&mut 向量);
        向量
    }
}

/// FNV-1a 64 位稳定哈希（无外部依赖，纯字面量实现，保证跨平台一致）
fn 稳定哈希(文本: &str) -> u64 {
    let mut 哈希: u64 = 0xcbf2_9ce4_8422_2325;
    for 字节 in 文本.as_bytes() {
        哈希 ^= *字节 as u64;
        哈希 = 哈希.wrapping_mul(0x0000_0100_0000_01b3);
    }
    哈希
}

/// L2 归一化（范数为 0 时不改，避免除零）
fn 归一化(向量: &mut [f32]) {
    let 范数: f32 = 向量.iter().map(|x| x * x).sum::<f32>().sqrt();
    if 范数 > 0.0 {
        for 值 in 向量.iter_mut() {
            *值 /= 范数;
        }
    }
}

/// 两文本语义相似度（归一化余弦，范围 0~1；任一为空返回 0）
pub fn 文本相似度(甲: &str, 乙: &str, 嵌入器: &dyn 嵌入器) -> f32 {
    let 甲向量 = 嵌入器.嵌入(甲);
    let 乙向量 = 嵌入器.嵌入(乙);
    余弦相似度(&甲向量, &乙向量)
}

/// 余弦相似度（两向量均 L2 归一化后，点积即余弦；长度不一时取较短范围）
pub fn 余弦相似度(甲: &[f32], 乙: &[f32]) -> f32 {
    let 长度 = 甲.len().min(乙.len());
    if 长度 == 0 {
        return 0.0;
    }
    (0..长度).map(|i| 甲[i] * 乙[i]).sum()
}

/// 在候选名中找与问题语义最相似的项；相似度 ≥ 阈值返回 Some((命中名, 相似度))，否则 None
pub fn 语义检索(
    问题: &str,
    候选名: &[String],
    嵌入器: &dyn 嵌入器,
    阈值: f32,
) -> Option<(String, f32)> {
    if 问题.trim().is_empty() || 候选名.is_empty() {
        return None;
    }
    let 问题向量 = 嵌入器.嵌入(问题);
    if 问题向量.is_empty() {
        return None;
    }
    let mut 最佳: Option<(String, f32)> = None;
    for 名 in 候选名 {
        if 名.is_empty() {
            continue;
        }
        let 名向量 = 嵌入器.嵌入(名);
        let 相似度 = 余弦相似度(&问题向量, &名向量);
        if 相似度 >= 阈值 && 最佳.as_ref().map_or(true, |(_, 当前)| 相似度 > *当前) {
            最佳 = Some((名.clone(), 相似度));
        }
    }
    最佳
}
