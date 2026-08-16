//! 批量 - 登记 - 园：变更报告 → 事件格位（汇总）+ 变更格位（逐文件明细）。
//!
//! 幂等：变更格位按相对路径设实体键，同一路径反复变更只保留最新链头。

use crate::{记录, 模型存储, 变更报告};
use rizhi_fu::{debug, info};

/// 登记变更：总处数 > 0 时，事件格位写一条汇总，变更格位逐文件写明细。
/// 无变更时零开销返回，不产生任何记录。
pub fn 登记变更(存储: &模型存储, 报告: &变更报告) -> Result<(), String> {
    let 总数 = 报告.总处数();
    if 总数 == 0 {
        return Ok(());
    }
    let 汇总 = format!(
        "地道·变更 {总数} 处（新增 {}、修改 {}、删除 {}）",
        报告.新增.len(),
        报告.修改.len(),
        报告.删除.len()
    );
    存储.写记录(&记录::新("事件", &汇总, "地道增量检测", "代码"))?;
    for 路径 in &报告.新增 {
        写变更明细(存储, 路径, "新增")?;
    }
    for 路径 in &报告.修改 {
        写变更明细(存储, 路径, "修改")?;
    }
    for 路径 in &报告.删除 {
        写变更明细(存储, 路径, "删除")?;
    }
    info!(总处数 = 总数, 事件 = "事件/变更", "变更已登记");
    Ok(())
}

/// 逐文件写一条变更明细（实体键 = 相对路径）。
fn 写变更明细(存储: &模型存储, 路径: &str, 类别: &str) -> Result<(), String> {
    let mut 记录 = 记录::新(
        "变更",
        &format!("{类别} {路径}"),
        &format!("地道增量检测：{路径}"),
        "代码",
    );
    记录.实体键 = 路径.to_string();
    存储.写记录(&记录)?;
    debug!(路径, 类别, "变更明细已登记");
    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use std::fs;

    fn 建报告() -> 变更报告 {
        变更报告 {
            新增: vec!["甲.rs".to_string(), "乙.rs".to_string()],
            修改: vec!["丙.rs".to_string()],
            删除: vec!["丁.rs".to_string()],
        }
    }

    #[test]
    fn 登记_写入事件与变更格位() {
        let 目录 = std::env::temp_dir().join(format!("地道登记测试-{}", crate::当前毫秒()));
        let 存储 = 模型存储::打开(&目录);
        登记变更(&存储, &建报告()).unwrap();
        let 事件们 = 存储.读格位("事件").unwrap();
        let 变更们 = 存储.读格位("变更").unwrap();
        assert_eq!(事件们.len(), 1);
        assert_eq!(变更们.len(), 4);
        assert!(事件们[0].内容.contains("地道·变更 4 处"));
        assert!(事件们[0].内容.contains("新增 2"));
        assert!(变更们.iter().all(|记录| 记录.实体键 == 记录.内容.split(' ').nth(1).unwrap()));
        fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 登记_同路径幂等只留链头() {
        let 目录 = std::env::temp_dir().join(format!("地道幂等测试-{}", crate::当前毫秒()));
        let 存储 = 模型存储::打开(&目录);
        let 报告 = 变更报告 {
            新增: vec!["甲.rs".to_string()],
            修改: Vec::new(),
            删除: Vec::new(),
        };
        登记变更(&存储, &报告).unwrap();
        登记变更(&存储, &报告).unwrap();
        let 链头 = 存储.读链头集("变更").unwrap();
        assert_eq!(链头.len(), 1, "同路径重复登记后，链头集只剩最新一条");
        fs::remove_dir_all(&目录).unwrap();
    }

    #[test]
    fn 登记_无变更零开销() {
        let 目录 = std::env::temp_dir().join(format!("地道零开销测试-{}", crate::当前毫秒()));
        let 存储 = 模型存储::打开(&目录);
        登记变更(&存储, &变更报告::default()).unwrap();
        assert!(存储.读格位("事件").unwrap().is_empty());
        assert!(存储.读格位("变更").unwrap().is_empty());
        fs::remove_dir_all(&目录).unwrap();
    }
}
