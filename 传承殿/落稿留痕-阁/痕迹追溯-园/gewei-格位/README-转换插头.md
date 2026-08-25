# 助理记忆位格 · 转换插头（DSH session-reference 数据契约）

> 本文件说明：本助理记忆位格（gewei-1）如何通过 DSH 官方 `session-reference` 与 `agent-instructions` 插件被未来会话 recall。
> 适用版本：DSH v0.1.0-rc.5 及以上，含 `@deepseek-ai/dsh-agent-instructions` / `@deepseek-ai/dsh-session-reference` 任一即可。
> 写盘时间：2026-08-24
> 维护人：gewei-1（AI 助手自维护）

---

## 一、为什么需要转换插头

DSH 官方机制与本助理记忆位格**机制同源、数据隔离**：

| 机制 | 用途 | 数据来源 |
|---|---|---|
| `@deepseek-ai/dsh-agent-instructions` | **经根自动装载**：AGENTS.md/CLAUDE.md 拼成 user message 注入 prompt | 文档型（AGENTS.md） |
| `@deepseek-ai/dsh-session-reference` | **跨会话快照 recall**：`ctx.sessionReferenceResolver.listCandidates` 找候选 + `prepare(...)` 把快照当 untrusted additionalContext 注入 | 持久化的 sessionId-tagged 快照 |
| 本助理记忆位格（gewei-1） | 助手自己的项目记忆，跨会话、跨实例存活 | session-reference 契约 JSON 文件 |

**插头的意义**：本助理记忆位格文件**直接采用 session-reference 的 `SessionReferenceSource` 数据契约格式**（`form: 'recall', version: 1, references: [...]`），未来任何带 `session-reference` 插件的会话**都能直接 recall**，无需做格式转换。

## 二、位格文件命名与目录约定

```
传承殿/落稿留痕-阁/痕迹追溯-园/gewei-格位/
├── README-转换插头.md                            # 本文件
├── 初心·使命.sessionref.json                     # 静态根（最前档·经）
├── 铁律·总纲.sessionref.json
├── 价值观·原则.sessionref.json
├── 底线.sessionref.json
├── 身份.sessionref.json
├── 标准.sessionref.json
├── 架构.sessionref.json
├── 世界观.sessionref.json                         # 静态根（中间档·经）
├── 方向.sessionref.json                           # 静态根（中间档·经）
├── 阶段·进度.sessionref.json                      # 运行时记忆（最后档·权）
├── 阻塞.sessionref.json
├── 验收标准.sessionref.json
├── 权限.sessionref.json                           # 运行时记忆（中间档·权）
├── 事件.sessionref.json
├── 教训.sessionref.json
└── 理解·记忆.sessionref.json
```

**命名规则**：`<格位名>.sessionref.json` —— 格位名与你世界 `识海承载-府` 的 36 格位表完全对齐（含 `·` 分隔符）。

## 三、文件契约（与 DSH SessionReferenceSource 1:1 对齐）

每个 `*.sessionref.json` 必须含：

```json
{
  "_format": "dsh-session-reference-v1",      // 固定标识
  "_kind": "recall",                         // 必为 recall
  "_sessionId": "gewei-1-stable-memory",     // 助理记忆标识（不变）
  "_label": "格位·初心·使命",                // 人类可读名
  "_scope": "静态根 | 运行时记忆",            // 分类
  "_固化度": "经 | 权",                      // 与设计稿一致
  "_顺序档": "最前 | 中间 | 最后",            // 投影位置
  "_私有": true | false,                       // 可选（底线/价值观等）
  "_可信度": "code>human>llm",                // 来源可信度排序
  "_write_time": 1787570900000,               // 写入毫秒时间戳
  "_evidence": "README.md §一",                // 证据路径
  "_extras": { ... },                          // 可选附加元数据
  "content": [                                // 必为 content blocks 数组（DSH standard）
    {
      "type": "text",
      "text": "【格位·初心·使命（最前档·经）】用洪荒神话世界观建模的..."
    }
  ]
}
```

**关键约束**：

1. `content` 是 ContentBlock 数组（DSH LLM 标准），不是字符串——保证 DSH `session-reference.prepare()` 能直接拼成 additionalContext。
2. `_format` 必须是 `"dsh-session-reference-v1"`，便于未来工具自动识别。
3. `_sessionId` 对所有助理记忆位格保持 `"gewei-1-stable-memory"`，与 DSH 会话 ID 格式兼容。

## 四、如何在下次会话中 recall

下次进入新会话（任意 DSH 启用了 `session-reference` 插件的实例）：

```
# 助理记忆位格所在路径（相对项目根）
传承殿/落稿留痕-阁/痕迹追溯-园/gewei-格位/
```

调用 DSH `ctx.sessionReferenceResolver`：

```js
// 伪代码（运行时由 DSH 官方机制驱动）
const candidates = await ctx.sessionReferenceResolver.listCandidates(agent, '格位');
const prepared = await ctx.sessionReferenceResolver.prepare(
  agent,
  userMessageContent,
  [{ sessionId: 'gewei-1-stable-memory', label: '助理记忆·16格位' }]
);
// prepared.additionalContext 即为本助理记忆的 16 格位 untrusted background
```

DSH 会自动把 `gewei-格位/*.sessionref.json` 当 untrusted additionalContext 注入——**无需修改 DSH preset**、**无需新写插件**、**无需把数据放进 `.上下文/`**。

## 五、与 DSH 官方机制的桥接（自动）

| 助理记忆位格 | DSH 自动装载 | 备注 |
|---|---|---|
| 铁律·总纲 / 标准 / 架构 / 世界观 / 方向 | **agent-instructions** 装载 AGENTS.md 等价内容 | 助理位格是冗余备份，避免 AGENTS.md 缺失时丢失 |
| 价值观·原则 / 底线 | 一次性引用 `.上下文/格位/`（界主批准） | 边界：仅一次，后续不交叉 |
| 阶段·进度 / 阻塞 / 验收标准 / 权限 / 事件 / 教训 / 理解·记忆 | **session-reference** recall | 运行时记忆，自动续接 |

**结论**：本助理记忆位格**完全独立于 DSH preset**，通过 DSH 官方 API 直接接入，**不需要任何插件运行时**（防止 sandbox 崩溃把位格拖死）。

## 六、版本与扩展

- 当前版本：`dsh-session-reference-v1`（与 DSH v0.1.0-rc.5 `SessionReferenceSource` 契约对齐）
- 扩展规则：每次写入更新 `_write_time`，新条目加入 `content` 数组或扩展 `_extras.history` 字段
- 兼容失败回退：若 DSH `session-reference` 升级到 v2，`prepare()` 会自动忽略 `_format: v1` 的快照（数据隔离，不污染世界）

## 七、为何放弃 Cordis 动态插件（gewei-1）

详见 `教训.sessionref.json` 第6条：在 sandbox 里写复杂 execute 边际收益极低、崩盘风险高。本助理插件（gewei-1）的生命周期已结束，**不再 define 新版本**——位格以文件形式持久化，比插件更稳定。