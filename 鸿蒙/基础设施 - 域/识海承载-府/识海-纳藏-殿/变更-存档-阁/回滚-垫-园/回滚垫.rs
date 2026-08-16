//! 回滚 - 垫 - 园：写操作落盘前的存档，供任务失败时单文件撤销。
//!
//! 职责：写/改/删 在落盘前把旧内容（或「曾不存在」标记）存进
//! `.上下文/回滚垫/{任务id}/`，任务失败时按存档恢复盘面，成功则清理丢弃。
//! 存档按任务id分组；线程本地记录当前任务，并发派遣各线程独立。
//!
//! 同一路径只备份首次（保留写前原始状态），多次写同一文件撤销时恢复最原始内容。

use crate::当前毫秒;
use rizhi_fu::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

/// 单份存档：文件路径 + 写前是否曾存在 + 旧内容。
#[derive(Serialize, Deserialize)]
pub struct 回滚条目 {
    pub 路径: String,
    pub 曾存在: bool,
    pub 内容: String,
    pub 时间戳: u64,
}

// 当前任务id（线程本地）：写入/撤销不传任务id时的归属。
thread_local! {
    #[allow(non_upper_case_globals)]
    static 当前任务id: RefCell<Option<String>> = RefCell::new(None);
}

/// 任务守卫：进入派遣时设置当前任务id，作用域结束自动清除。
pub struct 任务守卫;

impl Drop for 任务守卫 {
    fn drop(&mut self) {
        当前任务id.with(|槽| *槽.borrow_mut() = None);
    }
}

/// 进入任务：设置线程当前任务id，返回守卫（Drop 时自动清除）。
pub fn 进入任务(任务id: &str) -> 任务守卫 {
    当前任务id.with(|槽| *槽.borrow_mut() = Some(任务id.to_string()));
    任务守卫
}

/// 当前任务id（未进入任务则为空串，落在「全局」分组）。
pub fn 当前任务() -> String {
    当前任务id.with(|槽| 槽.borrow().clone()).unwrap_or_default()
}

/// 回滚垫：存档目录 = 工作区 `.上下文/回滚垫`。
pub struct 回滚垫 {
    目录: PathBuf,
}

impl 回滚垫 {
    /// 打开（目录不存在则创建）。
    pub fn 打开(目录: impl AsRef<Path>) -> 回滚垫 {
        let 目录 = 目录.as_ref().to_path_buf();
        let _ = fs::create_dir_all(&目录);
        回滚垫 { 目录 }
    }

    /// 在工作区根下打开（落 `.上下文/回滚垫/`）。
    pub fn 在工作区(工作区: &crate::工作区) -> 回滚垫 {
        回滚垫::打开(工作区.上下文目录().join("回滚垫"))
    }

    /// 写前备份：文件存在则存旧内容，不存在则存删除标记。
    /// 只备份工作区内的文件（外部路径不归档，防临时/测试文件污染）；
    /// 同一路径只备份首次；备份失败只警告不阻断写入（主流程优先）。
    pub fn 备份(&self, 任务id: &str, 路径: &str) -> Result<(), String> {
        let 任务id = 分组(&任务id);
        let 任务目录 = self.目录.join(任务id);
        let 存档 = 任务目录.join(format!("{}.json", 散列(路径)));
        if 存档.exists() {
            return Ok(());
        }
        // 回滚垫服务其所在工作区（.上下文/回滚垫 的父父级为工作区根）：根外路径不归档。
        let 工作区根 = self.目录.parent().and_then(|父| 父.parent());
        if let Some(根) = 工作区根 {
            if !Path::new(路径).starts_with(根) {
                return Ok(());
            }
        }
        let 绝对 = Path::new(路径);
        let (曾存在, 内容) = if 绝对.is_file() {
            match fs::read_to_string(绝对) {
                Ok(内容) => (true, 内容),
                Err(错误) => {
                    warn!(任务id, 路径, "回滚垫跳过：非文本或读失败：{错误}");
                    return Ok(());
                }
            }
        } else {
            (false, String::new())
        };
        fs::create_dir_all(&任务目录).map_err(|错误| format!("创建回滚存档目录失败：{错误}"))?;
        let 条目 = 回滚条目 {
            路径: 路径.to_string(),
            曾存在,
            内容,
            时间戳: 当前毫秒(),
        };
        let 文本 = serde_json::to_string(&条目).map_err(|错误| format!("序列化回滚条目失败：{错误}"))?;
        fs::write(&存档, 文本).map_err(|错误| {
            error!(任务id, 路径, "写回滚存档失败：{错误}");
            format!("写回滚存档失败：{错误}")
        })?;
        debug!(任务id, 路径, "写前已备份");
        Ok(())
    }

    /// 撤销：按时间戳恢复该任务全部写前状态（曾存在→恢复旧内容；曾不存在→删除），返回恢复数。
    pub fn 撤销(&self, 任务id: &str) -> Result<u32, String> {
        let 任务id = 分组(&任务id);
        let 任务目录 = self.目录.join(任务id);
        if !任务目录.is_dir() {
            return Ok(0);
        }
        let mut 条目们 = Vec::new();
        for 条目 in fs::read_dir(&任务目录).map_err(|错误| format!("读回滚存档目录失败：{错误}"))? {
            let 路径 = 条目.map_err(|错误| format!("读回滚存档目录项失败：{错误}"))?.path();
            if 路径.extension().map(|末| 末 != "json").unwrap_or(true) {
                continue;
            }
            let 文本 = fs::read_to_string(&路径).map_err(|错误| format!("读回滚存档失败：{错误}"))?;
            let 条目: 回滚条目 = serde_json::from_str(&文本).map_err(|错误| format!("解析回滚存档失败：{错误}"))?;
            条目们.push(条目);
        }
        条目们.sort_by_key(|条目| 条目.时间戳);
        let mut 恢复数 = 0u32;
        for 条目 in &条目们 {
            let 绝对 = Path::new(&条目.路径);
            if 条目.曾存在 {
                if let Some(父) = 绝对.parent() {
                    let _ = fs::create_dir_all(父);
                }
                fs::write(绝对, &条目.内容).map_err(|错误| {
                    error!(任务id, 路径 = %条目.路径, "撤销恢复失败：{错误}");
                    format!("撤销恢复失败：{}：{错误}", 条目.路径)
                })?;
            } else if 绝对.exists() {
                fs::remove_file(绝对).map_err(|错误| {
                    error!(任务id, 路径 = %条目.路径, "撤销删除失败：{错误}");
                    format!("撤销删除失败：{}：{错误}", 条目.路径)
                })?;
            }
            恢复数 += 1;
        }
        info!(任务id, 恢复数, "回滚垫已恢复该任务全部写前状态");
        Ok(恢复数)
    }

    /// 撤销当前任务（用线程当前任务id）。
    pub fn 撤销当前(&self) -> Result<u32, String> {
        self.撤销(&当前任务())
    }

    /// 清理：任务成功结束后丢弃存档。
    pub fn 清理(&self, 任务id: &str) -> Result<(), String> {
        let 任务id = 分组(&任务id);
        let 任务目录 = self.目录.join(任务id);
        if 任务目录.is_dir() {
            fs::remove_dir_all(&任务目录).map_err(|错误| format!("清理回滚存档失败：{错误}"))?;
            debug!(任务id, "回滚存档已清理");
        }
        Ok(())
    }

    /// 清理当前任务。
    pub fn 清理当前(&self) -> Result<(), String> {
        self.清理(&当前任务())
    }

    /// 撤销全部任务：遍历所有任务目录，逐个恢复写前状态，返回总恢复数。
    /// 供「打回撤销」用：回滚垫记录的是世界写前状态（含未存档的界主/助手改动），
    /// 比版本快照更精确——版本快照不含未存档改动，按快照恢复会覆盖它们。
    pub fn 撤销全部(&self) -> Result<u32, String> {
        if !self.目录.is_dir() {
            return Ok(0);
        }
        let 任务们: Vec<String> = fs::read_dir(&self.目录)
            .map_err(|错误| format!("读回滚垫目录失败：{错误}"))?
            .filter_map(|条目| 条目.ok())
            .filter(|条目| 条目.path().is_dir())
            .filter_map(|条目| 条目.file_name().to_str().map(|名| 名.to_string()))
            .collect();
        let mut 总恢复 = 0u32;
        for 任务 in 任务们 {
            总恢复 += self.撤销(&任务)?;
        }
        Ok(总恢复)
    }

    /// 清理全部任务：定档成功后丢弃所有存档（产物已入库，不再需要回滚）。
    pub fn 清理全部(&self) -> Result<(), String> {
        if !self.目录.is_dir() {
            return Ok(());
        }
        for 条目 in fs::read_dir(&self.目录).map_err(|错误| format!("读回滚垫目录失败：{错误}"))? {
            let 条目 = 条目.map_err(|错误| format!("读回滚垫目录项失败：{错误}"))?;
            if 条目.path().is_dir() {
                fs::remove_dir_all(条目.path()).map_err(|错误| format!("清理回滚存档失败：{错误}"))?;
            }
        }
        Ok(())
    }
}

/// 路径散列：路径含分隔符不能直接作文件名，散成短文件名。
fn 散列(路径: &str) -> String {
    let 值 = 路径
        .as_bytes()
        .iter()
        .fold(0u64, |累, &字节| 累.wrapping_mul(31).wrapping_add(字节 as u64));
    format!("{值:016x}")
}

/// 空任务id 归一为「全局」分组，防空串 join 退化为父目录误伤。
fn 分组(任务id: &str) -> &str {
    if 任务id.is_empty() {
        "全局"
    } else {
        任务id
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 临时工作区(名: &str) -> crate::工作区 {
        let 根 = std::env::temp_dir().join(format!("回滚垫-{名}-{}", 当前毫秒()));
        let 工作区 = crate::工作区::新(&根);
        工作区.初始化().unwrap();
        工作区
    }

    #[test]
    fn 撤销全部_恢复所有任务写前状态() {
        let 工作区 = 临时工作区("撤销全部");
        let 垫 = 回滚垫::在工作区(&工作区);
        let 文件1 = 工作区.根路径().join("任务1文件.txt");
        fs::write(&文件1, "旧1").unwrap();
        垫.备份("任务1", 文件1.to_str().unwrap()).unwrap();
        fs::write(&文件1, "新1").unwrap();
        let 文件2 = 工作区.根路径().join("任务2文件.txt");
        fs::write(&文件2, "旧2").unwrap();
        垫.备份("任务2", 文件2.to_str().unwrap()).unwrap();
        fs::write(&文件2, "新2").unwrap();

        let 恢复 = 垫.撤销全部().unwrap();
        assert_eq!(恢复, 2, "应恢复两个任务的写前状态");
        assert_eq!(fs::read_to_string(&文件1).unwrap(), "旧1");
        assert_eq!(fs::read_to_string(&文件2).unwrap(), "旧2");
        let _ = fs::remove_dir_all(工作区.根路径());
    }

    #[test]
    fn 清理全部_清空所有任务存档() {
        let 工作区 = 临时工作区("清理全部");
        let 垫 = 回滚垫::在工作区(&工作区);
        let 文件 = 工作区.根路径().join("文件.txt");
        fs::write(&文件, "旧").unwrap();
        垫.备份("任务1", 文件.to_str().unwrap()).unwrap();
        垫.清理全部().unwrap();
        let 垫目录 = 工作区.上下文目录().join("回滚垫");
        let 剩余 = fs::read_dir(&垫目录).map(|迭代| 迭代.count()).unwrap_or(0);
        assert_eq!(剩余, 0, "清理全部后任务目录应清空");
        let _ = fs::remove_dir_all(工作区.根路径());
    }
}
