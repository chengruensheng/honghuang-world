//! 三档 - 拼装 - 园：最前/中间/最后三档投影拼装。

use crate::{格位, 顺序档位, 模型存储};

/// 拼装投影：最前 → 最后 → 中间（首因+近因优先），按预算字节截断。
pub fn 拼装投影(存储: &模型存储, 格位们: &[格位], 预算字节: usize) -> Result<String, String> {
    let mut 顺序: Vec<&格位> = Vec::new();
    顺序.extend(格位们.iter().filter(|格位| 格位.顺序档位 == 顺序档位::最前));
    顺序.extend(格位们.iter().filter(|格位| 格位.顺序档位 == 顺序档位::最后));
    顺序.extend(格位们.iter().filter(|格位| 格位.顺序档位 == 顺序档位::中间));

    let mut 输出 = String::new();
    for 格位 in 顺序 {
        for 记录 in 存储.读格位(&格位.名字)? {
            let 行 = format!("【{}】{}（证据：{}）\n", 格位.名字, 记录.内容, 记录.证据);
            if 输出.len() + 行.len() > 预算字节 {
                return Ok(输出);
            }
            输出.push_str(&行);
        }
    }
    Ok(输出)
}
