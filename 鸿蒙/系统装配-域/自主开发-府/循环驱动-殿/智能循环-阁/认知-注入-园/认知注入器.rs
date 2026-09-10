use std::sync::{Arc, Mutex};
use hm_content_contract::{工具对话器, 对话消息};
use hm_cognition::{消息角色, 注入器, 上下文库, 三态存储, 图谱, 心智地图, 检索器, 检索源, 检索决策记录, 维度, 压缩器, 纠错引擎, 上下文压缩阈值};

/// 认知注入：三态（图谱/格位/临时上下文）接入智能体运行上下文的装配句柄。
///
/// 与 hm-cognition 的 注入器 不同：本结构持有共享所有权句柄（Arc<Mutex>），
/// 供 hm-agent 智能体在每次决策前 推（格位常驻）/拉（图谱按需）/流（最近过程）
/// 拼装注入段，并把每轮过程记录回临时态上下文库——三态从"机制"进入"养智能体"闭环。
///
/// 锁顺序固定：图谱 → 心智 → 上下文（短锁，不跨锁调用，避免死锁）。
#[derive(Clone)]
pub struct 认知注入 {
    图谱: Arc<Mutex<图谱>>,
    心智: Arc<Mutex<心智地图>>,
    上下文: Arc<Mutex<上下文库>>,
    /// 三态持久化存储（可选）：装配后每轮对话结束自动 保存 三态写穿落盘
    存储: Option<Arc<三态存储>>,
    /// 临时规则（流态第三态）：任务级规则，任务执行期间注入初始上下文，任务终态后清除
    临时规则们: Arc<Mutex<Vec<String>>>,
}

impl 认知注入 {
    /// 由数据服务/启动装配持有的三态共享句柄构造
    pub fn 新(
        图谱: Arc<Mutex<图谱>>,
        心智: Arc<Mutex<心智地图>>,
        上下文: Arc<Mutex<上下文库>>,
    ) -> Self {
        认知注入 { 图谱, 心智, 上下文, 存储: None, 临时规则们: Arc::new(Mutex::new(Vec::new())) }
    }

    /// 注入临时规则（流态第三态）：任务承接时调用，替换为当前任务的规则集（幂等：同任务重复承接不叠加）
    pub fn 注入临时规则(&self, 规则们: Vec<String>) {
        let 清洗后: Vec<String> = 规则们.into_iter().map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect();
        if 清洗后.is_empty() {
            return;
        }
        *self.临时规则们.lock().expect("临时规则锁中毒") = 清洗后;
    }

    /// 清除临时规则：任务终态（清理完成）后调用，规则不外溢到后续任务
    pub fn 清除临时规则(&self) {
        self.临时规则们.lock().expect("临时规则锁中毒").clear();
    }

    /// 链式装配三态持久化存储：装配后 保存() 把 图谱/心智/上下文 写穿落盘
    pub fn 装配存储(mut self, 存储: Arc<三态存储>) -> Self {
        self.存储 = Some(存储);
        self
    }

    /// 保存三态写穿落盘（未装配存储时静默跳过）
    pub fn 保存(&self) {
        let Some(存储) = &self.存储 else {
            return;
        };
        let 图谱 = self.图谱.lock().expect("图谱锁中毒");
        let 心智 = self.心智.lock().expect("心智锁中毒");
        let 上下文 = self.上下文.lock().expect("上下文锁中毒");
        if let Err(失败) = 存储.保存(&图谱, &心智, &上下文) {
            tracing::warn!("三态持久化保存失败: {失败}");
        }
    }

    /// 初始注入：推（已填格位按可信度降序 ≤10）+ 拉（图谱按任务关键词 ≤5）。
    /// 三态均空时返回空串（调用方跳过插入）。
    pub fn 初始注入(&self, 任务: &str) -> String {
        let 图谱 = self.图谱.lock().expect("图谱锁中毒");
        let 心智 = self.心智.lock().expect("心智锁中毒");
        let 上下文 = self.上下文.lock().expect("上下文锁中毒");
        let 注入器 = 注入器::新(&图谱, &心智, &上下文);

        let mut 段: Vec<String> = Vec::new();
        let 推 = 注入器.推_格位摘要();
        if !推.is_empty() {
            段.push(format!(
                "【格位·常驻】\n{}",
                推.iter()
                    .map(|p| p.内容.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        let 拉 = 注入器.拉_图谱片段(任务, 5);
        if !拉.is_empty() {
            段.push(format!(
                "【图谱·按需】\n{}",
                拉.iter()
                    .map(|p| p.内容.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        // 流态第三态：临时任务规则（任务执行期间强制约束，任务终态后随清除不再现）
        let 临时规则 = self.临时规则们.lock().expect("临时规则锁中毒").clone();
        if !临时规则.is_empty() {
            段.push(format!(
                "【临时·任务规则】（本任务执行期间的强制约束，必须遵守）\n{}",
                临时规则
                    .iter()
                    .enumerate()
                    .map(|(i, r)| format!("{}. {r}", i + 1))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        段.join("\n\n")
    }

    /// 流注入：临时上下文最近 20 条（过程流）；空上下文返回空串。
    pub fn 流注入(&self) -> String {
        let 上下文 = self.上下文.lock().expect("上下文锁中毒");
        let 最近 = 上下文.最近(20);
        if 最近.is_empty() {
            return String::new();
        }
        let 段: Vec<String> = 最近
            .iter()
            .map(|消息| format!("[{}] {}", 角色名(&消息.角色), 消息.内容))
            .collect();
        format!("【临时·上下文流】\n{}", 段.join("\n"))
    }

    /// 内部：执行三态检索（格位 → 临时 → 图谱 → 诚实兜底），返回决策记录。
    /// 图谱下沉时会轻量校准候选格位（下调置信度）；随后触发纠错闭环（阶段 0B），
    /// 把候选格位置信度封顶到 0.7 并记录纠错事件写回存储。
    fn 检索(&self, 问题: &str) -> 检索决策记录 {
        let 决策 = {
            let 图谱 = self.图谱.lock().expect("图谱锁中毒");
            let mut 心智 = self.心智.lock().expect("心智锁中毒");
            let 上下文 = self.上下文.lock().expect("上下文锁中毒");
            let mut 检索器 = 检索器::新(&图谱, &mut 心智, &上下文);
            检索器.检索(问题)
        };
        if 决策.最终来源 == 检索源::图谱 && !决策.候选格位.is_empty() {
            self.纠错校准(&决策.候选格位);
        }
        决策
    }

    /// 检索注入：三态检索，返回结构化检索答复文本（供智能体循环初始注入使用）。
    pub fn 检索注入(&self, 问题: &str) -> String {
        self.检索(问题).答复
    }

    /// 答复注入（阶段 0C）：检索 + 可选 LLM 组装为自然语言。
    /// LLM 缺失、返回空或失败时回退结构化答复（fail-loud 降级，不阻断认知链路）。
    pub fn 答复注入(&self, 问题: &str, 对话器: Option<Arc<dyn 工具对话器>>) -> String {
        let 决策 = self.检索(问题);
        self.组装答复(问题, &决策.答复, 对话器)
    }

    /// 检索决策（阶段 0C 范围 3）：返回完整决策轨迹（检索源/下沉路径/候选格位）+ 答复，
    /// 供前端认知问答接口消费（决策轨迹可见）；答复可选 LLM 组装。
    pub fn 检索决策(
        &self,
        问题: &str,
        对话器: Option<Arc<dyn 工具对话器>>,
    ) -> (检索决策记录, String) {
        let 决策 = self.检索(问题);
        let 答复 = self.组装答复(问题, &决策.答复, 对话器);
        (决策, 答复)
    }

    /// 内部：把结构化检索答复经 LLM 组装为自然语言（缺失/空/失败回退结构化答复）。
    fn 组装答复(&self, 问题: &str, 检索答复: &str, 对话器: Option<Arc<dyn 工具对话器>>) -> String {
        let Some(对话器) = 对话器 else {
            return 检索答复.to_string();
        };
        let 消息 = vec![
            对话消息::系统(
                "你是洪荒世界的认知助手。把下列认知检索的结构化结果整理成简洁自然的回答，不要编造检索结果之外的内容。"
                    .to_string(),
            ),
            对话消息::用户(format!("问题：{问题}\n\n认知检索结果：\n{检索答复}")),
        ];
        match 对话器.对话(消息, vec![]) {
            Ok(响应) => {
                let 答复 = 响应.内容.clone().unwrap_or_default();
                if 答复.trim().is_empty() {
                    tracing::warn!("认知答复 LLM 组装返回空，回退结构化答复");
                    检索答复.to_string()
                } else {
                    答复
                }
            }
            Err(失败) => {
                tracing::warn!("认知答复 LLM 组装失败，回退结构化答复: {失败}");
                检索答复.to_string()
            }
        }
    }

    /// 纠错校准（阶段 0B）：图谱真源胜过格位摘要时，用纠错引擎下调置信度并记录纠错事件。
    /// 锁序固定 图谱 → 心智 → 上下文，短锁不跨锁。
    fn 纠错校准(&self, 候选: &[(维度, String)]) {
        // 先收集需纠错格位快照（避免与纠错引擎的可变借用冲突）
        let 待纠错: Vec<(维度, String, String, Vec<String>)> = {
            let 心智 = self.心智.lock().expect("心智锁中毒");
            候选
                .iter()
                .filter_map(|(维度值, 名)| {
                    let 格位 = 心智.查询格位(*维度值, 名)?;
                    if 格位.摘要.is_empty() {
                        return None;
                    }
                    Some((*维度值, 名.clone(), 格位.摘要.clone(), 格位.证据引用.clone()))
                })
                .collect()
        };
        if 待纠错.is_empty() {
            return;
        }
        let 图谱 = self.图谱.lock().expect("图谱锁中毒");
        let mut 心智 = self.心智.lock().expect("心智锁中毒");
        let mut 上下文 = self.上下文.lock().expect("上下文锁中毒");
        let mut 纠错 = 纠错引擎::新(&图谱, &mut 心智, &mut 上下文);
        for (维度值, 名, 旧摘要, 旧证据) in 待纠错 {
            let 事件 = 纠错.纠正(
                维度值,
                &名,
                旧摘要,
                旧证据,
                "图谱下沉校准：格位摘要未命中查询，置信度下调",
            );
            tracing::info!("纠错事件：{}·{} 置信度→{}", 维度值.中文名(), 名, 事件.新可信度);
            if let Some(存储) = &self.存储 {
                if let Err(失败) = 存储.追加纠错事件(&事件) {
                    tracing::warn!("纠错事件写回失败: {失败}");
                }
            }
        }
    }

    /// 记录：把一轮过程写回临时态上下文库（硬上限 1000 丢最旧）；达压缩阈值时触发上下文压缩（阶段 0B）。
    pub fn 记录(&self, 角色: 消息角色, 内容: impl Into<String>) {
        let 内容 = 内容.into();
        let 超阈值 = {
            let mut 上下文 = self.上下文.lock().expect("上下文锁中毒");
            上下文.追加(角色, 内容);
            上下文.长度() >= 上下文压缩阈值
        };
        if 超阈值 {
            self.压缩();
        }
    }

    /// 压缩（阶段 0B）：临时上下文达阈值后执行一次三阶段压缩（滑动窗口 + 分层摘要 + 蒸馏晋升）。
    /// 锁序固定 心智 → 上下文，短锁不跨锁。
    fn 压缩(&self) {
        let mut 心智 = self.心智.lock().expect("心智锁中毒");
        let mut 上下文 = self.上下文.lock().expect("上下文锁中毒");
        let mut 压缩器 = 压缩器::新(&mut 上下文, &mut 心智);
        let 晋升 = 压缩器.手动压缩();
        tracing::info!(
            "上下文压缩完成：压缩至 {} 条，晋升 {} 条经历",
            上下文.长度(),
            晋升.len()
        );
    }
}

/// 消息角色 中文名（用于流注入展示）
fn 角色名(角色: &消息角色) -> &'static str {
    match 角色 {
        消息角色::系统 => "系统",
        消息角色::用户 => "用户",
        消息角色::助手 => "助手",
        消息角色::工具结果 => "工具",
        消息角色::信号 => "信号",
    }
}
