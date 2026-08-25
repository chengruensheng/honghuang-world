//! 登记 - 落册 - 园：角色卡登记落册，供工作流引擎按职司调取。
//! 阶段 3 角色插件化（融合蓝图 §14.10）：升级为注册器——生命周期状态机（登记→就绪→生效→卸载）、
//! 副作用可逆注册/撤销（对齐 Cordis disposer）、在途任务软保护。

use crate::类型_定义_殿::{执行角色, 角色状态};
use rizhi_fu::{debug, error, info, warn};
use shihai_fu::世界结果;
use std::collections::HashMap;

/// 角色副作用：生效时注册、卸载时撤销（可逆，对齐 Cordis disposer）。
/// 回调用 `Arc<dyn Fn>` 包裹——副作用可 Clone（条目/角色册可 Clone），语义与 Box 一致。
#[derive(Clone)]
pub struct 角色副作用 {
    /// 生效时执行的注册动作（提示词片段/工具 schema 挂载）。
    注册: std::sync::Arc<dyn Fn() -> 世界结果<()> + Send + Sync>,
    /// 卸载时执行的撤销动作。
    撤销: std::sync::Arc<dyn Fn() -> 世界结果<()> + Send + Sync>,
}

impl 角色副作用 {
    /// 构造副作用（注册 + 撤销 成对，保证可逆）。
    pub fn 新(
        注册: impl Fn() -> 世界结果<()> + Send + Sync + 'static,
        撤销: impl Fn() -> 世界结果<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            注册: std::sync::Arc::new(注册),
            撤销: std::sync::Arc::new(撤销),
        }
    }

    /// 执行注册动作。
    fn 生效(&self) -> 世界结果<()> {
        (self.注册)()
    }

    /// 执行撤销动作。
    fn 撤销(&self) -> 世界结果<()> {
        (self.撤销)()
    }
}

/// 登记条目：角色卡 + 生命周期状态 + 可选副作用。
#[derive(Clone)]
pub struct 登记条目 {
    /// 角色卡（不可变，变更走 登记 覆盖）。
    pub 角色: 执行角色,
    /// 生命周期状态（登记→就绪→生效→卸载）。
    pub 状态: 角色状态,
    /// 副作用（生效时注册、卸载时撤销）；None = 无副作用角色。
    pub 副作用: Option<角色副作用>,
}

impl 登记条目 {
    fn 新(角色: 执行角色) -> Self {
        Self {
            角色,
            状态: 角色状态::已登记,
            副作用: None,
        }
    }
}

/// 角色册：身份 → 登记条目（注册器）。
#[derive(Default, Clone)]
pub struct 角色册 {
    角色们: HashMap<String, 登记条目>,
    在途数们: HashMap<String, usize>,
}

impl 角色册 {
    /// 空册。
    pub fn 新() -> 角色册 {
        角色册::default()
    }

    /// 登记一张角色卡（同身份覆盖为 已登记，副作用随旧条目丢弃；登记即「声明」）。
    pub fn 登记(&mut self, 角色: 执行角色) {
        debug!(身份 = %角色.身份, "角色已登记");
        self.角色们.insert(角色.身份.clone(), 登记条目::新(角色));
    }

    /// 就绪校验：依赖（模型池）可用 → 已就绪；缺失 → 留在 已登记 并返回缺失池。
    /// 对齐 Cordis inject 激活：声明依赖就绪后才可进入下一态。
    pub fn 就绪(&mut self, 身份: &str, 可用模型池: &[&str]) -> 世界结果<()> {
        let Some(条目) = self.角色们.get_mut(身份) else {
            return Err(format!("角色「{身份}」未登记").into());
        };
        let 池 = &条目.角色.模型池;
        if 池.is_empty() {
            条目.状态 = 角色状态::已就绪;
            debug!(身份, "无依赖角色直接就绪");
            return Ok(());
        }
        if 可用模型池.iter().any(|可用| 可用 == 池) {
            条目.状态 = 角色状态::已就绪;
            debug!(身份, 模型池 = %池, "角色依赖已就绪");
            Ok(())
        } else {
            Err(format!(
                "角色「{身份}」依赖模型池「{池}」不可用（可用：{}）",
                可用模型池.join("/")
            )
            .into())
        }
    }

    /// 生效：注册副作用 → 已生效（重复生效报错）。无副作用直接生效。
    /// 对齐 Cordis apply(ctx)：副作用注册是可逆的（卸载时撤销）。
    pub fn 生效(&mut self, 身份: &str, 副作用: 角色副作用) -> 世界结果<()> {
        let Some(条目) = self.角色们.get_mut(身份) else {
            return Err(format!("角色「{身份}」未登记").into());
        };
        if 条目.状态 == 角色状态::已生效 {
            return Err(format!("角色「{身份}」已生效，重复生效").into());
        }
        副作用.生效()?;
        条目.副作用 = Some(副作用);
        条目.状态 = 角色状态::已生效;
        info!(身份, "角色已生效（副作用已注册）");
        Ok(())
    }

    /// 卸载一张角色卡（生命周期：卸载）。在途任务未清 → 拒绝（软保护，防变更影响在途）。
    /// 副作用已注册则先撤销（可逆副作用），再移除条目。
    pub fn 卸载(&mut self, 身份: &str) -> 世界结果<执行角色> {
        if self.在途数(身份) > 0 {
            return Err(format!(
                "角色「{身份}」仍有 {} 个在途任务，拒绝卸载（防变更影响在途）",
                self.在途数(身份)
            )
            .into());
        }
        let Some(条目) = self.角色们.remove(身份) else {
            return Err(format!("角色「{身份}」未登记").into());
        };
        if let Some(副作用) = &条目.副作用 {
            副作用.撤销().map_err(|说明| {
                warn!(身份, 说明 = %说明, "角色副作用撤销失败（条目已移除）");
                说明
            })?;
        }
        info!(身份, "角色已卸载（副作用已撤销）");
        Ok(条目.角色)
    }

    /// 依赖校验：角色声明的模型池不在可用池中则返回缺失的池名（兼容旧接口，内部走 就绪）。
    /// 未声明依赖（模型池为空）视为无依赖，返回 None。
    pub fn 缺失模型池(&self, 身份: &str, 可用模型池: &[&str]) -> Option<String> {
        let 角色 = self.角色们.get(身份)?.角色.clone();
        if 角色.模型池.is_empty() {
            return None;
        }
        if 可用模型池.iter().any(|池| 池 == &角色.模型池) {
            None
        } else {
            Some(角色.模型池.clone())
        }
    }

    /// 按身份取角色卡。
    pub fn 取(&self, 身份: &str) -> Option<&执行角色> {
        self.角色们.get(身份).map(|条目| &条目.角色)
    }

    /// 取角色生命周期状态。
    pub fn 取状态(&self, 身份: &str) -> Option<角色状态> {
        self.角色们.get(身份).map(|条目| 条目.状态.clone())
    }

    /// 全部登记条目（观测用：身份 + 状态）。
    pub fn 全部条目(&self) -> Vec<(&str, &角色状态)> {
        self.角色们
            .iter()
            .map(|(身份, 条目)| (身份.as_str(), &条目.状态))
            .collect()
    }

    /// 在途登记：任务派遣时登记（软保护计数）。
    pub fn 在途登记(&mut self, 身份: &str) {
        *self.在途数们.entry(身份.to_string()).or_insert(0) += 1;
    }

    /// 在途清除：任务回执落定时清除。
    pub fn 在途清除(&mut self, 身份: &str) {
        if let Some(数) = self.在途数们.get_mut(身份) {
            if *数 > 0 {
                *数 -= 1;
            }
        }
    }

    /// 在途任务数（卸载软保护依据）。
    pub fn 在途数(&self, 身份: &str) -> usize {
        self.在途数们.get(身份).copied().unwrap_or(0)
    }

    /// 全部身份清单。
    pub fn 全部身份(&self) -> Vec<&String> {
        self.角色们.keys().collect()
    }

    /// 角色数量。
    pub fn 数量(&self) -> usize {
        self.角色们.len()
    }

    /// 保存到 json 文件（只存角色卡；状态/副作用/在途为运行时态，不落盘）。
    pub fn 保存(&self, 路径: &str) -> 世界结果<()> {
        let 卡们: HashMap<String, 执行角色> = self
            .角色们
            .iter()
            .map(|(身份, 条目)| (身份.clone(), 条目.角色.clone()))
            .collect();
        let 文本 = serde_json::to_string_pretty(&卡们)
            .map_err(|错误| format!("序列化角色册失败: {错误}"))?;
        std::fs::write(路径, 文本).map_err(|错误| {
            error!(路径, "保存角色册失败：{错误}");
            format!("保存角色册失败: {错误}")
        })?;
        debug!(路径, "角色册已保存");
        Ok(())
    }

    /// 从 json 文件加载（条目状态初始化为 已登记，待 就绪/生效）。
    pub fn 加载(路径: &str) -> 世界结果<角色册> {
        let 文本 = std::fs::read_to_string(路径).map_err(|错误| {
            error!(路径, "读取角色册失败：{错误}");
            format!("读取角色册失败: {错误}")
        })?;
        let 卡们: HashMap<String, 执行角色> =
            serde_json::from_str(&文本).map_err(|错误| format!("解析角色册失败: {错误}"))?;
        info!(路径, 角色数 = 卡们.len(), "角色册已加载");
        let 角色们 = 卡们
            .into_iter()
            .map(|(身份, 角色)| (身份, 登记条目::新(角色)))
            .collect();
        Ok(角色册 {
            角色们,
            在途数们: HashMap::new(),
        })
    }
}

/// 全局角色册：进程级单例（static OnceLock），供 派发落单 在途登记 与 角色卸载 同源共享——
/// 卸载保护查询的在途数 = 派发落单登记的在途数（同一把锁同一数据，防两套计数漂移）。
pub fn 全局角色册() -> &'static std::sync::Mutex<角色册> {
    static 全局: std::sync::OnceLock<std::sync::Mutex<角色册>> = std::sync::OnceLock::new();
    全局.get_or_init(|| std::sync::Mutex::new(角色册::新()))
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::类型_定义_殿::执行角色;

    fn 造角色(身份: &str, 模型池: &str) -> 执行角色 {
        执行角色 {
            身份: 身份.to_string(),
            道: "代码炼化".to_string(),
            职司: "代码".to_string(),
            模型池: 模型池.to_string(),
            契约: "写代码".to_string(),
        }
    }

    #[test]
    fn 卸载_登记后卸载即取不到() {
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝", "executor"));
        assert_eq!(册.数量(), 1);
        let 卸 = 册.卸载("多宝");
        assert!(卸.is_ok(), "应成功卸载：{卸:?}");
        assert_eq!(册.数量(), 0);
        assert!(册.取("多宝").is_none(), "卸载后取不到");
    }

    #[test]
    fn 缺失模型池_依赖校验() {
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝", "executor"));
        // 可用池含 executor → 无缺失。
        assert!(册.缺失模型池("多宝", &["executor", "sage"]).is_none());
        // 可用池不含 executor → 缺失 executor。
        assert_eq!(
            册.缺失模型池("多宝", &["sage"]).as_deref(),
            Some("executor")
        );
        // 未登记 → None（不误报缺失）。
        assert!(册.缺失模型池("不存在", &["executor"]).is_none());
    }

    #[test]
    fn 缺失模型池_无声明依赖视为无依赖() {
        let mut 册 = 角色册::新();
        册.登记(造角色("无名", ""));
        assert!(册.缺失模型池("无名", &[]).is_none(), "模型池为空视为无依赖");
    }

    /// 生命周期状态机：登记(已登记) → 就绪(依赖可用) → 生效(副作用注册) → 卸载(撤销+移除)。
    #[test]
    fn 生命周期_登记就绪生效卸载() {
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝", "executor"));
        assert_eq!(册.取状态("多宝"), Some(角色状态::已登记));

        // 就绪：依赖可用 → 已就绪。
        册.就绪("多宝", &["executor"]).unwrap();
        assert_eq!(册.取状态("多宝"), Some(角色状态::已就绪));

        // 生效：注册副作用 → 已生效。
        let 副作用 = 角色副作用::新(|| Ok(()), || Ok(()));
        册.生效("多宝", 副作用).unwrap();
        assert_eq!(册.取状态("多宝"), Some(角色状态::已生效));

        // 重复生效报错。
        let 重复副作用 = 角色副作用::新(|| Ok(()), || Ok(()));
        assert!(册.生效("多宝", 重复副作用).is_err(), "重复生效应报错");

        // 卸载：撤销副作用 + 移除。
        册.卸载("多宝").unwrap();
        assert!(册.取("多宝").is_none());
    }

    /// 依赖缺失：就绪 停在 已登记，返回缺失池名。
    #[test]
    fn 就绪_依赖缺失停已登记() {
        let mut 册 = 角色册::新();
        册.登记(造角色("女娲", "sage"));
        let 结果 = 册.就绪("女娲", &["executor"]);
        assert!(结果.is_err(), "依赖缺失应报错");
        assert!(结果.unwrap_err().to_string().contains("sage"));
        assert_eq!(
            册.取状态("女娲"),
            Some(角色状态::已登记),
            "依赖缺失应停在已登记"
        );
    }

    /// 副作用可逆：卸载时撤销动作被执行。
    #[test]
    fn 副作用_卸载时撤销执行() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝", "executor"));
        let 撤销数 = std::sync::Arc::new(AtomicUsize::new(0));
        let 撤销探针 = 撤销数.clone();
        let 副作用 = 角色副作用::新(
            || Ok(()),
            move || {
                撤销探针.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        );
        册.生效("多宝", 副作用).unwrap();
        册.卸载("多宝").unwrap();
        assert_eq!(撤销数.load(Ordering::SeqCst), 1, "卸载应执行撤销动作一次");
    }

    /// 在途保护：有在途任务时拒绝卸载。
    #[test]
    fn 在途_有任务拒绝卸载() {
        let mut 册 = 角色册::新();
        册.登记(造角色("多宝", "executor"));
        册.在途登记("多宝");
        let 结果 = 册.卸载("多宝");
        assert!(结果.is_err(), "在途任务未清应拒绝卸载");
        assert!(结果.unwrap_err().to_string().contains("在途"));
        assert!(册.取("多宝").is_some(), "拒绝后角色仍在册");
        // 在途清除后可卸载。
        册.在途清除("多宝");
        册.卸载("多宝").unwrap();
    }
}
