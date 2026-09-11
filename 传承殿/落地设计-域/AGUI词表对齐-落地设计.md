# AG-UI 词表对齐 · 落地设计（阶段三）

> 定位：把对外事件流（SSE）从「中文词表 + 扁平 KV」升级为 **AG-UI 标准事件流**，使前端渲染层可与官方 AG-UI SDK 互通；采用**适配器映射**策略，内部存储结构与混沌事件模型**零改动**。

- 文档型：落地设计（实施契约）
- 版本：1.0
- 域：鸿蒙/基础契约-域（交互协议）+ 鸿蒙/基础能力-域（数据服务适配器）〔原含 乾坤/界面呈现-域（渲染层），该前端域已随 2026-09-11 前端一刀切移除〕
- 更新：2026-09-08

> **前端移除补注（2026-09-11）**：本文档中，**后端 AG-UI 契约（hm-agui）与数据服务适配器/端点**部分已实施且仍有效；**前端渲染层**相关内容（第七节 `render_agui`、第六节末尾「前端迁移后旧端点下线」、第九节第 4 步与「文件级落点·前端」）随 2026-09-11「前端一刀切」重构移除，保留历史留档。对外消费方统一称「客户端」。

---

## 一、背景与目标

### 背景
阶段一已把任务过程事件从短轮询升级为 SSE（`/api/dev/stream`），阶段二已把道祖对话升级为 SSE 流式打字机（`/api/dev/chat/stream`，已用 `RUN_STARTED/TEXT_MESSAGE_CONTENT/RUN_FINISHED` 词）。但对外事件流仍存在两处与 AG-UI 标准的差距：

1. **词表不标准**：过程事件沿用中文类型词（思考/工具调用/工具结果/任务答复），阶段事件沿用（空闲/阶段完成/错误），前端需自定义分发逻辑，无法接入官方 AG-UI SDK。
2. **结构不齐**：缺 `messageId/toolCallId/runId/threadId` 关联字段；缺 `TEXT_MESSAGE_START/END`、`TOOL_CALL_START/ARGS/END` 的三段式边界；混沌事件 `载荷: Vec<(String,String)>` 扁平 KV 无法承载 `STATE_DELTA` 的 RFC 6902 JSON Patch 与 `TOOL_CALL_ARGS` 的嵌套增量。

### 目标
1. 新建 `hm-agui` 契约 crate，定义 AG-UI 标准事件类型与序列化。
2. 在数据服务府加**适配器**，把内部事件（驱动过程/阶段事件）翻译成 AG-UI 标准事件，经新增 SSE 端点对外输出。
3. 前端渲染层按 `type` 分发渲染，与官方 AG-UI SDK 的事件模型对齐。
4. **非破坏**：内部事件结构、混沌 Event、会话落盘、旧接口、证道测试全保留。

### 决策（已确认）
- 演进策略：**适配器映射**（不改内部，SSE 输出层翻译）。
- 落点：**新建 hm-agui 契约 crate**（鸿蒙/基础契约-域/交互协议-府）。
- 思考词表：**REASONING_MESSAGE_\***（不用已弃用的 THINKING_*）。

---

## 二、现状差距盘点（只读依据）

| 项 | 现状 | AG-UI 标准 | 差距 |
|---|---|---|---|
| 过程事件类型 | 思考/工具调用/工具结果/任务答复（`驱动过程事件.类型:String`） | REASONING_MESSAGE_* / TOOL_CALL_* / TEXT_MESSAGE_* | 词表不同 |
| 阶段事件类型 | 空闲/阶段完成/错误（`驱动阶段事件.类型:String`） | STEP_STARTED/FINISHED、RUN_ERROR | 词表不同 |
| 关联 id | 仅 `序号`/`任务id`，无 messageId/toolCallId | 每条消息/工具调用需稳定 id 贯穿 Start→Content→End | 缺 |
| 三段式边界 | 单条事件 = 完整事实记录 | Start → Content×N → End | 需展开 |
| 嵌套 delta | `内容:String`（截断200）；混沌 `载荷:Vec<(String,String)>` | `STATE_DELTA.delta` 为 JSON Patch；`TOOL_CALL_ARGS.delta` 为字符串增量 | 承载不了 |
| 对话流 | 已用 RUN_STARTED/TEXT_MESSAGE_CONTENT/RUN_FINISHED | 需补 TEXT_MESSAGE_START/messageId/runId/threadId | 部分齐 |

---

## 三、AG-UI 标准事件词表（本阶段采用子集）

> 来源：[AG-UI Events 规范](https://docs.ag-ui.com/concepts/events)。只引入当前五行驱动实际用到的事件，不引入 ACTIVITY_*、SUBAGENT_*、CUSTOM 等未用类型（避免过度设计）。

### 生命周期
| 事件 | 关键字段 |
|---|---|
| RUN_STARTED | threadId, runId, parentRunId?, input? |
| RUN_FINISHED | threadId, runId, result? |
| RUN_ERROR | message, code? |
| STEP_STARTED | stepName |
| STEP_FINISHED | stepName |

### 文本消息
| 事件 | 关键字段 |
|---|---|
| TEXT_MESSAGE_START | messageId, role? |
| TEXT_MESSAGE_CONTENT | messageId, delta |
| TEXT_MESSAGE_END | messageId |

### 工具调用
| 事件 | 关键字段 |
|---|---|
| TOOL_CALL_START | toolCallId, toolCallName, parentMessageId? |
| TOOL_CALL_ARGS | toolCallId, delta |
| TOOL_CALL_END | toolCallId |
| TOOL_CALL_RESULT | toolCallId, messageId?, content |

### 思考（推理）
| 事件 | 关键字段 |
|---|---|
| REASONING_MESSAGE_START | messageId |
| REASONING_MESSAGE_CONTENT | messageId, delta |
| REASONING_MESSAGE_END | messageId |

### 状态
| 事件 | 关键字段 |
|---|---|
| STATE_SNAPSHOT | snapshot（任意 JSON） |
| STATE_DELTA | delta（RFC 6902 JSON Patch 数组） |

---

## 四、适配器映射表（内部 → AG-UI）

适配器输入为内部事件（`驱动过程事件`/`驱动阶段事件`），输出为 AG-UI 事件序列。**单条内部事件展开为三段式**（内容整段作为一个 delta，粒度对齐「结构」，非逐 token——逐 token 流式见第八节可选演进）。

### 4.1 过程事件映射

| 内部类型 | 输出 AG-UI 序列 |
|---|---|
| 思考 | `REASONING_MESSAGE_START{messageId}` → `REASONING_MESSAGE_CONTENT{messageId, delta=内容}` → `REASONING_MESSAGE_END{messageId}` |
| 工具调用 | `TOOL_CALL_START{toolCallId, toolCallName=工具名}` → `TOOL_CALL_ARGS{toolCallId, delta=内容}` → `TOOL_CALL_END{toolCallId}` |
| 工具结果 | `TOOL_CALL_RESULT{toolCallId, content=内容}`（toolCallId 用上一工具调用同一 id，见 4.3） |
| 任务答复 | `TEXT_MESSAGE_START{messageId, role="assistant"}` → `TEXT_MESSAGE_CONTENT{messageId, delta=内容}` → `TEXT_MESSAGE_END{messageId}` |

### 4.2 阶段事件映射

| 内部类型 | 输出 AG-UI 序列 |
|---|---|
| 阶段完成 | `STEP_FINISHED{stepName=新状态 或 层级}` |
| 空闲 | `RUN_FINISHED{runId, result=最近结果}`（本轮收尾） |
| 错误 | `RUN_ERROR{message=消息, code="DRIVE_ERROR"}` |

### 4.3 关联 id 生成策略（适配器补全，确定性、可回放）

| 字段 | 生成规则 | 说明 |
|---|---|---|
| runId | `run-{会话id}` | 一次驱动会话（`驱动会话id`）对应一个 run |
| threadId | `thread-{会话id}` | 会话回放/恢复/分叉共用同一 thread |
| messageId | `msg-{会话id}-{序号}` | 思考/任务答复各自一条，序号唯一 |
| toolCallId | `tc-{会话id}-{序号}` | 工具调用；工具结果复用**同轮次最近一次工具调用的 toolCallId** |

> 工具结果与工具调用的配对：适配器维护「最近一次工具调用 toolCallId」游标，遇到「工具结果」即用它作为 `toolCallId`。确定性、无状态外依赖、可随会话回放重建。

### 4.4 对话流补全（chat/stream）

阶段二 `chat/stream` 已输出 `RUN_STARTED/TEXT_MESSAGE_CONTENT/RUN_FINISHED`，本阶段补齐合规字段：

```
data: {"type":"RUN_STARTED","threadId":"thread-0","runId":"run-接待"}
data: {"type":"TEXT_MESSAGE_START","messageId":"msg-接待-1","role":"assistant"}
data: {"type":"TEXT_MESSAGE_CONTENT","messageId":"msg-接待-1","delta":"好的，"}
...
data: {"type":"TEXT_MESSAGE_END","messageId":"msg-接待-1"}
data: {"type":"RUN_FINISHED","threadId":"thread-0","runId":"run-接待","result":{"任务id":3}}
```

---

## 五、hm-agui 契约结构设计

新建 crate `hm-agui`，落「鸿蒙/基础契约-域/交互协议-府」。

```
交互协议-府 (Cargo.toml, 模块.rs)                     # crate 名 hm-agui
├ 事件词表-殿 (模块.rs)
│  ├ 生命周期-阁 (模块.rs)
│  │  └ 运行-事件-园 (模块.rs, 运行事件.rs)            # RunStarted/RunFinished/RunError/StepStarted/StepFinished
│  ├ 文本消息-阁 (模块.rs)
│  │  └ 文本-事件-园 (模块.rs, 文本事件.rs)            # TextMessageStart/Content/End
│  └ 工具调用-阁 (模块.rs)
│     └ 工具-事件-园 (模块.rs, 工具事件.rs)            # ToolCallStart/Args/End/Result
├ 思考推理-殿 (模块.rs)
│  └ 推理-阁 (模块.rs)
│     └ 推理-事件-园 (模块.rs, 推理事件.rs)            # ReasoningMessageStart/Content/End
└ 状态同步-殿 (模块.rs)
   └ 快照增量-阁 (模块.rs)
      └ 状态-事件-园 (模块.rs, 状态事件.rs)            # StateSnapshot/StateDelta
```

### 5.1 类型设计（核心 API）

```rust
/// 事件类型枚举：AG-UI 标准词（串行化为大写蛇形）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum 事件 {
    RunStarted(RunStarted),
    RunFinished(RunFinished),
    RunError(RunError),
    StepStarted(StepStarted),
    StepFinished(StepFinished),
    TextMessageStart(TextMessageStart),
    TextMessageContent(TextMessageContent),
    TextMessageEnd(TextMessageEnd),
    ToolCallStart(ToolCallStart),
    ToolCallArgs(ToolCallArgs),
    ToolCallEnd(ToolCallEnd),
    ToolCallResult(ToolCallResult),
    ReasoningMessageStart(ReasoningMessageStart),
    ReasoningMessageContent(ReasoningMessageContent),
    ReasoningMessageEnd(ReasoningMessageEnd),
    StateSnapshot(StateSnapshot),
    StateDelta(StateDelta),
}
```

> 用 `#[serde(tag = "type")]` 的邻接标记，序列化自动产生 `{"type":"TEXT_MESSAGE_CONTENT","messageId":...,"delta":...}`，与 AG-UI 帧格式逐字节一致。`delta`（STATE_DELTA 的 JSON Patch）用 `serde_json::Value` 承载嵌套结构，规避混沌 Event 扁平 KV 的局限。

### 5.2 合规校验
- 域：鸿蒙 `基础契约-域` 下新建第 7 府（域下府数不限，合规）。
- 府名「交互协议」≠ 域名「基础契约」 ✓；殿/阁/园语义词均不与直接上级同名 ✓。
- 府下殿 3 ≤ 6、殿下阁 ≤ 4 ✓。
- crate 名 `hm-agui`（鸿蒙=hm + 英文语义 agui）✓；无 `src`、无英文目录 ✓。

---

## 六、端点设计（非破坏：旧端点保留）

| 端点 | 现状 | 阶段三后 |
|---|---|---|
| `GET /api/dev/stream` | 输出中文词过程事件 | **保留不动**（旧客户端/证道） |
| `GET /api/dev/stream/state` | 输出中文词阶段事件 | **保留不动** |
| `POST /api/dev/chat` | 同步整体返回 | **保留不动** |
| `POST /api/dev/chat/stream` | 半合规流式 | **补齐 messageId/START/END/runId** |
| `GET /api/dev/stream/agui` | — | **新增**：过程事件 → AG-UI 标准词（适配器） |
| `GET /api/dev/stream/agui/state` | — | **新增**：阶段事件 → STEP/RUN 标准词 |

> 新增 `/agui` 端点走适配器，复用现有 `过程事件增量(since)`/`驱动事件增量(since)` 与环形缓冲，仅换输出层序列化。前端迁移到 AG-UI 渲染器后，旧端点可择机下线（不在本阶段）。

---

## 七、前端渲染层

乾坤 `对话流.js` 新增**按 type 分发的渲染器**（`render_agui(ev)`），替代现有「按中文类型词 + 工具名」的分支：

| type | 渲染动作 |
|---|---|
| RUN_STARTED / RUN_FINISHED / RUN_ERROR | 任务卡头状态位 / 顶徽章 |
| REASONING_MESSAGE_START | 新建思考折叠块 |
| REASONING_MESSAGE_CONTENT | 追加思考文本 |
| REASONING_MESSAGE_END | 折叠块收尾 |
| TOOL_CALL_START | 新建工具卡（名 = toolCallName） |
| TOOL_CALL_ARGS | 追加参数 |
| TOOL_CALL_END / TOOL_CALL_RESULT | 工具卡标记完成 + 结果 |
| TEXT_MESSAGE_START/CONTENT/END | 答复气泡打字机 |
| STATE_SNAPSHOT / STATE_DELTA | 五行阶段卡状态位 |

> 前端与官方 AG-UI SDK 事件模型对齐后，可直接换用官方 `@ag-ui/client` 消费（可选，不强制）。

---

## 八、可选演进（不在本阶段）

1. **驱动过程逐 token 流式**：当前驱动循环用一次性生成（`内容生成器::生成`），过程事件是整段记录。若让驱动循环改用阶段二的 `流式对话器`（或新增 `流式生成器`），思考/答复可真正逐 token 推送，delta 粒度达到 AG-UI 原生水平。依赖内容生成层扩展，另行设计。
2. **STATE_DELTA 落五行**：把五行阶段状态以 JSON Patch 增量推送，替代现有「阶段事件 → 阶段卡」的整段推进，需在驱动循环埋状态变更探针。
3. **人工中断闸门（Interrupt/Resume）**：现有「先审后写/接受拒绝」走独立 POST 接口；如需对齐 AG-UI 的人机交互（HUMAN_INTERRUPT），需引入中断续跑事件，另行设计。

---

## 九、实施顺序与落点

### 顺序
1. **契约先行**：新建 hm-agui crate（事件枚举 + 各 struct + 序列化），`cargo build` 通过。
2. **适配器**：数据服务府新增「协议-适配-园」/「协议-事件-园」，实现 `内部事件 → 事件` 翻译 + id 游标。
3. **端点**：新增 `/api/dev/stream/agui`、`/api/dev/stream/agui/state`；补全 `chat/stream` 字段。
4. **前端**：新增 `render_agui` 分发渲染器，订阅 `/agui` 端点。
5. **验收**：证道增 AG-UI 端点端到端用例（府内不写 `#[cfg(test)]`）。

### 文件级落点
- 契约：鸿蒙/基础契约-域/交互协议-府（hm-agui，新建）。
- 适配器 + 端点：鸿蒙/基础能力-域/数据服务-府（路由模块.rs + 新适配器园 + 驱动处理.rs）。
- 前端：乾坤/界面呈现-域/门面承载-府/功能组件-殿/对话流-阁/对话-组件-园/对话流.js。
- 测试：证道/基础验证-域。

### 约定
- 阶段三实施完成即更新本文档版本与修订记录。
- 适配器映射为纯函数（输入内部事件、输出 AG-UI 事件序列），便于证道单测；不得在适配器内做网络/锁操作。

---

## 十、风险与偏离说明

| 项 | 说明 |
|---|---|
| 偏离文档 4.3①/②（阶段二） | 阶段二曾定义 `流式对话器` 替代 `流式生成器`；本阶段契约 hm-agui 与 `hm_content_contract` 独立，不互相依赖（契约层单向：hm-agui 仅依赖 serde/serde_json + hm-error，不依赖内容契约）。 |
| 词表范围裁剪 | 不引入 ACTIVITY_*、SUBAGENT_*、CUSTOM、RAW、弃用的 THINKING_*，只覆盖当前实际用到的事件，避免过度设计。 |
| delta 粒度 | 适配器展开的 delta 为「整段」而非「逐 token」，因内部过程事件本就是整段记录；逐 token 流式列入可选演进，不在本阶段。 |
| 工具结果配对 | 依赖「最近一次工具调用 toolCallId」游标，若出现无前置工具调用的孤立工具结果，适配器回退用 `tc-{会话id}-{序号}` 兜底，不 panic。 |
