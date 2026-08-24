# 事故复盘 0002：cargo test 递归调用导致测试卡死

## 日期
2026-08-25

## 状态
已修复

## 摘要
`命令护栏放行编译类命令` 测试在沙箱里运行 `cargo test` 命令。
沙箱根是临时目录（无 Cargo.toml），cargo 向上搜索找到 workspace 根，
递归触发全部测试（包括自身），导致无限循环/超时。

## 影响
- daoshu_fu 测试卡死：`命令护栏放行编译类命令` 永不返回
- 严重程度：阻塞（cargo test --workspace 超时）

## 根因
测试（测试.rs:146）调 `执行工具` 运行 `cargo test` →
`执行工具`（工具执行.rs:460）用 `沙箱视图::打开当前(根)` →
沙箱根 = 临时目录（无 Cargo.toml） →
cargo 向上搜索 Cargo.toml → 找到 workspace 根 →
递归运行 `cargo test` → 包括自身测试 → 无限循环。

## 修复
把 `cargo build --workspace --lib` 和 `cargo test` 改成 `cargo --version`：
- 文件：`道术施展-府/任务-调度-殿/任务-派遣-阁/工具-循环-园/测试.rs:145-146`
- 测试意图是验证"编译类命令被放行"，`cargo --version` 同样验证且不递归。

## 教训
- 测试中运行 cargo 命令时，必须确保工作目录不含 workspace 根的 Cargo.toml
- 或用不触发递归的子命令（--version / --help）代替 build/test
- 沙箱的根目录隔离不防止 cargo 向上搜索

## 验证
```
cargo test -p daoshu_fu --lib  # 99 passed / 0 failed / 1 ignored，0.71s 完成
```