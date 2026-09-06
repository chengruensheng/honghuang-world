use std::sync::{Arc, Mutex};
use hm_cognition::{消息角色, 注入器, 上下文库, 三态存储, 图谱, 心智地图};

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
}

impl 认知注入 {
    /// 由数据服务/启动装配持有的三态共享句柄构造
    pub fn 新(
        图谱: Arc<Mutex<图谱>>,
        心智: Arc<Mutex<心智地图>>,
        上下文: Arc<Mutex<上下文库>>,
    ) -> Self {
        认知注入 { 图谱, 心智, 上下文, 存储: None }
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

    /// 记录：把一轮过程写回临时态上下文库（硬上限 1000 丢最旧）
    pub fn 记录(&self, 角色: 消息角色, 内容: impl Into<String>) {
        self.上下文
            .lock()
            .expect("上下文锁中毒")
            .追加(角色, 内容);
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
