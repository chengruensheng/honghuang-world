use std::sync::{Arc, Mutex};
use hm_contract::Component;
use hm_error::{Error, Result};
use hm_execute_contract::执行器;
use tc_task::{扫尾检查, 扫尾记录, TaskBoard, 漂移检测, 解析快照};

/// 扫尾执行者：组合真实执行器与看板，对任务执行扫尾检查并写入交付证据链。
///
/// 流程：取任务实现文档声明的代码变更路径 → 执行器 `按名找文件("**/*")` 采集工作区快照 →
/// 漂移检测（声明 × 实际）→ 扫尾检查（含自检）→ 写回任务 `扫尾记录` 并返回。
pub struct 扫尾执行者 {
    执行器: Arc<dyn 执行器>,
    看板: Arc<Mutex<TaskBoard>>,
}

impl 扫尾执行者 {
    pub fn 新(执行器: Arc<dyn 执行器>, 看板: Arc<Mutex<TaskBoard>>) -> Self {
        扫尾执行者 { 执行器, 看板 }
    }

    /// 对指定任务执行扫尾检查。
    ///
    /// 错误分类：任务不存在 → `Error::任务不存在`；任务无实现文档 → `Error::Config`。
    pub fn 执行(&self, 任务id: u64) -> Result<扫尾记录> {
        let 标识 = {
            let 看板 = self.看板.lock().map_err(锁中毒)?;
            let 任务 = 看板
                .查询(任务id)
                .ok_or_else(|| Error::任务不存在(任务id))?;
            任务.任务标识.任务id
        };

        let 快照 = self.执行器.按名找文件("**/*")?;
        let 实际 = 解析快照(&快照);

        // 在锁内读取声明+自检、计算并写回（原子完成，避免并发改写撕裂）
        let record = {
            let mut 看板 = self.看板.lock().map_err(锁中毒)?;
            let 任务 = 看板
                .查询(任务id)
                .ok_or_else(|| Error::任务不存在(任务id))?;
            let 实现 = 任务
                .实现文档
                .as_ref()
                .ok_or_else(|| Error::Config(format!("任务 {任务id} 无实现文档，无法扫尾")))?;
            let 声明: Vec<String> = 实现.代码变更.iter().map(|c| c.文件路径.clone()).collect();
            let 自检通过 = 实现.自检.通过;
            let 报告 = 漂移检测(&声明, &实际);
            let 记录 = 扫尾检查(hm_contract::当前时间戳(), 声明.len(), 自检通过, &报告);
            let 写回记录 = 记录.clone();

            看板.按标识改写(&标识, move |t| t.扫尾记录 = Some(写回记录))?;
            记录
        };
        Ok(record)
    }
}

fn 锁中毒(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, TaskBoard>>,
) -> Error {
    Error::Other("看板锁中毒".into())
}

impl Component for 扫尾执行者 {
    fn name(&self) -> &'static str { "扫尾执行者" }
}

