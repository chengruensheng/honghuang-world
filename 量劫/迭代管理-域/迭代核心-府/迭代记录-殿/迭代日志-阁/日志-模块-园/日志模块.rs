use std::collections::HashMap;
use std::sync::Arc;
use hm_contract::{Component, 当前时间戳};
use hm_error::{Error, Result};
use hm_signal::{信号总线, 信号类型, 信号载荷};
use hm_signal::引擎支撑;
use hm_domain_contract::迭代日志契约;
use serde::{Deserialize, Serialize};
use crate::版本定义_殿::Version;
use crate::迭代记录_殿::{Iteration, 迭代状态};

/// 容量上限默认值：不限（约束由装配层注入）。
/// 可移植写法：64 位平台可序列化 i64::MAX，32 位平台回退 usize::MAX 避免溢出。
fn 默认容量上限() -> usize {
    match usize::try_from(i64::MAX) {
        Ok(上限) => 上限,
        Err(_) => usize::MAX,
    }
}

/// 迭代日志：登记版本演进历史，追踪当前版本，支持容量约束与落盘持久化。
/// 以 Vec 保存迭代保证持久化顺序，另建 id 索引使按 id 查询 O(1)。
#[derive(Clone, Serialize, Deserialize)]
pub struct IterationLog {
    iterations: Vec<Iteration>,
    next_id: u64,
    #[serde(default = "默认容量上限")]
    容量上限: usize,
    #[serde(skip)]
    索引: HashMap<u64, usize>,
    #[serde(skip)]
    信号总线: Option<Arc<dyn 信号总线>>,
    #[serde(skip)]
    持久化路径: Option<String>,
}

impl IterationLog {
    pub fn new() -> Self {
        IterationLog {
            iterations: Vec::new(),
            next_id: 1,
            容量上限: 默认容量上限(),
            索引: HashMap::new(),
            信号总线: None,
            持久化路径: None,
        }
    }

    引擎支撑!();

    /// 设置进行中迭代容量上限（水克火：过盛时约束开启）
    pub fn 设置容量上限(&mut self, 上限: usize) {
        self.容量上限 = 上限;
    }

    /// 开启迭代（火之变革开始），返回迭代 id；进行中迭代过盛时拒绝
    pub fn 开启(&mut self, version: Version, 变更说明: String) -> Result<u64> {
        let 进行中数 = self
            .iterations
            .iter()
            .filter(|it| it.status == 迭代状态::进行中)
            .count();
        if 进行中数 >= self.容量上限 {
            return Err(Error::容量超限(format!("进行中迭代已达上限 {}", self.容量上限)));
        }
        let id = self.next_id;
        self.next_id += 1;
        let it = Iteration::新建(id, version, 变更说明, 当前时间戳());
        self.索引.insert(id, self.iterations.len());
        self.iterations.push(it);
        self.自动保存();
        Ok(id)
    }

    /// 完成迭代（焚炼完成，重生为已完成）
    pub fn 完成(&mut self, id: u64) -> Result<()> {
        let 下标 = *self.索引.get(&id).ok_or_else(|| Error::迭代不存在(id))?;
        let it = &mut self.iterations[下标];
        if it.status != 迭代状态::进行中 {
            return Err(Error::状态流转非法(format!("迭代 {id}: {:?}", it.status)));
        }
        let version = it.version;
        let 说明 = it.变更说明.clone();
        it.status = 迭代状态::已完成;
        self.发布信号(
            信号类型::迭代完成,
            信号载荷 {
                版本: Some(version.to_string()),
                变更说明: Some(说明),
                ..信号载荷::default()
            },
        );
        self.自动保存();
        Ok(())
    }

    /// 放弃迭代
    pub fn 放弃(&mut self, id: u64) -> Result<()> {
        let 下标 = *self.索引.get(&id).ok_or_else(|| Error::迭代不存在(id))?;
        let it = &mut self.iterations[下标];
        if it.status != 迭代状态::进行中 {
            return Err(Error::状态流转非法(format!("迭代 {id}: {:?}", it.status)));
        }
        let version = it.version;
        let 说明 = it.变更说明.clone();
        it.status = 迭代状态::已放弃;
        self.发布信号(
            信号类型::迭代放弃,
            信号载荷 {
                版本: Some(version.to_string()),
                变更说明: Some(说明),
                ..信号载荷::default()
            },
        );
        self.自动保存();
        Ok(())
    }

    /// 按 id 查询迭代
    pub fn 查询(&self, id: u64) -> Option<&Iteration> {
        self.索引.get(&id).map(|&i| &self.iterations[i])
    }

    /// 演进历史（全部迭代）
    pub fn 全部(&self) -> Vec<&Iteration> {
        self.iterations.iter().collect()
    }

    /// 当前版本：按创建时间最新的已完成迭代的版本；无则返回 0.0.0。
    /// 创建时间相同时以版本号为兜底排序，保证结果确定。
    pub fn 当前版本(&self) -> Version {
        self.iterations
            .iter()
            .filter(|it| it.status == 迭代状态::已完成)
            .max_by_key(|it| (it.created_at, it.version))
            .map(|it| it.version)
            .unwrap_or(Version::new(0, 0, 0))
    }

    /// 保存到文件（落盘）
    pub fn 保存(&self, path: &str) -> Result<()> {
        let content = toml::to_string(self)
            .map_err(|e| Error::序列化(format!("序列化迭代失败: {e}")))?;
        hm_contract::原子写入文件(path, &content)?;
        Ok(())
    }

    /// 从文件加载（还原），信号总线需重新注入
    pub fn 加载(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(Error::Io)?;
        let mut log: IterationLog = toml::from_str(&content)
            .map_err(|e| Error::反序列化(format!("解析迭代文件失败: {e}")))?;
        log.重建索引();
        Ok(log)
    }

    /// 重建 id 索引（加载后调用）
    fn 重建索引(&mut self) {
        self.索引.clear();
        for (i, it) in self.iterations.iter().enumerate() {
            self.索引.insert(it.id, i);
        }
    }

}

impl Component for IterationLog {
    fn name(&self) -> &'static str { "迭代日志" }
}

impl 迭代日志契约<Iteration, Version> for IterationLog {
    fn 开启(&mut self, 版本: Version, 变更说明: String) -> Result<u64> {
        IterationLog::开启(self, 版本, 变更说明)
    }

    fn 完成(&mut self, id: u64) -> Result<()> {
        IterationLog::完成(self, id)
    }

    fn 放弃(&mut self, id: u64) -> Result<()> {
        IterationLog::放弃(self, id)
    }

    fn 查询(&self, id: u64) -> Option<&Iteration> {
        IterationLog::查询(self, id)
    }

    fn 全部(&self) -> Vec<&Iteration> {
        IterationLog::全部(self)
    }

    fn 当前版本(&self) -> Version {
        IterationLog::当前版本(self)
    }

}
