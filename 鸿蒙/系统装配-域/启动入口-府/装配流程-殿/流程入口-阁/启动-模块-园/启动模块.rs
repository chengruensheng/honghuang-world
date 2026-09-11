use std::sync::{Arc, Mutex};

use crate::{装配开发受理台, 装配看板驱动台};
use hm_agent::{认知注入, 道祖接待};
use hm_cognition::上下文库;
use hm_content::LLM池;

/// 道祖接待会话持久化文件名（位于 persistence.dir 下）
const 道祖接待文件名: &str = "道祖接待.json";

pub fn 启动() -> hm_error::Result<Arc<hm_linkage::组件容器>> {
    let config = hm_config::运行配置();
    let logger = hm_log::Logger::new(&config.log);

    // 持久化目录：空字符串 = 纯内存（不持久化）。
    // 目录不存在则自动创建（原子写入不建父目录，五行引擎自动保存依赖目录就位）；
    // 创建失败回退纯内存并告警（与三态存储的降级策略一致）。
    let 持久化目录 = if config.persistence.dir.trim().is_empty() {
        None
    } else {
        match std::fs::create_dir_all(&config.persistence.dir) {
            Ok(()) => Some(config.persistence.dir.clone()),
            Err(e) => {
                tracing::warn!("持久化目录 {} 创建失败，回退纯内存模式: {e}", config.persistence.dir);
                None
            }
        }
    };

    // 商业级 LLM 池：配置 providers 非空 → 从配置（多供应商池）；否则回退环境变量单点（兼容 v1.43）。
    // 状态文件相对 persistence.dir；无持久化目录时仅运行时生效（不落盘）。
    let mut llm配置 = config.llm.clone();
    if !llm配置.state_file.is_empty() {
        if let Some(d) = &持久化目录 {
            llm配置.state_file = format!("{d}/{}", llm配置.state_file);
        } else {
            llm配置.state_file = String::new();
        }
    }
    let llm池: Option<Arc<LLM池>> = if !llm配置.providers.is_empty() {
        let 池 = LLM池::从配置(&llm配置);
        // 运行时接入的 env 引用供应商（llm-接入.json）合并进池，重启自动恢复
        if let Err(e) = 池.从接入文件合并() {
            tracing::warn!("LLM 接入文件合并失败: {e}");
        }
        if 池.可用() {
            tracing::info!("LLM 池已装配（配置 {} 个供应商）", 池.供应商清单().len());
            Some(Arc::new(池))
        } else {
            tracing::warn!("LLM 池配置无可用供应商（全部禁用或密钥缺失）");
            None
        }
    } else {
        match LLM池::从环境() {
            Ok(池) => {
                tracing::info!("LLM 池已装配（环境变量注入，供应商 {} 个）", 池.供应商清单().len());
                Some(Arc::new(池))
            }
            Err(e) => {
                tracing::warn!("LLM 池未装配（未配置供应商且环境密钥缺失）: {e}");
                None
            }
        }
    };

    // 生产路径装配五行：五引擎 + 信号总线 + 认知三态 + 日志记录器，串成闭环
    let 装配 = hm_linkage::五行装配::装配带扫描(持久化目录.clone(), Some(config.app.scan_root.clone()));

    // 通过运行时容器统一管理生命周期：注册日志器并初始化（府可插拔）
    装配.容器.注册初始化(Arc::new(logger));
    装配.容器.初始化()?;

    tracing::info!("{} v{} 启动成功", config.app.name, config.app.version);
    tracing::info!(
        "五行相生装配完成：任务/迭代/记忆/规则/事件 五引擎与信号总线已串联（已注册组件 {:?}）",
        装配.容器.组件名()
    );

    let 鉴权令牌 = if config.http.auth_token.trim().is_empty() {
        None
    } else {
        Some(config.http.auth_token.clone())
    };

    // 数据服务状态：五引擎 + 认知三态 + 任务看板 + 日志记录器 + 开发执行台 + 鉴权令牌
    let 看板路径 = match &持久化目录 {
        Some(d) => format!("{d}/任务看板.jsonl"),
        None => std::env::temp_dir().join("洪荒任务看板.jsonl").to_string_lossy().to_string(),
    };
    // 加载历史看板（文件不存在则空板）：重启恢复，避免看板失忆
    let 任务看板 = Arc::new(std::sync::Mutex::new(match tc_task::TaskBoard::加载(&看板路径) {
        Ok(看板) => 看板,
        Err(e) => {
            tracing::warn!("任务看板加载失败，使用空看板: {e}");
            tc_task::TaskBoard::新建(&看板路径)
        }
    }));
    {
        let mut 看板 = 任务看板.lock().expect("看板锁中毒");
        看板.设置信号总线(装配.信号总线.clone());
    }
    let mut 数据状态 = hm_http::数据服务状态::新(
        装配.任务仓库.clone(),
        装配.迭代日志.clone(),
        装配.记忆库.clone(),
        装配.规则库.clone(),
        装配.事件总线.clone(),
        装配.图谱.clone(),
        装配.心智地图.clone(),
        装配.语境.clone(),
        任务看板.clone(),
        装配.日志记录器.clone(),
        Arc::new(hm_http::开发执行台::新()),
        Arc::new(hm_http::看板驱动台::新()),
        llm池.clone(),
        鉴权令牌,
    );
    数据状态.扫描根 = Arc::new(Mutex::new(config.app.scan_root.clone()));

    // 自主开发智能体上线：run_dev_agent=true 时装配到 HTTP 受理台（默认关闭）。
    // LLM key 缺失仅告警，受理台保持未上线（受理接口 503），不影响数据服务；
    // dev_task 非空时自动受理为首个任务
    if config.app.run_dev_agent {
        // 执行链视图：「执行」身份独立模型绑定（无绑定回退全局选择；UI 改绑即时生效）
        let 执行视图: Option<Arc<dyn hm_content_contract::工具对话器>> =
            llm池.as_ref().map(|池| 池.绑定视图("执行") as Arc<dyn hm_content_contract::工具对话器>);
        match 装配开发受理台(
            &数据状态.开发执行台,
            &config.app.dev_workspace,
            config.app.dev_max_rounds,
            config.app.executor_timeout_secs,
            config.app.executor_max_output_bytes,
            执行视图.clone(),
        ) {
            Ok(()) => {
                tracing::info!("自主开发智能体已上线（HTTP 受理模式，工作区 {}）", config.app.dev_workspace);
                // 设置工作区重装配回调：外部可通过 POST /api/dev/workspace 切换工作区
                let 执行台 = 数据状态.开发执行台.clone();
                let 最大轮数 = config.app.dev_max_rounds;
                let 超时秒 = config.app.executor_timeout_secs;
                let 输出上限 = config.app.executor_max_output_bytes;
                let 重装配视图 = 执行视图.clone();
                数据状态.重装配工作区 = Some(Arc::new(move |新工作区: &str| {
                    装配开发受理台(&执行台, 新工作区, 最大轮数, 超时秒, 输出上限, 重装配视图.clone())
                }));
                if !config.app.dev_task.trim().is_empty() {
                    match hm_http::受理开发任务(&数据状态, config.app.dev_task.clone()) {
                        Ok(id) => tracing::info!("已自动受理初始任务 id={id}：{}", config.app.dev_task),
                        Err(失败) => tracing::warn!("初始任务受理失败（不影响启动）: {失败:?}"),
                    }
                }
                // 看板驱动台装配：驱动接口 /api/dev/pilot 就绪（LLM key 缺失时未就绪，驱动接口 503 fail-loud）
                let 驱动上下文路径 = match &持久化目录 {
                    Some(d) => format!("{d}/看板驱动上下文.jsonl"),
                    None => std::env::temp_dir().join("洪荒看板驱动上下文.jsonl").to_string_lossy().to_string(),
                };
                // 三态认知注入：看板驱动通道带上 格位/图谱/临时上下文（推/拉/流），每轮过程记录回临时态
                // 三态持久化：persistence.dir/认知三态/ 存在历史则恢复（损坏告警保持空态），每轮对话后自动写穿
                let 尝试恢复: Option<认知注入> = 持久化目录.as_ref().and_then(|d| {
                    let 存储 = match hm_cognition::三态存储::新(format!("{d}/认知三态")) {
                        Ok(存储) => 存储,
                        Err(失败) => {
                            tracing::warn!("认知三态存储目录创建失败，跳过持久化: {失败}");
                            return None;
                        }
                    };
                    if !存储.已存在() {
                        return None;
                    }
                    match 存储.加载() {
                        Ok((图谱, 心智, 上下文库)) => {
                            *数据状态.图谱.lock().expect("图谱锁中毒") = 图谱;
                            *数据状态.心智地图.lock().expect("心智锁中毒") = 心智;
                            tracing::info!("认知三态已从历史恢复（临时消息 {} 条）", 上下文库.长度());
                            Some(
                                认知注入::新(
                                    数据状态.图谱.clone(),
                                    数据状态.心智地图.clone(),
                                    Arc::new(Mutex::new(上下文库)),
                                )
                                .装配存储(Arc::new(存储)),
                            )
                        }
                        Err(失败) => {
                            tracing::warn!("认知三态恢复失败，保持空态继续启动: {失败}");
                            None
                        }
                    }
                });
                let 认知注入 = match 尝试恢复 {
                    Some(注入) => 注入,
                    None => {
                        let 存储 = 持久化目录.as_ref().and_then(|d| {
                            hm_cognition::三态存储::新(format!("{d}/认知三态")).ok()
                        });
                        match 存储 {
                            Some(存储) => 认知注入::新(
                                数据状态.图谱.clone(),
                                数据状态.心智地图.clone(),
                                Arc::new(Mutex::new(上下文库::新_带上限(1000))),
                            )
                            .装配存储(Arc::new(存储)),
                            None => 认知注入::新(
                                数据状态.图谱.clone(),
                                数据状态.心智地图.clone(),
                                Arc::new(Mutex::new(上下文库::新_带上限(1000))),
                            ),
                        }
                    }
                };
                // 规则种子注入：从 rules/ 目录加载 .md 规则文件，写入心智地图的规则维度格位
                {
                    let 种子们 = hm_agent::加载规则种子("rules");
                    if !种子们.is_empty() {
                        hm_agent::注入种子到格位(&数据状态.心智地图, &种子们);
                    }
                }
                // 驱动会话落盘目录：persistence.dir/驱动会话（空=纯内存，重启丢失）
                let 驱动会话目录 = match &持久化目录 {
                    Some(d) => format!("{d}/驱动会话"),
                    None => String::new(),
                };
                match 装配看板驱动台(
                    &数据状态.看板驱动台,
                    任务看板.clone(),
                    &驱动上下文路径,
                    &config.app.dev_workspace,
                    config.app.dev_max_rounds,
                    config.app.executor_timeout_secs,
                    config.app.executor_max_output_bytes,
                    Some(认知注入.clone()),
                    执行视图.clone(),
                    &驱动会话目录,
                ) {
                    Ok(()) => tracing::info!("看板驱动已上线（HTTP 驱动模式）"),
                    Err(e) => tracing::warn!("看板驱动装配失败，驱动接口不可用（不影响启动）: {e}"),
                }
                // 看板驱动台重装配回调：顶栏切换项目工作区时，与开发受理台同步重装配，
                // 确保五层协作驱动器的执行器工作区与扫描根一致（产出直接落项目根）
                let 看板驱动台克隆 = 数据状态.看板驱动台.clone();
                let 看板克隆 = 任务看板.clone();
                let 上下文路径克隆 = 驱动上下文路径.clone();
                let 会话目录克隆 = 驱动会话目录.clone();
                let 认知克隆 = 认知注入.clone();
                let 视图克隆 = 执行视图.clone();
                let 最大轮数 = config.app.dev_max_rounds;
                let 超时秒 = config.app.executor_timeout_secs;
                let 输出上限 = config.app.executor_max_output_bytes;
                数据状态.重装配看板驱动 = Some(Arc::new(move |新工作区: &str| {
                    装配看板驱动台(
                        &看板驱动台克隆,
                        看板克隆.clone(),
                        &上下文路径克隆,
                        新工作区,
                        最大轮数,
                        超时秒,
                        输出上限,
                        Some(认知克隆.clone()),
                        视图克隆.clone(),
                        &会话目录克隆,
                    )
                }));
                // 道祖接待器装配：主控澄清会话（持久化到 persistence.dir/道祖接待.json，重启恢复）
                let 道祖路径 = match &持久化目录 {
                    Some(d) => format!("{d}/{道祖接待文件名}"),
                    None => std::env::temp_dir().join(道祖接待文件名).to_string_lossy().to_string(),
                };
                // 道祖视图：「道祖」身份独立模型绑定（澄清对话通道，无绑定回退全局选择）。
                // llm池 在密钥缺失/供应商全禁用时为 None，不得 expect（否则 run_dev_agent=true 且
                // 池未就绪时启动即崩）——此处与上方「执行视图」的 .map 兜底保持一致。
                if let Some(池) = llm池.as_ref() {
                    let 道祖视图 = 池.绑定视图("道祖");
                    match 道祖接待::加载(道祖视图.clone(), 道祖路径) {
                        Ok(接待) => {
                            let 接待 = 接待
                                .装配认知(认知注入.clone())
                                .装配流式(道祖视图);
                            数据状态.道祖接待 = Some(Arc::new(Mutex::new(接待)));
                            tracing::info!("道祖接待已上线（主控澄清模式，认知装配已对齐，流式对话已开启）");
                        }
                        Err(e) => tracing::warn!("道祖接待装配失败，对话澄清不可用（不影响启动）: {e}"),
                    }
                } else {
                    tracing::warn!("道祖接待装配跳过（LLM 池未就绪）");
                }
                // 三态认知注入注入数据服务状态：认知问答接口据此提供检索决策与 LLM 组装答复（阶段 0C）
                数据状态.认知注入 = Some(认知注入.clone());
                // 扫尾执行者装配：太乙金仙清理后的交付证据核验（sweep 接口就绪）
                let 扫尾执行器 = Arc::new(hm_execute::本地执行器::new_with_limits(
                    &config.app.dev_workspace,
                    config.app.executor_timeout_secs,
                    config.app.executor_max_output_bytes,
                ));
                数据状态.扫尾执行者 = Some(Arc::new(hm_http::扫尾执行者::新(
                    扫尾执行器,
                    任务看板.clone(),
                )));
                tracing::info!("扫尾执行者已上线（sweep 核验接口就绪）");
            }
            Err(e) => tracing::warn!("智能体装配失败，HTTP 受理不可用（不影响启动）: {e}"),
        }
    }

    // 对外契约（独立文件配置化）：前端/客户端消费面参数；增删前端只改 对外契约.toml，不改后端代码
    let 契约 = hm_config::对外契约配置();
    数据状态 = 数据状态.设置并发上限(契约.对外.sse_max);

    // 启动数据服务：纯 API（独立线程，失败仅告警不影响主程序）
    hm_http::启动数据服务(数据状态, config.http.bind.clone(), config.http.port, 契约.对外);

    // 启动自检仅在显式开启时运行（默认关闭，避免污染真实业务数据）；
    // 验证失败仅告警并继续启动，不得因失败导致程序退出
    if config.app.run_self_test {
        match hm_linkage::演示闭环(&装配) {
            Ok(状态) => tracing::info!("五行闭环验证通过：任务 {}、迭代 {}、记忆 {}、规则 {}、事件 {}", 状态.任务数, 状态.迭代数, 状态.记忆数, 状态.规则数, 状态.事件数),
            Err(e) => tracing::warn!("五行闭环验证失败（不影响启动）: {e}"),
        }
        match hm_linkage::演示相克(&装配) {
            Ok(克制) => tracing::info!("五行克制验证通过：事件去重 {}", 克制.去重事件),
            Err(e) => tracing::warn!("五行克制验证失败（不影响启动）: {e}"),
        }
    }


    Ok(装配.容器.clone())
}
