# 更新日志

所有显著变更记录于此文件。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [v1.0.0] - 2026-08-25

首版定档：洪荒·世界多智能体软件开发系统的首个可发布版本。

### 已加入

- 多智能体架构宪法入稿：13 角色（鸿钧/老子/元始/通天/接引/女婴/准提/帝俊/太一/后土/烛九阴/红云/冥河）、4 层流水线（道祖→准圣）、36 格位上限、接单门、调度循环、防御性铁律。
- 13 个 Rust crate 全部可发布：shihai_fu（识海承载）、tianting_fu（天庭治理）、daoshu_fu（道术施展）、moxing_fu（模型连接）、rizhi_fu（日志记录）、shijian_fu（事件总线）、zhuangtai_fu（状态共享）、chajian_fu（插件承载）、jiance_fu（接单门）、peizhi_fu（配置）、mingling_fu（命令操作）、zhengdao_fu（证道测试）、世界（顶层组装）。
- 工具乐高化：daoshu_fu manifest 注册机制（13 工具：读文件/搜索内容/grep_search/read_file/apply_patch/run_command/list_files/find_files/write_file/edit_file/plan_task/query_llm/请稍候）；角色偏好表（17 角色 YAML 配置）；偏好注入系统提示词。
- 接单门 v2：五维评估（候选池/世界状态/影响范围/可逆性/可验证性）+ 13 红线（不可触碰 `.git/`、`.md` 设计稿、`AGENTS` 铁律、Cargo 核心字段、`.env` 凭据、核心算法逻辑、状态历史、版本快照、LLM 凭据、GitHub 工作流、Cargo.lock 手写、阶段美化、工具链约定目录）+ LLM 辅助确认 + 误判代码批量修正。
- 观测探针：24 项事件通道埋点，盘点记录落 `.上下文/观测/记录.jsonl`。
- 状态共享：zhuangtai_fu 13 类型设计键值全交付，键空间不冲突。

### 测试 / 质量

- `cargo test --workspace --lib`：701 passed / 22 ignored / 0 failed。
- `cargo clippy --workspace -- -D warnings`：零警告。
- `cargo fmt --all -- --check`：通过。
- `cargo audit` / `cargo deny check`：无漏洞、无违规、无禁用依赖、无许可证问题。
- `cargo doc --workspace --no-deps`：13 crate 全部生成、零 rustdoc 警告。
- 10 项门禁脚本全绿（编译/测试/警告/格式/文档/审计/依赖/无空目录/无 src 平铺/临时目录）。

### 约束守住

- 13 角色不动 / 4 层流水线不动 / 36 格位不增不减 / Service Definition 签名不变。
- 测试不退化：v1 之前 689 项测试全保留，新增 12 项偏好表测试。
- 设计稿先入稿再落码（AGENTS §8）：工具乐高化设计纲要先入多智能体 §1.5.6 / 智能体 §五 / 上下文 §十一，再写代码。
- 文档收割门（AGENTS §13）：每次 .md 设计稿迭代前 commit，迭代后 git diff 对比，确认无「[object Object]」/内容断崖式缩水/章节级损坏。

### 已知局限（v1+ 处理）

- 任务调度入口软化（并行探查 + 错误恢复）：超出 v1 范围，v1+ 再评估。
- 工具执行异步化（tokio 切换）：当前同步执行，v1+ 评估（涉及核心异步运行时改动）。
- 监控 / 看板 / UI：v1 阶段按界主拍板不投入，核心（脑/身/心）优先于门面；UI 权重最低，待达到 DeepSeek Harness 级「使用体验质量」再考虑。