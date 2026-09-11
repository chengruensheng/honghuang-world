use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use hm_contract::Component;
use hm_content_contract::{工具对话器, 对话消息, 工具调用};
use hm_error::{Error, Result};
use hm_execute_contract::{开发事件, 开发事件类型, 开发执行契约, 执行器};
use super::super::任务_清单_园::{任务项, 格式化清单, 解析任务清单, 清单项键_内容, 清单项键_状态, 状态_待办, 状态_进行中, 状态_已完成};
use super::super::认知_注入_园::认知注入;
use hm_cognition::消息角色 as 认知消息角色;

/// 事件内容截断上限（读文件/命令输出可能很长，事件流只保留摘要）
const 事件内容上限: usize = 200;
/// 任务答复上限：答复承载这一棒的结论，不能与工具输出共用 200 字节的帽子——
/// 推理模型的思考链本身常有数百字节，200 字节会被它整段吃满，
/// 真正的结论（JSON 正文）连一个字符都进不了事件流（2026-09-11 真机实测）。
const 答复内容上限: usize = 4000;

/// 退化循环告警阈值：连续以完全相同签名（工具名+规范化参数）调用同一工具且持续失败达此次数，
/// 说明模型已卡死，向消息流注入强纠正提示，禁止再用相同参数重试。
/// 取 2：2026-09-11 真机实测 LLM 失败 1-2 次即放弃重试，阈值 3 在真机不可达。
const 同调失败告警阈值: usize = 2;
/// 退化循环熔断阈值：告警后仍以相同签名失败达此次数，直接终止本轮自主开发，
/// 避免一路空转到最大轮数（2026-09-11 实测：空参「读文件」曾连败 60 轮耗尽轮数）。
/// 取 3：紧接告警阈值之后一次即掐断，避免真机上「够不着」而形同虚设。
const 同调失败熔断阈值: usize = 3;

/// 退化循环检测器：连续以完全相同签名（工具名 + 规范化参数）失败的计数状态。
///
/// 用 `Arc<Mutex>` 包裹使状态可跨轮、跨层共享——驱动器每轮都会新建智能体实例，
/// 若计数随实例创建而重置，同一工具在五层流转中反复失败也无法累积到阈值，
/// 熔断将永不触发（2026-09-11 真机验证：6 次工具失败分散在 5 个实例，零熔断）。
#[derive(Clone, Default)]
pub struct 退化检测器 {
    内: Arc<Mutex<退化计数>>,
}

#[derive(Default)]
struct 退化计数 {
    /// 上一次失败的签名（规范化后）
    上次失败签名: Option<String>,
    /// 连续同签名失败次数（换签名或成功即归零）
    同签名失败次数: usize,
}

impl 退化检测器 {
    /// 新建独立检测器（智能体默认持有；驱动器注入同一实例以跨轮共享）
    pub fn 新() -> Self {
        Self::default()
    }

    /// 重置计数：任务切换时调用，避免上一任务的失败账记到新任务
    pub fn 重置(&self) {
        let mut 内 = self.内.lock().expect("退化检测锁中毒");
        内.上次失败签名 = None;
        内.同签名失败次数 = 0;
    }

    /// 记录一次工具调用结果，返回（是否达告警阈值, 是否达熔断阈值, 当前连续同签名失败次数）。
    /// 成功调用或换签名即归零，故「连续」严格成立。
    pub fn 记录(&self, 签名: &str, 失败: bool) -> (bool, bool, usize) {
        let mut 内 = self.内.lock().expect("退化检测锁中毒");
        if 失败 {
            if 内.上次失败签名.as_deref() == Some(签名) {
                内.同签名失败次数 += 1;
            } else {
                内.同签名失败次数 = 1;
                内.上次失败签名 = Some(签名.to_string());
            }
        } else {
            内.同签名失败次数 = 0;
            内.上次失败签名 = None;
        }
        let 次数 = 内.同签名失败次数;
        (失败 && 次数 == 同调失败告警阈值, 失败 && 次数 >= 同调失败熔断阈值, 次数)
    }
}

/// 工具名常量（function calling 的 function.name）
const 读文件: &str = "读文件";
const 写文件: &str = "写文件";
const 运行命令: &str = "运行命令";
const 列目录: &str = "列目录";
const 按名找文件: &str = "按名找文件";
const 搜索内容: &str = "搜索内容";
const 精确编辑: &str = "精确编辑";
const 删除文件: &str = "删除文件";
const 任务清单: &str = "任务清单";

/// 工具对外名（function calling 里 `function.name` 实际发给网关的值）。
///
/// 规范名是上面的中文常量，但部分 OpenAI 兼容网关会把 `function.name` 中的非 ASCII
/// 字符逐字替换成下划线（2026-09-11 真机实测 a6api：发「读文件」回来的是「___」），
/// 模型据此调用时本地一个都匹配不上，整轮全部以「未知工具」失败并触发熔断。
/// 故对外统一改用 ASCII 别名，收到调用后还原成中文规范名——内部命名与执行逻辑不变。
fn 对外工具名(规范名: &str) -> &'static str {
    match 规范名 {
        读文件 => "read_file",
        写文件 => "write_file",
        运行命令 => "run_command",
        列目录 => "list_dir",
        按名找文件 => "find_files",
        搜索内容 => "search_content",
        精确编辑 => "edit_file",
        删除文件 => "delete_file",
        任务清单 => "update_todos",
        _ => "unknown_tool",
    }
}

/// 还原工具名：把模型给的名字（可能是 ASCII 别名，也可能是没被网关改写的中文规范名）
/// 映射回本地规范名；两者都不认识时原样返回，交由 `执行调用` 报「未知工具」。
fn 还原工具名(模型给的名: &str) -> &str {
    match 模型给的名 {
        "read_file" => 读文件,
        "write_file" => 写文件,
        "run_command" => 运行命令,
        "list_dir" => 列目录,
        "find_files" => 按名找文件,
        "search_content" => 搜索内容,
        "edit_file" => 精确编辑,
        "delete_file" => 删除文件,
        "update_todos" => 任务清单,
        其它 => 其它,
    }
}

/// 工具参数载荷键常量（与函数定义 schema 的 property 名一致，中文为规范键）
const 参数键_路径: &str = "路径";
const 参数键_内容: &str = "内容";
const 参数键_命令: &str = "命令";
const 参数键_模式: &str = "模式";
const 参数键_关键词: &str = "关键词";
const 参数键_旧: &str = "旧";
const 参数键_新: &str = "新";
const 参数键_清单: &str = "清单";

/// 兼容性别名：部分模型会用英文键，回退识别以提升鲁棒性（规范键仍为中文，不在 schema 中暴露）
const 参数键_路径_英: &str = "path";
const 参数键_内容_英: &str = "content";
const 参数键_命令_英: &str = "command";
const 参数键_模式_英: &str = "pattern";
const 参数键_关键词_英: &str = "keyword";
const 参数键_旧_英: &str = "old";
const 参数键_新_英: &str = "new";
const 参数键_清单_英: &str = "todos";

/// 系统提示：约束 LLM 的角色与工具使用方式
const 系统提示: &str = "你是一个自主开发智能体，在指定工作区内完成开发任务。可调用九个工具（function.name 为英文标识，括号内为中文名）：read_file（读文件）、write_file（写文件）、run_command（运行命令）、list_dir（列目录）、find_files（按名找文件）、search_content（搜索内容）、edit_file（精确编辑）、delete_file（删除文件）、update_todos（任务清单）。列目录看一层条目，按名找文件用 glob 模式（*、**、?）找文件，搜索内容按关键词递归检索文本（返回 路径:行号:内容），精确编辑把文件中唯一匹配的旧串替换为新串（多处匹配会报错，需提供更精确上下文），删除文件删除工作区内文件（清理临时/备份产物，如写文件/精确编辑自动产生的 .bak 备份），任务清单用数组维护待办/进行中/已完成的多步计划。运行环境是 Windows，命令须用 cmd 语法（列目录用 dir、查看文件用 type、构建测试用 cargo）。每步先思考再行动；工具调用失败要读取错误信息并修正参数重试，不得在工具失败时直接宣告任务完成；任务完成后停止调用工具并给出简短说明。每次修改代码或配置后必须运行 cargo build 与 cargo test 验证通过，验证失败须读取错误并修复，不得跳过验证。跨文件或跨 crate 改动时：新增依赖加到实际使用它的 crate 的 Cargo.toml；修改函数签名后须同步更新所有调用点。";

/// 自主开发智能体：LLM 大脑 + 执行器手脚的循环
pub struct 智能体 {
    对话器: Arc<dyn 工具对话器>,
    执行器: Arc<dyn 执行器>,
    最大轮数: usize,
    中断标志: Arc<AtomicBool>,
    事件回调: Option<Arc<dyn Fn(&开发事件) + Send + Sync>>,
    任务清单: Arc<Mutex<Vec<任务项>>>,
    /// 三态认知注入（可选）：装配后 LLM 决策前带 推（格位）/拉（图谱）/流（临时），并把过程记录回临时态
    认知: Option<认知注入>,
    /// 工作区根路径（可选）：注入系统提示告知 LLM 用相对路径在工作区内探索
    工作区根: Option<String>,
    /// 检查点回调（可选）：每轮结束后透传「已完成轮数 + 消息快照 + 任务清单快照」，供外部落盘运行断点
    检查点回调: Option<Arc<dyn Fn(usize, &[对话消息], &[任务项]) + Send + Sync>>,
    /// 退化循环检测器：默认独立实例；驱动器注入同一实例后，同签名失败计数跨轮/跨层累积
    退化检测器: 退化检测器,
}

impl 智能体 {
    pub fn new(对话器: Arc<dyn 工具对话器>, 执行器: Arc<dyn 执行器>, 最大轮数: usize) -> Self {
        智能体 { 对话器, 执行器, 最大轮数, 中断标志: Arc::new(AtomicBool::new(false)), 事件回调: None, 任务清单: Arc::new(Mutex::new(Vec::new())), 认知: None, 工作区根: None, 检查点回调: None, 退化检测器: 退化检测器::新() }
    }

    /// 链式装配共享退化检测器：驱动器每轮新建智能体时注入同一实例，
    /// 使同一任务的同签名失败计数跨轮/跨层累积，而非随实例创建被重置。
    pub fn 装配退化检测器(mut self, 检测器: 退化检测器) -> Self {
        self.退化检测器 = 检测器;
        self
    }

    /// 链式装配三态认知注入：未装配时行为与旧版完全一致
    pub fn 装配认知(mut self, 认知: 认知注入) -> Self {
        self.认知 = Some(认知);
        self
    }

    /// 链式设置工作区根：注入系统提示，告知 LLM 用相对路径在工作区内探索
    pub fn 设置工作区(mut self, 根: String) -> Self {
        self.工作区根 = Some(根);
        self
    }

    /// 链式注入事件回调：循环各步（思考/工具调用/工具结果/答复）回调通知外部
    pub fn 设置事件回调(mut self, 回调: Arc<dyn Fn(&开发事件) + Send + Sync>) -> Self {
        self.事件回调 = Some(回调);
        self
    }

    /// 链式注入检查点回调：每轮结束后透传（已完成轮数, 消息快照, 任务清单快照）供外部落盘断点
    pub fn 设置检查点回调(mut self, 回调: Arc<dyn Fn(usize, &[对话消息], &[任务项]) + Send + Sync>) -> Self {
        self.检查点回调 = Some(回调);
        self
    }

    /// 返回中断句柄：外部设置 true 可请求停止循环（如 Ctrl+C 回调）
    pub fn 中断句柄(&self) -> Arc<AtomicBool> {
        self.中断标志.clone()
    }

    /// 返回当前任务清单的可读文本（只读，供外部面板展示或测试验证）
    pub fn 当前任务清单(&self) -> String {
        let 清单 = self.任务清单.lock().expect("任务清单锁中毒");
        格式化清单(&清单)
    }

    /// 发出一个开发事件（未注入回调时静默跳过；内容按类型做有界摘要）
    fn 发事件(&self, 轮次: usize, 类型: 开发事件类型, 工具名: &str, 内容: &str) {
        if let Some(回调) = &self.事件回调 {
            // 答复承载结论，给足额度；工具输出是过程噪音，限 200 字。
            let 摘要 = if matches!(&类型, 开发事件类型::任务答复) {
                截断(内容, 答复内容上限)
            } else if matches!(&类型, 开发事件类型::工具结果)
                && 还原工具名(工具名) == "运行命令"
            {
                // 命令结论在尾部（test result / Finished / error[E..]），尾优先保结论，
                // 不再像通用截断那样切掉验收结论行（2026-09-11 真机实证）。
                super::结果摘要::摘要命令输出(内容, 事件内容上限)
            } else {
                截断(内容, 事件内容上限)
            };
            let 事件 = 开发事件 {
                轮次,
                类型,
                工具名: 工具名.to_string(),
                内容: 摘要,
            };
            回调(&事件);
        }
    }

    /// 运行自主开发循环，返回 LLM 最终答复
    pub fn 运行(&self, 任务: String) -> Result<String> {
        self.运行从(任务, Vec::new())
    }

    /// 从已存消息恢复运行自主开发循环（Resume/Fork 用）。
    /// 已存消息非空则跳过头部（系统提示/初始注入）重建，直接基于历史消息继续下一次对话，避免从头探索。
    pub fn 运行从(&self, 任务: String, 已存消息: Vec<对话消息>) -> Result<String> {
        let 从零开始 = 已存消息.is_empty();
        // 三态初始注入：推（格位常驻）+ 拉（图谱按任务关键词）+ 检索答复（结构化），插在系统提示之后、用户任务之前
        // 恢复时历史已含该注入，不再重复记录（避免污染三态上下文）
        let 初始注入 = if 从零开始 {
            match &self.认知 {
                Some(认知) => {
                    认知.记录(认知消息角色::用户, 任务.clone());
                    let 初始 = 认知.初始注入(&任务);
                    let 检索 = 认知.检索注入(&任务);
                    [初始, 检索]
                        .into_iter()
                        .filter(|段| !段.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n\n")
                }
                None => String::new(),
            }
        } else {
            String::new()
        };
        let mut 消息 = if 从零开始 {
            let 系统提示文本 = match &self.工作区根 {
                Some(根) => format!(
                    "{}\n\n【工作区约定】工作区根路径为：{根}。所有工具调用（读文件/写文件/列目录/按名找文件/搜索内容/精确编辑/删除文件/运行命令）的路径与命令都必须在工作区内，且一律使用相对路径（如 src/main.rs），禁止使用绝对路径；浏览工作区根用：列目录(路径: \".\")。写文件/精确编辑会自动把原文件备份为同目录下带 .bak 后缀的文件，任务结束前须用 删除文件 清理这些 .bak 备份。",
                    系统提示
                ),
                None => 系统提示.to_string(),
            };
            let mut 消息 = vec![
                对话消息::系统(系统提示文本),
                对话消息::用户(任务),
            ];
            if !初始注入.is_empty() {
                消息.insert(1, 对话消息::系统(初始注入));
            }
            消息
        } else {
            // 恢复路径：任务已在历史消息里，无需再入头部；显式消费避免 unused
            drop(任务);
            已存消息
        };
        let 工具 = 工具定义();

        for 轮次 in 0..self.最大轮数 {
            if self.中断标志.load(Ordering::SeqCst) {
                tracing::warn!("收到中断请求，停止智能体循环");
                return Err(Error::中断("循环被用户中断".into()));
            }
            tracing::info!("══════ 第 {} 轮 ══════", 轮次 + 1);
            self.发事件(轮次, 开发事件类型::思考, "", "正在思考下一步行动");
            // 每轮流注入：临时上下文最近过程（非空才插入）
            if let Some(认知) = &self.认知 {
                let 流 = 认知.流注入();
                if !流.is_empty() {
                    消息.push(对话消息::系统(流));
                }
            }
            // 发"思考开始"事件（和ChatGPT一样，显示"正在思考..."动画）
            self.发事件(轮次, 开发事件类型::思考, "", "__思考开始__");

            let 响应 = self.对话器.对话(消息.clone(), 工具.clone())?;

            // 发"思考结束"事件
            self.发事件(轮次, 开发事件类型::思考, "", "__思考结束__");

            if 响应.工具调用.is_empty() {
                let 答复 = 响应.内容.clone().unwrap_or_default();
                tracing::info!("【LLM 最终答复】{}", 答复);
                self.发事件(轮次, 开发事件类型::任务答复, "", &答复);
                if let Some(认知) = &self.认知 {
                    认知.记录(认知消息角色::助手, 答复.clone());
                    认知.保存();
                }
                return Ok(答复);
            }

            消息.push(对话消息::助手调用(响应.工具调用.clone()));
            // 本回合待注入的熔断纠正提示：须待本回合全部工具结果入队后再追加系统消息，
            // 避免在 assistant(tool_calls) 与 tool 结果之间插入非 tool 消息而破坏调用协议
            let mut 本回合纠正: Option<String> = None;
            for 调用 in &响应.工具调用 {
                tracing::info!("【LLM 调用工具】{}  参数：{}", 调用.名称, 调用.参数);
                self.发事件(轮次, 开发事件类型::工具调用, 还原工具名(&调用.名称), &调用.参数);
                // 工具失败不回传终止循环，而是把错误信息回填给 LLM，让 LLM 看到错误后修正重试
                let (结果, 本调用失败) = match self.执行调用(调用) {
                    Ok(输出) => (输出, false),
                    Err(e) => {
                        tracing::warn!("【工具失败】{}", e);
                        (format!("工具执行失败: {e}"), true)
                    }
                };
                tracing::info!("【工具返回】{}", 结果);
                self.发事件(轮次, 开发事件类型::工具结果, &调用.名称, &结果);
                if let Some(认知) = &self.认知 {
                    认知.记录(认知消息角色::助手, format!("调用工具 {} 参数 {}", 调用.名称, 调用.参数));
                    认知.记录(认知消息角色::工具结果, 结果.clone());
                }

                // 退化循环熔断：连续以完全相同签名（工具名 + 规范化参数）失败 ⇒ 先注入纠正，再犯则终止
                let 签名 = 调用签名(还原工具名(&调用.名称), &调用.参数);
                let (达告警, 达熔断, 连续次数) = self.退化检测器.记录(&签名, 本调用失败);
                if 达熔断 {
                    let 提示 = format!(
                        "连续 {} 次以相同参数调用「{}」均失败（最近错误：{}），判定为退化循环，已终止本轮自主开发以防空转。请检查工具参数或改用其它工具后重试。",
                        连续次数,
                        调用.名称,
                        截断(&结果, 120)
                    );
                    tracing::error!("【退化循环熔断】{}", 提示);
                    self.发事件(轮次, 开发事件类型::思考, "", &提示);
                    消息.push(对话消息::工具结果(调用.id.clone(), 结果));
                    return Err(Error::退化循环熔断(提示));
                }
                if 达告警 {
                    let 纠正 = format!(
                        "【熔断告警】你已连续 {} 次以完全相同参数调用「{}」且全部失败。禁止再用相同参数重试；请立即更换策略：修正参数、改读其它文件，或先用「列目录」「按名找文件」探明真实路径。最近错误：{}",
                        连续次数,
                        调用.名称,
                        截断(&结果, 120)
                    );
                    self.发事件(轮次, 开发事件类型::思考, "", "熔断告警：检测到重复失败调用，已注入纠正提示");
                    本回合纠正 = Some(纠正);
                }
                消息.push(对话消息::工具结果(调用.id.clone(), 结果));
            }
            // 本回合工具结果全部入队后，追加熔断纠正系统消息
            if let Some(纠正) = 本回合纠正 {
                消息.push(对话消息::系统(纠正));
            }
            // 工具轮结束：三态写穿落盘（装配存储时）
            if let Some(认知) = &self.认知 {
                认知.保存();
            }
            // 每轮末检查点回调：透传已完成轮数 + 消息快照 + 任务清单快照，供外部落盘运行断点
            if let Some(回调) = &self.检查点回调 {
                let 清单 = self.任务清单.lock().expect("任务清单锁中毒");
                回调(轮次 + 1, &消息, &清单);
            }
        }

        Err(Error::超出轮数(format!("超过最大轮数 {} 仍未完成任务", self.最大轮数)))
    }

    /// 分发并执行单个工具调用
    fn 执行调用(&self, 调用: &工具调用) -> Result<String> {
        let 参数: serde_json::Value = serde_json::from_str(&调用.参数)
            .map_err(|e| Error::反序列化(format!("解析工具参数失败: {e}")))?;
        match 还原工具名(&调用.名称) {
            读文件 => {
                let 路径 = 取参数字符串(&参数, &[参数键_路径, 参数键_路径_英])?;
                self.执行器.读文件(&路径)
            }
            写文件 => {
                let 路径 = 取参数字符串(&参数, &[参数键_路径, 参数键_路径_英])?;
                let 内容 = 取参数字符串(&参数, &[参数键_内容, 参数键_内容_英])?;
                self.执行器.写文件(&路径, &内容)?;
                Ok("写入成功".to_string())
            }
            运行命令 => {
                let 命令 = 取参数字符串(&参数, &[参数键_命令, 参数键_命令_英])?;
                self.执行器.运行命令(&命令)
            }
            列目录 => {
                // 路径可缺省：模型常传空参数 `{}` 表示「列出工作区根目录」，缺省列根（点号）
                let 路径 = 取参数字符串_或默认(&参数, &[参数键_路径, 参数键_路径_英], ".");
                self.执行器.列目录(&路径)
            }
            按名找文件 => {
                let 模式 = 取参数字符串(&参数, &[参数键_模式, 参数键_模式_英])?;
                self.执行器.按名找文件(&模式)
            }
            搜索内容 => {
                let 关键词 = 取参数字符串(&参数, &[参数键_关键词, 参数键_关键词_英])?;
                self.执行器.搜索内容(&关键词)
            }
            精确编辑 => {
                let 路径 = 取参数字符串(&参数, &[参数键_路径, 参数键_路径_英])?;
                let 旧 = 取参数字符串(&参数, &[参数键_旧, 参数键_旧_英])?;
                let 新 = 取参数字符串(&参数, &[参数键_新, 参数键_新_英])?;
                self.执行器.精确编辑(&路径, &旧, &新)
            }
            删除文件 => {
                let 路径 = 取参数字符串(&参数, &[参数键_路径, 参数键_路径_英])?;
                // 删除防护门：.bak 等自动备份免审直删，其余文件须过独立 LLM 审查（拒绝/不可解析均不放行）
                super::删除审查::审查并删除(&self.对话器, &self.执行器, &路径)
            }
            任务清单 => {
                let 清单值 = 参数
                    .get(参数键_清单)
                    .or_else(|| 参数.get(参数键_清单_英))
                    .ok_or_else(|| Error::缺少参数(参数键_清单.into()))?;
                let 新清单 = 解析任务清单(清单值)?;
                let mut 清单 = self.任务清单.lock().expect("任务清单锁中毒");
                *清单 = 新清单;
                Ok(格式化清单(&清单))
            }
            _ => Err(Error::未知工具(调用.名称.clone())),
        }
    }
}

/// 从工具参数中按候选键顺序取出字符串字段；缺失或非字符串时报错（以规范键命名错误信息）
fn 取参数字符串(参数: &serde_json::Value, 候选键: &[&str]) -> Result<String> {
    for 键 in 候选键 {
        if let Some(值) = 参数.get(*键).and_then(|v| v.as_str()) {
            return Ok(值.to_string());
        }
    }
    let 规范键 = match 候选键.first() {
        Some(键) => (*键).to_string(),
        None => "参数".to_string(),
    };
    Err(Error::缺少参数(规范键))
}

/// 从工具参数中按候选键顺序取出可选字符串字段；缺失或空串时返回默认值。
/// 用于语义上「可缺省」的工具（如「列目录」路径缺省列工作区根），避免模型传空参数连败。
fn 取参数字符串_或默认(参数: &serde_json::Value, 候选键: &[&str], 默认: &str) -> String {
    for 键 in 候选键 {
        if let Some(值) = 参数.get(*键).and_then(|v| v.as_str()) {
            if !值.is_empty() {
                return 值.to_string();
            }
        }
    }
    默认.to_string()
}

/// 计算工具调用的规范化签名：`工具名|规范化参数`。
///
/// 参数先按 JSON 解析再紧凑序列化（serde_json 的对象键为有序 BTreeMap），
/// 使 `{"路径": "a"}` 与 `{"路径":"a"}`（仅空白或键序差异）判为同一签名——
/// 旧实现直接拼接原始 JSON 字符串，同一语义调用因空白差异被误判为不同签名，
/// 致连续失败计数被重置、熔断永不累积（2026-09-11 真机暴露）。
/// 解析失败（模型给出非法 JSON）时回退原始字符串。
fn 调用签名(名称: &str, 原始参数: &str) -> String {
    let 规范 = serde_json::from_str::<serde_json::Value>(原始参数)
        .ok()
        .and_then(|值| serde_json::to_string(&值).ok())
        .unwrap_or_else(|| 原始参数.to_string());
    format!("{名称}|{规范}")
}

/// 按字符数截断文本（事件流/面板摘要用），超长部分以省略号结尾
fn 截断(文本: &str, 上限: usize) -> String {
    if 文本.chars().count() <= 上限 {
        文本.to_string()
    } else {
        let 头部: String = 文本.chars().take(上限).collect();
        format!("{头部}…")
    }
}

/// 智能体实现开发执行契约：受理即运行循环，中断句柄透传
impl Component for 智能体 {
    fn name(&self) -> &'static str { "智能体" }
}

impl 开发执行契约 for 智能体 {
    fn 执行开发任务(&self, 任务: String) -> Result<String> {
        self.运行(任务)
    }

    fn 中断句柄(&self) -> Arc<AtomicBool> {
        智能体::中断句柄(self)
    }
}

/// 三个工具的函数定义（OpenAI function calling 的 tools 数组元素）
fn 工具定义() -> Vec<serde_json::Value> {
    vec![
        单参函数(对外工具名(读文件), "读取工作区内文件的完整文本", 参数键_路径, "相对工作区的文件路径"),
        双参函数(对外工具名(写文件), "把内容写入工作区文件（覆盖）", 参数键_路径, "相对工作区的文件路径", 参数键_内容, "要写入的完整文本"),
        单参函数(对外工具名(运行命令), "在工作区目录下运行命令，返回标准输出", 参数键_命令, "要执行的命令"),
        单参函数_可选(对外工具名(列目录), "列出工作区内目录下条目（一层，区分目录/文件）；路径可缺省，缺省列工作区根目录", 参数键_路径, "相对工作区的目录路径，缺省为根目录"),
        单参函数(对外工具名(按名找文件), "按 glob 模式（*、**、?）递归匹配工作区内文件", 参数键_模式, "glob 模式，如 **/*.rs"),
        单参函数(对外工具名(搜索内容), "递归搜索工作区内文本文件内容，返回匹配行", 参数键_关键词, "要搜索的关键词"),
        三参函数(对外工具名(精确编辑), "把文件中唯一匹配的旧串替换为新串（多处匹配会报错）", 参数键_路径, "相对工作区的文件路径", 参数键_旧, "要被替换的旧文本（须唯一）", 参数键_新, "替换后的新文本"),
        单参函数(对外工具名(删除文件), "删除工作区内的单个文件（清理临时/备份产物，如 .bak）", 参数键_路径, "相对工作区的文件路径"),
        任务清单函数(),
    ]
}

/// 构造单字符串参数的函数定义
fn 单参函数(名: &str, 描述: &str, 键: &str, 键说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键.to_string(), serde_json::json!({"type": "string", "description": 键说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": [键]
            }
        }
    })
}

/// 构造可选单字符串参数的函数定义（参数可缺省，调用侧给默认值，如「列目录」缺省列工作区根）
fn 单参函数_可选(名: &str, 描述: &str, 键: &str, 键说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键.to_string(), serde_json::json!({"type": "string", "description": 键说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": []
            }
        }
    })
}

/// 构造双字符串参数的函数定义
fn 双参函数(名: &str, 描述: &str, 键一: &str, 键一说明: &str, 键二: &str, 键二说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键一.to_string(), serde_json::json!({"type": "string", "description": 键一说明}));
    属性.insert(键二.to_string(), serde_json::json!({"type": "string", "description": 键二说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": [键一, 键二]
            }
        }
    })
}

/// 构造三字符串参数的函数定义
fn 三参函数(名: &str, 描述: &str, 键一: &str, 键一说明: &str, 键二: &str, 键二说明: &str, 键三: &str, 键三说明: &str) -> serde_json::Value {
    let mut 属性 = serde_json::Map::new();
    属性.insert(键一.to_string(), serde_json::json!({"type": "string", "description": 键一说明}));
    属性.insert(键二.to_string(), serde_json::json!({"type": "string", "description": 键二说明}));
    属性.insert(键三.to_string(), serde_json::json!({"type": "string", "description": 键三说明}));
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 名,
            "description": 描述,
            "parameters": {
                "type": "object",
                "properties": 属性,
                "required": [键一, 键二, 键三]
            }
        }
    })
}

/// 构造任务清单工具的函数定义（参数为「清单」数组，每项含「内容」「状态」）
fn 任务清单函数() -> serde_json::Value {
    let mut 清单属性 = serde_json::Map::new();
    清单属性.insert(
        参数键_清单.to_string(),
        serde_json::json!({
            "type": "array",
            "description": "任务条目数组",
            "items": {
                "type": "object",
                "properties": {
                    (清单项键_内容): {"type": "string", "description": "任务内容"},
                    (清单项键_状态): {"type": "string", "enum": [状态_待办, 状态_进行中, 状态_已完成]}
                },
                "required": [清单项键_内容]
            }
        }),
    );
    serde_json::json!({
        "type": "function",
        "function": {
            "name": 任务清单,
            "description": "覆盖式更新多步任务清单（待办/进行中/已完成）",
            "parameters": {
                "type": "object",
                "properties": 清单属性,
                "required": [参数键_清单]
            }
        }
    })
}
