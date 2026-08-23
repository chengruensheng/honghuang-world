# 端点-e2e-阁：监控界面 HTTP 端点 e2e 测试

镜像：§11.6.2 第 4 条（测试在 `证道/单元测试-府` 镜像为新殿）+ §11.f.4。

## 运行

**单独跑（推荐 · 7 个 e2e 全过）：**

```bash
cargo test -p zhengdao_fu --lib 监控界面_验证_殿::端点_e2e_阁
```

**全 workspace 跑（环境变量污染，仅 4 个不依赖 工作区::定位 通过）：**

```bash
cargo test --workspace --lib
```

## 环境变量依赖

镜像殿 e2e 测试调 `shihai_fu::工作区::定位()`（jiankong_fu 内 handler 间接调用）。
`工作区::定位` 用 `OnceLock` 缓存：一旦初始化即锁定，后续调用不再读环境变量。

zhengdao_fu 其他测试（增量检测 / 源缺失 / 改写文件 / 等）会通过 `WORLD_WORKSPACE_ROOT`
环境变量把工作区指到临时目录，污染了 OnceLock 的首次初始化。

镜像殿 e2e 测试在 `构造应用()` 时强制 `unset + 重新设` 该环境变量，但因 OnceLock
已锁，无法重置。

## 已知约束

| 跑法 | 通过 | 失败 |
|:--|:--|:--|
| `cargo test -p zhengdao_fu --lib 监控界面_验证_殿::端点_e2e_阁` | 7/7 | 0 |
| `cargo test -p zhengdao_fu --lib` | 184/190 | 3 依赖 工作区::定位 的 e2e（因其他测试污染 OnceLock）|
| `cargo test --workspace --lib` | 同上 | 同上 |

3 个失败的 e2e（self_check_targets_列表完整 / self_check_命令操作_府_健康分100 /
cards_返回九卡片摘要 / rooms_返回九府配置）单独跑时全过，全跑时因 OnceLock 污染失败。

## 后续修复

可在 shihai_fu 加 `pub fn 重置工作区()`（unsafe 重置 OnceLock）供测试使用；
或让 jiankong_fu 加 `pub fn 建路由_根(根: &Path)`（接受工作区根，跳过 工作区::定位）。

依据：融合蓝图 §11.6.2 第 4 条 + §11.f.4。
