//! 验收 - 裁决：确认设计 / 验收裁决 / 定档。
//!
//! `验收裁决` 入口保留旧签名（向后兼容），内部委托 `终裁::终裁裁决_无名` 执行三段流程。
//! 三段流程：① 机械前置门槛 → ② 六准圣 LLM 分维独立审验 → ③ 鸿钧终裁。
//! 无 LLM 配置或无要求书时自动降级为规则兜底（路径相符 + 模块接入）。

use crate::净化涉及路径;
use crate::类型_定义_殿::*;
use moxing_fu::用量;
use rizhi_fu::{info, warn};

use super::终裁::终裁裁决_无名;

/// 确认设计：机械校验（拆解非空 / 工作流合法 / 涉及路径合法 / 自评非空）。
/// fail-closed 但必须可诊断：失败逐类点名（拆解空/目标空/工作流非法/涉及路径非法/自评空），
/// 防「设计校验未过」一句抽象结论让模型与界主盲改（2026-08-17 世界 昼夜 设计打回实测）。
/// 涉及路径合法性复用 `净化涉及路径`（首段已知维度/不含 `..`/段名合法/合法扩展名或结构目录），
/// 逐路径单独净化，结果为空即该路径非法（2026-08-20 设计阶段加固，设计稿 §11.2）。
pub fn 确认设计(方案: &设计方案, 涉及路径: &[String]) -> 验收结论 {
    let 拆解合法 = !方案.拆解.is_empty()
        && 方案
            .拆解
            .iter()
            .all(|项| !项.目标.is_empty() && 合法工作流(&项.工作流));
    // 涉及路径合法性：逐路径单独净化，净化剔除（返回空）即该路径非法。
    let 路径合法 = 涉及路径
        .iter()
        .all(|路径| !净化涉及路径(vec![路径.to_string()]).is_empty());
    // 失败明细：五类原因按优先级点名第一个违规项
    // （拆解空 > 目标空 > 工作流非法 > 涉及路径非法 > 自评空）。
    let 明细: Option<String> = if 方案.拆解.is_empty() {
        Some("拆解为空（未产出子任务）".to_string())
    } else if 方案.拆解.iter().find(|项| 项.目标.is_empty()).is_some() {
        Some("拆解项存在空目标".to_string())
    } else if let Some(项) = 方案.拆解.iter().find(|项| !合法工作流(&项.工作流)) {
        Some(format!("拆解项工作流非法：{}", 项.工作流))
    } else if let Some(路径) = 涉及路径
        .iter()
        .find(|路径| 净化涉及路径(vec![路径.to_string()]).is_empty())
    {
        Some(format!("涉及路径非法：{路径}"))
    } else if 方案.自评.is_empty() {
        Some("自评为空（缺少验收自证）".to_string())
    } else {
        None
    };
    let 结论 = if 拆解合法 && 路径合法 && !方案.自评.is_empty() {
        验收结论::通过
    } else {
        warn!(要求 = %方案.要求id, 明细 = ?明细, "设计校验未过，打回重审");
        验收结论::打回
    };
    info!(要求 = %方案.要求id, 结论 = ?结论, "设计确认完成");
    结论
}

/// 工作流标识是否合法。
pub fn 合法工作流(工作流: &str) -> bool {
    matches!(工作流, "L1_qa" | "L2_script" | "L3_program" | "L4_complex")
}

/// 验收裁决：产物清单 → 三段流程。
///
/// 向后兼容旧签名（`pub fn 验收裁决(要求id, 产物们, 耗时秒, 涉及文件, 失败说明) -> 验收回执`），
/// 内部委托 `终裁裁决_无名`，不传要求书与 LLM 配置，自动降级为规则兜底。
/// 全版本（含要求书 + 六准圣 LLM 审验 + 鸿钧终裁）请用 `终裁::终裁裁决(要求书, 产物们, ..., 配置)`。
pub fn 验收裁决(
    要求id: &str,
    产物们: &[产物条目],
    耗时秒: f64,
    涉及文件: &[String],
    失败说明: Option<&str>,
) -> 验收回执 {
    终裁裁决_无名(要求id, None, 产物们, 耗时秒, 涉及文件, 失败说明, None).验收
}

/// 定档：验收回执 → 回填识海承载-府的「验收结果」格位。
/// 记录内容为 JSON：结论 / 生成物（相对路径 + 绝对路径 + 字节数）/ token 用量（任务全程累计），便于后续按路径管理、对账。
pub fn 定档(
    存储: &shihai_fu::模型存储,
    回执: &验收回执,
    产物们: &[产物条目],
    用量: &用量,
) -> Result<(), String> {
    // 接口契约扫描（设计稿 §4.2 规则6 配套）：定档时刷新 workspace pub API 清单入格位，
    // 供执行现状拼装注入「可用API清单」。扫描失败不阻断定档（只 warn）。
    扫描接口契约写入格位(存储);
    let 根 = shihai_fu::工作区::定位();
    let 根路径 = 根.根路径();
    let 生成物们 = 产物们
        .iter()
        .map(|产物| {
            let 绝对 = 根路径.join(&产物.路径);
            serde_json::json!({
                "路径": 产物.路径,
                "绝对路径": 绝对.to_string_lossy(),
                "字节数": 产物.字节数,
            })
        })
        .collect::<Vec<_>>();
    let 内容 = serde_json::json!({
        "结论": format!("{:?}", 回执.结论),
        "生成物": 生成物们,
        "token": {
            "提示词": 用量.提示词,
            "输出": 用量.输出,
            "缓存命中": 用量.缓存命中,
            "总计": 用量.总计,
        },
    })
    .to_string();
    let 结果 = 存储.写记录(&shihai_fu::记录::新(
        "验收结果",
        &内容,
        &format!("验收裁决「{}」", 回执.要求id),
        "代码",
    ));
    match &结果 {
        Ok(()) => info!(
            要求 = %回执.要求id, 提示词 = 用量.提示词, 缓存命中 = 用量.缓存命中, 产物数 = 产物们.len(),
            "定档入库"
        ),
        Err(错误) => warn!(要求 = %回执.要求id, "定档失败：{错误}"),
    }
    结果
}

/// 扫描 workspace 全部 crate 的库根 pub API 签名，写入「结构」格位（设计稿 §4.2 规则6 配套）。
/// 定档环节调用：每次定档时刷新一次 workspace pub API 清单，供执行现状拼装注入「可用API清单」。
/// 亦供构造现状冷启动调用：首次执行时「结构」格位无 API 契约记录，先扫描一次打破死循环。
/// 每个crate一条记录，实体键=API·{lib名}（前缀区分结构格位中其他记录），内容=该crate的pub符号签名清单。
/// 跨府只经 shihai_fu lib 根：读workspace成员缓存在 / 依赖图::加载自工作区 / 记录::新 / 存储.写记录。
pub fn 扫描接口契约写入格位(存储: &shihai_fu::模型存储) {
    let 工作区 = shihai_fu::工作区::定位();
    let 图 = shihai_fu::依赖图::加载自工作区(&工作区).unwrap_or_default();
    let Some(摘要) = shihai_fu::读workspace成员缓存在(&工作区) else {
        warn!("workspace 成员摘要未就绪，跳过接口契约扫描");
        return;
    };
    // crate目录名 → lib名 映射（无lib名用crate目录名兜底）
    let 目录到lib: std::collections::HashMap<String, String> = 摘要
        .府间依赖
        .iter()
        .map(|府| {
            (
                府.府名.clone(),
                府.lib名.clone().unwrap_or_else(|| 府.府名.clone()),
            )
        })
        .collect();
    // crate目录名 → 库根文件名 映射（从Cargo.toml [lib] path获取，不硬编码文件名）
    let 目录到库根: std::collections::HashMap<String, String> = 摘要
        .府间依赖
        .iter()
        .map(|府| (府.府名.clone(), 府.库根文件名.clone()))
        .collect();
    // 按crate目录名（档案.模块）分组，每组收集签名清单；库根文件名从workspace成员摘要获取（项目无关）
    let mut 按crate分组: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for 档案 in &图.档案们 {
        if 档案.签名.is_empty() {
            continue;
        }
        let 模块 = &档案.模块;
        按crate分组
            .entry(模块.clone())
            .or_default()
            .push(档案.签名.clone());
    }
    let mut 总条数 = 0usize;
    for (crate目录名, 签名们) in &按crate分组 {
        let lib名 = 目录到lib
            .get(crate目录名)
            .cloned()
            .unwrap_or_else(|| crate目录名.clone());
        let 库根 = 目录到库根
            .get(crate目录名)
            .cloned()
            .unwrap_or_else(|| "（未找到）".to_string());
        let 内容 = format!("[{}] 库根={}\n  {}", lib名, 库根, 签名们.join("\n  "));
        let mut 记录 =
            shihai_fu::记录::新("结构", &内容, &format!("定档扫描·{}", lib名), "代码");
        记录.实体键 = format!("API·{}", lib名);
        if let Err(错误) = 存储.写记录(&记录) {
            warn!(crate = %lib名, 错误 = %错误, "接口契约写入失败");
        } else {
            总条数 += 1;
        }
    }
    info!(crate数 = 总条数, "接口契约已扫描写入格位");
}

#[cfg(test)]
mod 测试 {
    use super::确认设计;
    use crate::类型_定义_殿::{拆解项, 设计方案, 验收结论};

    fn 造合法方案() -> 设计方案 {
        设计方案 {
            要求id: "r1".to_string(),
            设计: "设计".to_string(),
            拆解: vec![拆解项 {
                目标: "目标".to_string(),
                执行层角色: vec![],
                工作流: "L2_script".to_string(),
            }],
            自评: "自评".to_string(),
        }
    }

    #[test]
    fn 确认设计拒绝含上跳段的涉及路径() {
        let 方案 = 造合法方案();
        let 非法路径 = vec!["鸿蒙/基础设施 - 域/../etc/x.rs".to_string()];
        assert_eq!(确认设计(&方案, &非法路径), 验收结论::打回);
    }

    #[test]
    fn 确认设计拒绝首段非已知维度的涉及路径() {
        let 方案 = 造合法方案();
        let 非法路径 = vec!["不存在的维度/某-府/x.rs".to_string()];
        assert_eq!(确认设计(&方案, &非法路径), 验收结论::打回);
    }

    #[test]
    fn 确认设计拒绝非合法扩展名的涉及路径() {
        let 方案 = 造合法方案();
        let 非法路径 = vec!["鸿蒙/基础设施 - 域/某-府/x.非法后缀".to_string()];
        assert_eq!(确认设计(&方案, &非法路径), 验收结论::打回);
    }

    #[test]
    fn 确认设计通过合法涉及路径() {
        let 方案 = 造合法方案();
        let 合法路径 = vec![
            "鸿蒙/基础设施 - 域/天庭治理-府/天层-主控-殿/鸿钧-主政-阁/主政-落笔-园/验收.rs"
                .to_string(),
        ];
        assert_eq!(确认设计(&方案, &合法路径), 验收结论::通过);
    }

    #[test]
    fn 确认设计通过结构目录路径() {
        let 方案 = 造合法方案();
        // 末段以 -园 结尾的结构目录路径应合法（新建园目录类任务落点）。
        let 合法路径 = vec!["鸿蒙/基础设施 - 域/某-府/某-殿/某-阁/某-园".to_string()];
        assert_eq!(确认设计(&方案, &合法路径), 验收结论::通过);
    }

    #[test]
    fn 确认设计空涉及路径不阻断() {
        let 方案 = 造合法方案();
        assert_eq!(确认设计(&方案, &[]), 验收结论::通过);
    }

    #[test]
    fn 确认设计混合路径有非法则打回() {
        let 方案 = 造合法方案();
        let 路径 = vec![
            "鸿蒙/基础设施 - 域/某-府/a.rs".to_string(),
            "鸿蒙/基础设施 - 域/../b.rs".to_string(),
        ];
        assert_eq!(确认设计(&方案, &路径), 验收结论::打回);
    }
}
