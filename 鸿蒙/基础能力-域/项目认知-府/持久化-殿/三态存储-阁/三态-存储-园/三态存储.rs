use std::fs;
use std::path::{Path, PathBuf};
use hm_error::{Error, Result};
use crate::{上下文库, 上下文消息, 图谱, 心智地图, 纠错事件};

/// 心智地图落盘文件名
const 心智文件名: &str = "心智地图.json";
/// 图谱落盘文件名
const 图谱文件名: &str = "图谱.json";
/// 上下文库落盘文件名（JSONL：每条 上下文消息 一行）
const 上下文文件名: &str = "上下文库.jsonl";
/// 纠错事件日志文件名（JSONL：只追加）
const 纠错事件文件名: &str = "纠错事件.jsonl";
/// 上下文库加载后的硬上限（与 上下文库::默认 一致）
const 上下文加载上限: usize = 1000;

/// 三态存储：心智地图/图谱/上下文库 的写穿落盘（persistence.dir/认知三态/）。
///
/// - 心智地图/图谱：整体 JSON，原子写（临时文件 + rename），防半写；
/// - 上下文库：JSONL 全量重写（保存）→ 逐行解析 + 截断上限（加载）；
/// - 纠错事件：JSONL 只追加（审计日志，不回写）。
/// 文件损坏或缺失：加载对应文件返回 Err（坏行跳过容忍日志污染），由调用方决定保持空态。
pub struct 三态存储 {
    目录: PathBuf,
}

impl 三态存储 {
    /// 创建存储句柄（目录不存在自动创建）
    pub fn 新(目录: impl Into<PathBuf>) -> Result<Self> {
        let 目录 = 目录.into();
        fs::create_dir_all(&目录)?;
        Ok(三态存储 { 目录 })
    }

    /// 目录路径（供日志/诊断）
    pub fn 目录(&self) -> &Path {
        &self.目录
    }

    /// 任一三态文件存在（作为「历史三态可恢复」的判断）
    pub fn 已存在(&self) -> bool {
        [心智文件名, 图谱文件名, 上下文文件名]
            .iter()
            .any(|名| self.目录.join(名).exists())
    }

    /// 一键保存三态（心智 + 图谱 + 上下文）
    pub fn 保存(&self, 图谱: &图谱, 心智: &心智地图, 上下文: &上下文库) -> Result<()> {
        self.保存心智(心智)?;
        self.保存图谱(图谱)?;
        self.保存上下文(上下文)
    }

    /// 一键加载三态（任一文件缺失/损坏即返回 Err，由调用方决定保持空态）
    pub fn 加载(&self) -> Result<(图谱, 心智地图, 上下文库)> {
        Ok((self.加载图谱()?, self.加载心智()?, self.加载上下文()?))
    }

    /// 保存心智地图（整体 JSON，原子写）
    pub fn 保存心智(&self, 心智: &心智地图) -> Result<()> {
        let 内容 = serde_json::to_string_pretty(心智).map_err(|e| Error::序列化(e.to_string()))?;
        self.原子写(心智文件名, &内容)
    }

    /// 保存图谱（整体 JSON，原子写）
    pub fn 保存图谱(&self, 图谱: &图谱) -> Result<()> {
        let 内容 = serde_json::to_string_pretty(图谱).map_err(|e| Error::序列化(e.to_string()))?;
        self.原子写(图谱文件名, &内容)
    }

    /// 保存上下文库（JSONL 全量重写，原子写）
    pub fn 保存上下文(&self, 上下文: &上下文库) -> Result<()> {
        let mut 内容 = String::new();
        for 消息 in 上下文.全部() {
            内容.push_str(&serde_json::to_string(消息).map_err(|e| Error::序列化(e.to_string()))?);
            内容.push('\n');
        }
        self.原子写(上下文文件名, &内容)
    }

    /// 加载心智地图（文件缺失/损坏 → Err）
    pub fn 加载心智(&self) -> Result<心智地图> {
        let 文本 = fs::read_to_string(self.目录.join(心智文件名))?;
        serde_json::from_str(&文本).map_err(|e| Error::反序列化(e.to_string()))
    }

    /// 加载图谱（文件缺失/损坏 → Err）
    pub fn 加载图谱(&self) -> Result<图谱> {
        let 文本 = fs::read_to_string(self.目录.join(图谱文件名))?;
        serde_json::from_str(&文本).map_err(|e| Error::反序列化(e.to_string()))
    }

    /// 加载上下文库（JSONL 逐行解析，坏行跳过容忍日志污染；截断到硬上限）
    pub fn 加载上下文(&self) -> Result<上下文库> {
        let 文本 = fs::read_to_string(self.目录.join(上下文文件名))?;
        let mut 消息流: Vec<上下文消息> = Vec::new();
        for 行 in 文本.lines() {
            let 行 = 行.trim();
            if 行.is_empty() {
                continue;
            }
            if let Ok(消息) = serde_json::from_str::<上下文消息>(行) {
                消息流.push(消息);
            }
        }
        Ok(上下文库::导入(消息流, 上下文加载上限))
    }

    /// 追加一条纠错事件（JSONL 只追加，审计日志不回写）
    pub fn 追加纠错事件(&self, 事件: &纠错事件) -> Result<()> {
        let mut 行 = serde_json::to_string(事件).map_err(|e| Error::序列化(e.to_string()))?;
        行.push('\n');
        use std::io::Write;
        let mut 文件 = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.目录.join(纠错事件文件名))?;
        文件.write_all(行.as_bytes())?;
        Ok(())
    }

    /// 加载全部纠错事件（坏行跳过）
    pub fn 加载纠错事件(&self) -> Result<Vec<纠错事件>> {
        let 路径 = self.目录.join(纠错事件文件名);
        if !路径.exists() {
            return Ok(Vec::new());
        }
        let 文本 = fs::read_to_string(路径)?;
        let mut 事件集 = Vec::new();
        for 行 in 文本.lines() {
            let 行 = 行.trim();
            if 行.is_empty() {
                continue;
            }
            if let Ok(事件) = serde_json::from_str::<纠错事件>(行) {
                事件集.push(事件);
            }
        }
        Ok(事件集)
    }

    /// 原子写：临时文件 + rename（Windows rename 覆盖已存在目标）
    fn 原子写(&self, 文件名: &str, 内容: &str) -> Result<()> {
        let 目标 = self.目录.join(文件名);
        let 临时 = self.目录.join(format!("{文件名}.tmp"));
        fs::write(&临时, 内容)?;
        fs::rename(&临时, &目标)?;
        Ok(())
    }
}
