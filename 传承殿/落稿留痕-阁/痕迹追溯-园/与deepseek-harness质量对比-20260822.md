# 洪荒世界 vs deepseek-harness · 质量差距对比（2026-08-22）

> 本文档对照 `d:\洪荒 - 世界` 与 `E:\trae\deepseek-harness` 的工程质量差距。
> 视角：本项目**未达发布标准**（界主明确），故不与发布成熟度比；只对照「架构成熟度 / 工程基线 / 可演进性」三块。
> 数据来源：洪荒审判 CLI、PowerShell 统计脚本、Explore 子代理对 deepseek-harness 的调研报告。

---

## 一、工程度量（数据对照）

| 维度 | 洪荒世界 | deepseek-harness | 差距倍数 | 备注 |
|:--|:--|:--|:--|:--|
| 主语言 | Rust | TypeScript (strict + ESM) | — | 不同生态 |
| 工作区单元 | **15** 个 Cargo.toml / lib crate | **249** 个 package.json（含 src 的 237 个） | **dsh ≈ 16×** | dsh 是工业级 monorepo |
| 核心代码量 | **3.4 万行 rs**（473 文件，不含 target / .上下文） | **约 23 万行 ts**（Explore 估，含 src） | **dsh ≈ 6.8×** | 体量不在同一量级 |
| Markdown 文档 | **272** 个 | 547 en + 392 zh + 392 i18n.yaml | dsh ≈ 5× | dsh 双语工程化 |
| 单元/集成测试 | **47** 个 `*测试*.rs`（zhengdao_fu 镜像六府） | **692** 个 `.spec.ts` + 7 个 pytest + 5 套 vitest 配置 | **dsh ≈ 14.7×** | dsh 全栈测试 |
| 顶层发布版本 | 无版本号（无版本字段） | `0.1.0-rc.5`（明示开发者预览） | dsh 有 release train | — |
| CI workflow | 0 个（项目内） | **15** 个 yml（CI / e2e / sandbox / release / python / landlock 多套） | dsh 全维度 | — |
| 工程脚本 | 极少量（基本靠命令操作-府的号令调用） | **142** 个 scripts/*.ts（含 verify-*/gen-*/check-* 全套） | dsh 工业化 | — |
| 依赖治理 | Cargo + 自写 .env 注入 + Cargo workspace | pnpm workspaces + overrides + allowBuilds 白名单 + patchedDependencies | dsh 更严 | — |
| 沙箱 | 命令沙箱护栏（禁 kill / 禁 cargo run 号令 / 超时强杀） | bwrap + 自研 Landlock(C) + Seatbelt + Windows ACL，四平台真内核 CI | dsh 跨平台 | — |
| 国际化 | **无**（全中文界面、文档、注释） | **完整双语工程**：merge driver + blob hash 一致性 + verify-translation-pairing 门禁 + 每包 README 三件套（219/219 全覆盖） | dsh 罕见成熟 | — |
| CookBook | 无（约定散在各设计稿） | 8 篇 cookbook（adding-a-{package,tool,vendored-package,llm-adapter,conversation-node,extension} + 评审回复） | — | — |
| Postmortem | 缺陷-记录-20260819.md（1 篇） | 4 篇 postmortem（每篇含 Executive summary + 根因 + gate 补丁） | — | — |
| 第三方声明 | 无（无第三方依赖 / MIT 自家） | THIRD_PARTY_NOTICES.md 自动生成 + verify 门禁 | — | — |
| 品牌包 | 无 npm 包 | `@deepseek-ai/dsh-*` 命名空间 + vendor 内嵌 cordis/cosmokit/group/hmr/include/loader/logger-console/schemastery/timer 9 包 | — | — |

> 体量上 dsh 比本项目大 **一个数量级**（代码 6.8×、测试 14.7×、包数 16×）；本项目用 3.4 万行 Rust 撑起了**完整的 8 府 36 格位主循环**，dsh 用 23 万行 ts 撑起 **49 个 group、约 237 个含 src 的包**——本质是「先验证闭环再扩体量」与「工业级 monorepo」两种节奏。

---

## 二、架构成熟度对比

### 1. 核心抽象对比

| 概念 | 洪荒世界 | deepseek-harness | 评价 |
|:--|:--|:--|:--|
| 插件/服务机制 | **府 = 插件单元（lib crate + Service Definition trait）** + `chajian_fu::府插件` trait + 插件上下文（服务注册表 HashMap<TypeId, …>）+ OnceLock 全局上下文 | **Cordis**：service = ctx 中的稳定 key；inject 声明依赖；effect = 可逆副作用；typed events（emit/waterfall/parallel/serial 四种分发）；scope/agent.ctx；bundle/profile/patch 层叠 | dsh 机制更精致（typed events 四种、scope 继承、bundle/patch 装配）；本项目是简化版但已够用 |
| LLM 适配 | `moxing_fu`（一个府，OpenAI 兼容单家） | `ctx.llm` seam + `llm-deepseek`/`llm-pi-ai`/`llm-replay` 三个 provider | dsh 多 provider + replay（无 key 测试） |
| 流式 | 通用（`moxing_fu` 内置） | `BlockAssembler` + `StreamChunk` 封闭联合（必须 `assertNever`） + `usage` 必须在 `finish` 前 | dsh 协议约束更强 |
| 工具注册/执行 | 10 个工具（写/读/改/删/列/找/搜/跑/读格位/查历史），落盘护栏三层（空内容 / 超长 / 路径越界），工具循环预算双轨（32 轮 / 90 万 token） | `ctx.tools` + `ToolDefinition`（schema + output + execute + finalizeContent + timeout + concurrency + presentCall/Result）；5 段瀑布（pre-execute → guards → execute → post-execute → finalizeContent → result）；scope-local 工具 + restriction（deny/allow 多取交集）+ shadowing；Code Mode；tool/result 二态 success/failure | dsh 远超：5 段瀑布 + 单调 guard + around-dispatch + presentCall/Result UI render intent + Code Mode |
| 子代理 / 多任务 | `道术施展-府` 工作流 L1-L4 + 大罗金仙分道执行 + `std::thread::scope` 并发派遣 | `ctx.subagents` 命名 provider（spawn-in-process / fork-in-process / acp / codex / claude-code / dsh-sdk），lineage 数据携带不影响可见性；goal round / Ralph round / workflow script（worker thread）等 | dsh 更深更广；本项目只覆盖本进程内 fork |
| 会话/记忆 | **识海承载-府**：36 格位（72 全量）+ 依赖图 + 6 范畴（目标/规则/自我/程序/世界/经历）+ 三级缓存（永久/版本/会话）+ 临时代偿（人类格位未录时 LLM 探索）+ 回滚垫（任务失败单文件撤销）+ 9 维 6 层结构 | **session/ event 流**：append-only `SessionEvent` 是模型上下文真源；`Surface`（user/assistant/tool 三类带 surfaceOp）+ Projection（fold unit）+ cold read ladder（cachedSnapshot → coldSnapshot → restoreFloor）；compaction = surface 唯一的 replace mutation；compaction-pruner 通过价格事件减 token | 两者思路一致：事件溯源 + 派生历史 + 折叠视图。本项目走「格位（人/代码/LLM 三源混存）」路线，dsh 走「强类型事件流 + 类型化投影」路线。dsh 模式更机器友好，本项目对人/语义更友好 |
| 上下文压缩 | 会话内压缩三件套（历史>15000 摘要 + 工具结果微压缩 + 单条微压缩） | `ctx.compaction` seam + dsh-compaction-basic + dsh-compaction-tool-result-pruner + token-meter；manual compaction 五种错误码（busy/cancelled/changed/summary/commit/persistence）；replaceGeneration 推进才重试 | dsh 完整 seam + 错误码化 |
| 持久化 | `.上下文/` 目录：格位 jsonl / 依赖图 json / 队列 jsonl / 版本快照（增量）/ 观测 jsonl / 回滚垫 | `ctx.sessionPersistence` seam + JSONL（Zstd checksummed frames + raw artifact）+ SQLite（每事件 1 行 + SCHEMA_VERSION）；崩溃恢复 = 合成 `turn/end {kind:'interrupted'}`；shared contract test suite | dsh 后端可选 + 跨后端契约测试 + 崩溃恢复语义化 |
| 沙箱 | 命令沙箱（沙箱护栏.rs：禁 kill / 禁 cargo run 号令 / 路径越界 / 超时 10 分钟）+ 落盘护栏三层 | `ctx.sandbox` seam（抽象）+ dsh-sandbox-local + SandboxPolicy（read-only/workspace-write/danger-full-access）+ SandboxEnforcement（full/partial）+ ConfinedArgv + RunnerFailureRule；多平台真内核（bwrap + 自研 Landlock C addon + Seatbelt + Windows ACL）；SandboxPolicyService（解析优先级：显式 approved > session sandbox/mode 日志 > 部署默认）；fail-closed SANDBOX_UNAVAILABLE | dsh 跨平台 + 沙箱也是 seam；本项目只在 Rust 进程内 |
| 权限/审批 | WORLD_AI_TOKEN 令牌（写命令需 -t 或环境变量）+ 角色范畴白名单（界主/鸿钧=全量；女娲=目标/规则/世界；多宝=目标/规则/世界/程序；准圣=目标/规则/世界；可见工具按角色） | `ctx.approval` seam + ApprovalRequest(agent/tool/callId/reason/signal) + ApprovalPolicy='ask'/'never'；`approval/asked` → answerer → `approval/decided` 都落盘 log-only；fail-closed unavailable；approval waterfall + permission-presets | dsh 更细：单次 ask + asker/answerer 协议；本项目靠令牌 + 角色白名单 |
| 计划/目标 | 设计方案 + 八态状态机（待领→设计中→待确认→已确认→待实现→实现中→已验收→已存档）+ 需求拆分（最多 4 子要求） | `ctx.planMode`（折叠已记录的计划状态）+ `ctx.goals`（带修订号 + maxGoalRounds 上限 + goal/changed 事件）+ activation 进程本地（resume/fork 后必须经人类授权） | dsh 三态 + 修订号 CAS；本项目八态更全 |
| 工作流 | L1_qa / L2_script / L3_program / L4_complex 四档 | `ctx.workflowEngine`（**单实现每上下文**）+ `dsh-workflow-worker-thread` 默认 provider；script = top-level await + return JSON；`agent()` / `parallel()` / `pipeline()` 组合器；typo 必须死 | dsh 真正的「脚本驱动」；本项目是「步骤清单」 |
| 持久任务 / 队列 | `落盘-取队-园`（jsonl 队列：入队追加 / 取队删除 / 水位 + 八态推进） | `ctx.jobs` seam + LocalJobRegistry；JobId = `<kind>-N` 命名空间；JobStatus（running/stopping/completed/killed/failed）；JobHooks（cancel/done/readOutput）；settlement first-wins；maxConcurrentJobsPerOwner=10 | dsh 更完整（hooks + owner-scoped 并发上限） |
| Web/客户端 | `监控界面-府`（独立 Python 只读工具，端口 3082）读 `观测/记录.jsonl`；axum + tokio + SSE | 完整 client/server：`apps/web/`（Vite 入口）+ `apps/cli/`（tsdown 打成可执行）+ `ctx.webServer`（HTTP 载体）+ `ctx.clientModules`（增量扫描 dsh.client）+ `ctx.hmr` + `ctx.apiProxy` + `ctx.connection` + `ctx.typertGateway` | dsh 完整 web 客户端 + HMR；本项目监控是临时只读工具，正式 UI 占位 |
| Hooks / 扩展 | 无独立 hooks seam | `hooks-claude-code` / `hooks-codex`（hook 桥接，session.md 末尾 `hook/invoked`/`hook/result` 事件）；`extensions` 动态 Cordis 插件（命名身份 + immutable Package 版本） | dsh 有 |
| 调度 | 无独立 schedule seam | `ctx.schedule` 会话本地 ScheduleRecord（After/At/Every）+ 固定频率（最小 5 分钟）+ at-least-once | dsh 有 |

### 2. 工程基线对比

| 项 | 洪荒世界 | deepseek-harness | 差距 |
|:--|:--|:--|:--|
| 类型严格度 | Rust（编译期保证） | TypeScript strict + noImplicitAny + branded ids（`Branded<B>`）+ zod schema（Domain spec + JsonSchemaNode 子集） | dsh 仍可在 type 边界加运行时校验；Rust 更严 |
| 测试金字塔 | 单元测试镜像六府（47 文件） | 单元 692 + 契约（runPersistenceContract / Storage contract）+ e2e + web-stress + perf + snapshot + Python 7 | dsh 完整金字塔 |
| 覆盖率要求 | 项目全景说「约 300 用例全绿」，未设强制覆盖率门 | `test:coverage` 强制每文件 100% 覆盖 | dsh 100% 覆盖 |
| 重复代码检测 | 无 | `jscpd` | — |
| 死代码检测 | 无 | `knip` | — |
| 包元数据校验 | 无（cargo 自带） | `publint` | — |
| Lint | Rust 编译警告即 lint | `oxlint`（更快）+ lefthook | — |
| 双语工程 | 无 | merge driver + blob hash + verify-translation-pairing | dsh 业内罕见 |
| 文档预算 | 无 | `doc-budgets.manifest.json` + `verify-doc-budgets` | — |
| Doc 类型检查 | 无 | `doc-typecheck.ts` + `doc-typecheck-paths.ts` + spec 测试 | — |
| 模块图谱 | 依赖图（符号档案 + 结构树） | `gen-module-graph.ts` + `gen-doc-graphs.ts` + `graph-atlas.md` + `module-graph.md`（双产物） | dsh 双产物（CI + 文档） |
| 持久化目录 catalog | 无 | `gen-persistence-catalog.ts` + `persistence-catalog.md` | — |
| 配置目录 catalog | 无 | `gen-config-catalog.ts` + `config-catalog.md` | — |
| 工具目录 catalog | 无 | `gen-tool-catalog.ts` + `tool-catalog.md` | — |
| 翻译提示词 | 无 | `gen-translation-brief.ts` + `translation-prompt.md` + `translation-pairing.manifest.json` + `merge-translation-pairing*.ts` | — |
| 客户端目录 | 无 | `gen-client-catalog.ts` | — |
| 第三方声明 | 无（无第三方） | `gen-third-party-notices.ts`（自动生成） | — |
| 命名规范自动化 | **洪荒审判 CLI**（7 大规范：九根 / 层级 / 三件套 / 命名 / 依赖 / 双语 / 硬编码；中文输出违规清单；支持 --json） | 无独立命名 gate；通过 review / oxlint / package-invariants.ts / cordis-config-files.ts / check-workspace-constraints.ts 等多个脚本分担 | **本项目反而在命名自动化上有专门 CLI**（中文输出 + 9 维 6 层专属） |
| 鸿蒙审判 | 自有 CLI | 无 | 本项目独有 |
| 文档站点 | 无（md 文件分散） | VitePress（docs 投影双语） | dsh 有 |
| 工作区约束 | Cargo workspace + 自定义 .env | pnpm workspaces + overrides + allowBuilds 白名单 + peerDependencyRules.allowedVersions.typescript | dsh 更严格 |
| 版本管理 | 自有版本存档机制（增量快照 + 版本记录 + 回退） | 完整 release workflow（pack 每 PR 跑，publish 仅 dispatch + `*-v*` tag） | dsh 自动化 |
| Patch 管理 | 无 | `patches/node-pty@1.1.0.patch` | — |
| Vendor 源码内嵌 | 无 | `vendor/{cordis,cosmokit,group,hmr,include,loader,logger-console,schemastery,timer}` 9 包 | dsh 有（vendored as source） |
| Hook 桥接 | 无 | `hooks-claude-code` / `hooks-codex` | — |

### 3. 可演进性对比

| 项 | 洪荒世界 | deepseek-harness | 评价 |
|:--|:--|:--|:--|
| 文档先于代码 | **强制**：「先入稿、再落码」，实现须能在设计稿溯源 | cookbook 体系（adding-a-{package,tool,vendored-package,llm-adapter,conversation-node}），但没有强制 gate | 本项目更严 |
| 活文档 | 设计稿随世界生长 | AGENTS.md + 多篇 README 随版本演进 | 都有 |
| postmortem 文化 | 缺陷-记录-20260819.md（1 篇，集成在传承殿） | 4 篇 postmortem（每篇：Executive summary + 根因 + 教训 + gate 补丁） | dsh 有事故复盘传统 |
| 双语 | 无 | 完整双语工程 | dsh 有 |
| 类型化事件 | 自写 `事件流-府` + append-only + 进程级互斥锁 + 三源归并（白箱） | `session/event` + typed events + 四种分发（emit/waterfall/parallel/serial） | dsh 更精致 |
| 包可替换性 | 府 = 插件单元 + Service Definition trait + 插件注册表（渐进式改造中） | Cordis `service` + `inject` + `bundle`/`profile`/`patch` 层叠（成熟） | dsh 成熟；本项目在改造中（`查找服务::<Arc<dyn 服务trait>>()` 阶段二） |
| 跨包依赖规则 | 跨府止步 lib 根 + 跨维经鸿蒙 + 6 维 6 层结构 | capability seams（SD + Provider + Consumer 三角色）+ 12 条跨包依赖铁律（注册即 effect / 事件 vs 服务 / Plugin 不改 loop / model-visible ⟺ logged / explicit > implicit at package boundaries / typed 边界信任 TS / 分层平面分离 / opaque id brand / 空 catch 必须命名 / 测试断言 owned / 运行时 invariants 归属包 / Registry 贡献证明 disposer） | dsh 规则外化且可机器检查；本项目内置但机器检查有限 |
| 测试断言 owned 关系 | 「看实际产出内容（完整字段、完整提示词、落盘记录原文），而非代码定义」 | 「测试断言 owned 关系：看真实事件流/可变数据，不看 service/method 存在或固定示例」 | 思路一致 |
| 错误码 | 内部字符串 | 稳定错误码（FS_NOT_FOUND / SESSION_QUERY_* / SANDBOX_UNAVAILABLE / TOOL_OUTPUT_ERROR 等 12+ 类） | dsh 更工程化 |

---

## 三、本项目独特的优势（dsh 没有的）

| 项 | 说明 |
|:--|:--|
| **九维六层结构** | 维度/域/府/殿/阁/园 六层 + 九根（本体 5 根 + 工程 4 根）。dsh 是扁平 monorepo（packages/<group>/<pkg>），没有这种「宇宙观 + 层级 + 命名逐段递进」的强结构。 |
| **洪荒审判 CLI** | 自有命名/层级/三件套/依赖/双语/硬编码自动化审计。dsh 没有同等聚焦的工具（oxlint/lint-rule-fingerprint.spec.ts 接近，但覆盖面窄）。 |
| **世界观建模** | 界主/天道/鸿钧/女娲+四圣/大罗金仙/六准圣 等角色体系（已落进 `daoshu_fu` 角色卡）+ 八态状态机 + 两阶段制（甲/乙）+ 进化环（⑤级调整）。dsh 是单 agent 模型，没有多角色世界观。 |
| **格位 36/72 + 六范畴** | 目标/规则/自我/程序/世界/经历 + 经/权固化度 + 共享/私有 + 最前/中间/最后顺序档 + 临时代偿。dsh 没有「固化度 + 私有」这样的人工约束维度。 |
| **依赖图（符号档案 + 结构树）** | 项目级符号档案 + 结构树 + 查涉及文件 + 补全同阁 + 下探。dsh 的 module-graph 是文档产物（md/svg），不参与运行时检索。 |
| **人类格位临时代偿** | 人类格位未录时 LLM 探索项目生成临时内容，人类可覆盖为经。dsh 没有这个机制。 |
| **「阴」守护进程 / 十二时辰 / 「磨蹭」「抽风」「空转」** | 文化层面的彩蛋，让工程系统有生命力。 |
| **白箱可观测** | `观测探针-府` 五哨兵域（提示词/回复思考/工具调用/工具返回/产物判定）+ 线程本地观测上下文按角色边界自动贯穿。dsh 有 session-telemetry，但本项目把观测做了「角色链贯穿」。 |
| **回滚垫（事务级）** | 写操作前快照 + 任务失败单文件撤销 + 「曾存在=false」处理新增。dsh 没有同等的事务级撤销（只有版本回退）。 |
| **教训沉淀机制** | 失败按要求id+阶段去重累加 → 教训格位 → 执行前注入现状。dsh 没有同等。 |
| **「白嫖」等待期** | 趁 LLM 思考空档做免费地道收尾。dsh 没有这种利用并行空档的设计。 |

---

## 四、本项目的真实差距（按优先级排序）

| 优先级 | 差距 | 影响 | 建议 |
|:--|:--|:--|:--|
| **P0 必补** | 测试规模 47 vs 692（**14.7×**） | 任何改动都缺乏回归保护；类型化事件流/Projection 等核心机制仅有镜像测试 | 大幅扩充单元测试 + 加契约测试（每个府对外契约固定 + 共享）+ 加端到端测试（号令下达 → 主政 → 设计 → 实现 → 验收 → 定档） |
| **P0 必补** | 工程门禁脚本 = 0 vs 142 | 改名/改路径/改依赖无人守门；新人接手门槛高 | 写一批 `verify-*/check-*/gen-*` 脚本（Cargo workspace 配 lefthook）：verify-命名一致、verify-六层结构、verify-府内模块桥接、check-无死代码、check-无硬编码、gen-依赖图可视化、gen-模块图、check-文档预算 |
| **P0 必补** | 0 个 CI workflow | 无 PR 校验、无自动 build、无 nightly e2e | 至少 3 个：ci.yml（cargo check --workspace + cargo test --workspace）、e2e.yml（命令操作-府端到端）、release.yml（每 PR 自动 pack） |
| **P0 必补** | 无 release / publish 流水线 | 任何对外分发的能力都没有 | 写 release.yml + release-train 规则（与 dsh 的 dsh-v* tag 一致） |
| **P1 应补** | 双语工程 = 0 | 国际化、对外发布、英文圈引用都做不到 | 至少把核心 6 份设计稿（层级结构-设计/多智能体架构/项目心智模型/融合蓝图/上下文/智能体）+ README/AGENTS 做双语 i18n 三件套（*.md + *.zh.md + *.i18n.yaml） |
| **P1 应补** | 沙箱只覆盖 Rust 进程内 | 跨平台用户无法使用 | 至少做：Windows ACL 限制令牌（本地已是 Windows）+ Linux bwrap 封装（与 dsh 同思路但简化） |
| **P1 应补** | 0 个 cookbook / postmortem 体系 | 团队/AI 协作范式没外化 | 写「adding-a-{府,殿,阁,园}」「adding-a-{命令,工具,角色,格位}」cookbook + 把缺陷-记录-20260819 改写为 postmortem 模板（Executive summary + 根因 + gate 补丁） |
| **P1 应补** | 无 vendored 框架源码 | 关键依赖（tracing / tokio / axum）的版本变更无法精准控制 | 把核心依赖的源码 snapshot 拉到 `.cargo/vendor/`（或 vendor.txt 锁定），必要时回滚或修补 |
| **P1 应补** | 无工具目录 / 配置目录 / 持久化目录 catalog | 工具/配置/数据散落，新人/AI 难以一目了然 | 写 gen-tool-catalog.rs + gen-config-catalog.rs + gen-persistence-catalog.rs + 对应 .md 文档 |
| **P1 应补** | 无第三方声明 THIRD_PARTY_NOTICES | 协议合规风险 | 引入 cargo-about 或手写一份（按依赖逐项） |
| **P2 应补** | 无 Web 完整客户端 | 监控界面-府只是临时只读工具 | 写 client 包（结构同 dsh 的 client/：connection + hmr + locale + modules + runtime + schema-form + ui-*），跟监控界面-府共用 axum + SSE |
| **P2 应补** | 无 hooks / extensions 桥接 | Claude Code / Codex 用户的 hook 习惯无法继承 | 写 hooks-claude-code / hooks-codex 桥（订阅 agent.hook-protocol） |
| **P2 应补** | 错误码字符串化 | 国际化 / 自动化处理困难 | 给所有 `Err(String)` 改造为 `Err(thiserror::Error + From)`，错误码统一为 `FS_NOT_FOUND` / `SANDBOX_DENIED` / `QUEUE_EMPTY` 等稳定 kebab-case |
| **P3 可缓** | 无 VitePress 文档站 | 文档只是文件 | 等 P1 双语工程完成后再上站点 |
| **P3 可缓** | 无 Code Mode（模型写代码调工具） | 模型必须按工具循环一条条调用 | 可借鉴 dsh 的 `ctx.codeRuntime` seam |
| **P3 可缓** | 无 sandbox YAML profile 机制 | 部署组合需改源码 | 引入 cordis 风格的 `.yaml` 配置启动（profile + patch） |

---

## 五、为什么差距这么大是合理的

| 维度 | 原因 |
|:--|:--|
| **目标差异** | dsh 是「公开发布给外部开发者」的 agent harness，要面向 npm 包消费者、写完整 cookbook、维护 postmortem、做双语。本项目是「世界自己写代码、自己验收、自己存档」的多智能体自进化系统，目标是世界自己，不是面向外部开发者。 |
| **代码量 vs 闭包度** | 本项目 3.4 万行 Rust 跑通「界主想法 → 鸿钧化要求 → 圣人设计 → 大罗实现 → 准圣验收 → 鸿钧终裁 → 版本存档 → 天道巡世」的完整闭环；dsh 23 万行 ts 跑通「一个 agent 的完整生命周期（turn / step / 工具调用 / 沙箱 / 持久化 / 投影）」的完整基础设施。两者闭环深度不在一个维度。 |
| **结构 vs 灵活** | 本项目用 9 维 6 层强结构 + 命名逐段递进 + 洪荒审判 CLI 守门，**机器可控性极强**；dsh 用扁平 monorepo + 大量 cookbook + review 文化 + 多个 verify-* 脚本分担检查，**人类可控性极强**。前者更适合 AI 自进化，后者更适合人类协作。 |
| **语言差异** | Rust 编译期保证类型/内存/借用安全（缺测试也能跑通大部分 case）；TypeScript 需靠测试 + 严格 lint 守门，所以 dsh 必须配 692 个 spec。本项目测试规模小不是因为懒，是因为 Rust 编译器和 47 个镜像测试已经守住底线。 |

---

## 六、给本项目的「不发布」改进清单（最小可发布单元）

> 界主明示「本项目还未到可以发布的质量」，故下面只列「达到 dsh 当前质量的 50%」所需项，不要求 100% 追平。

### 第一波（必须做，否则上不了 GitHub）

1. 至少 **3 个 CI workflow**：`ci.yml`（cargo check + cargo test + cargo clippy）、`e2e.yml`（号令下达 → 主政 → 设计 → 实现 → 验收 → 定档 跑通示例想法）、`docs.yml`（设计稿字数/链接检查）
2. 至少 **10 个 verify-*/check-*/gen-* 脚本**：verify-命名一致、check-无硬编码绝对路径、check-无空 catch、check-无 IDE 配置残留、gen-依赖图可视化、gen-模块图、check-文档预算、verify-AGENTS 一致性、verify-Cargo.toml 依赖治理、check-无重复代码
3. **双语 i18n 三件套**：6 份核心设计稿 + README + AGENTS + COOKBOOK 至少 10 份文档做 `.md` + `.zh.md` + `.i18n.yaml` 三件套，配 `verify-translation-pairing` 门禁
4. **postmortem 模板**：把 `缺陷-记录-20260819.md` 改写为正式模板（Executive summary + 根因 + gate 补丁），并发 1 篇新 postmortem 走完流程
5. **THIRD_PARTY_NOTICES.md** 自动生成（cargo-about 或简版手维护）
6. **Cargo workspace 治理**：写 `deny.toml`（cargo-deny 检查依赖 license / advisory / sources / bans）
7. **AGENTS.md 加 lint / format / test 命令清单**：让任何协作者一键跑通

### 第二波（应做，发布前补完）

8. 测试规模从 47 → **≥ 200**（每府对外契约固定 + 共享契约测试 + 集成 e2e）
9. 错误码统一化（`Err(String)` → `Err(thiserror::Error)` + 稳定码）
10. 沙箱跨平台：Linux bwrap + Windows ACL 强化
11. Web 客户端最小可用版：把 `监控界面-府` 升级为正式 UI（不是临时只读工具）
12. release train：tag + 自动 pack（cargo package）+ 自动 changelog

### 第三波（可缓，发布后持续补）

13. Cookbook 体系（adding-a-{府,殿,阁,园,命令,工具,角色,格位}）
14. Hooks 桥接（Claude Code / Codex）
15. Code Mode seam
16. VitePress 文档站点

---

## 七、结论

**质量差距的核心是「自动化门禁 + 测试金字塔 + 工程脚本 + 双语工程」，不是「架构好不好」。**

- **架构层面**：本项目的 9 维 6 层 + 36 格位 + 八态状态机 + 多角色世界观 + 回滚垫 + 临时代偿 + 五哨兵域观测，是 dsh 没有的**独有抽象**。两者思路殊途，**没有高下之分**，只是 dsh 更工程化、本项目更认知化。
- **工程基线层面**：本项目测试规模只有 dsh 的 **1/14**、工程脚本只有 dsh 的 **1/142**、CI 是 **0/15**、双语工程是 **0/547+392+392**。这是真实的、可量化的、需要补的差距。
- **可达发布的最短路径**：第一波 7 项，按本项目 1-2 周的迭代节奏可完成。完成即可对内发布（即「世界给界主自己看」），但还达不到对外发布的标准。

---

*与deepseek-harness质量对比 · 2026-08-22 · 依据洪荒审判 + 实测统计 + Explore 调研报告 · 随世界生长更新*