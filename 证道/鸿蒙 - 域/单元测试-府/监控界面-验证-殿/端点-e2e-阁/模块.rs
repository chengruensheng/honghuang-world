//! 端点-e2e-阁：监控界面端到端 e2e 测试（HTTP axum TestServer）。
//!
//! 依据：融合蓝图 §11.6.2 第 4 条（测试在 `证道/单元测试-府` 镜像为新殿）+ §11.f.4。
//! 之前 §13.d / §11.f 的 13 个 e2e 测试内联在 jiankong_fu 内 mod self_check_e2e，
//! 违背镜像约束。本阁把 e2e 测试迁移到镜像殿。
//!
//! 测试运行：`cargo test -p zhengdao_fu --lib 监控界面_验证_殿::端点_e2e_阁`

#![allow(non_snake_case)]

#[path = "e2e 测试.rs"]
pub mod e2e_测试;
