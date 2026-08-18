//! 沙箱 - 护栏 - 园：运行命令护栏，禁止终止进程、运行/构建项目自身二进制，并强制超时上限。
//!
//! 背景：真实任务实测模型跑 `cargo run --bin 号令` 撞主进程文件锁（os error 32，退出码 101），
//! 随后用 `Stop-Process` 杀掉主进程自身，导致任务中断、运行一轮未收尾。
//! 护栏在工具循环的「运行命令」分支与沙箱执行前统一生效，不依赖模型自觉。
//!
//! 超时护栏：原版「运行命令」无超时上限，命令可无限卡死挂起整轮执行。
//! 模型传入的「超时毫秒」必须在 (0, 最大超时毫秒] 区间内，防指定 0 / 负数 / 极大值绕过护栏。

/// 最大超时上限（毫秒）：10 分钟。模型可调小，不可调大；防指定极大值挂死任务。
pub const 最大超时毫秒: u64 = 600_000;

/// 校验命令护栏：命令进沙箱前的最后拦截。
/// 含超时校验：超时毫秒必须 > 0 且 <= 最大超时毫秒。
pub fn 校验命令护栏(
    命令: &str,
    参数们: &[&str],
    超时毫秒: Option<u64>,
) -> Result<(), String> {
    let 整行 = std::iter::once(命令)
        .chain(参数们.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let 小写 = 整行.to_lowercase();

    // 超时护栏：必须 > 0 且 <= 最大上限。None 由调用方兜底为 默认超时毫秒。
    if let Some(超时) = 超时毫秒 {
        if 超时 == 0 {
            return Err(format!(
                "命令被护栏拦截：超时毫秒为 0 必须大于 0（最大上限 {最大超时毫秒}）"
            ));
        }
        if 超时 > 最大超时毫秒 {
            return Err(format!(
                "命令被护栏拦截：超时 {超时} 毫秒超过最大上限 {最大超时毫秒} 毫秒"
            ));
        }
    }

    // 进程终止/管理类操作一律拦截（含 cmd/powershell 内嵌参数）。
    for 词 in [
        "taskkill",
        "stop-process",
        "stop-service",
        "pkill",
        "killall",
    ] {
        if 小写.contains(词) {
            return Err(format!(
                "命令被护栏拦截：含进程终止操作「{词}」，不得终止任何进程"
            ));
        }
    }
    if 命令.eq_ignore_ascii_case("kill") {
        return Err("命令被护栏拦截：kill 命令不得执行".to_string());
    }

    // cargo 系列：仅放行编译/检查/测试（不产出自身 exe 也不运行）；run 与 build 自身 bin 一律拦截。
    if 命令.eq_ignore_ascii_case("cargo") {
        let 首参 = 参数们.first().map(|s| s.to_lowercase()).unwrap_or_default();
        if 首参 == "run" {
            return Err("命令被护栏拦截：cargo run 会运行项目自身二进制并撞文件锁，请改用 cargo build --workspace --lib 验证编译".to_string());
        }
        if 首参 == "build" {
            let 撞自身 = 参数们.iter().any(|p| {
                let 低 = p.to_lowercase();
                低.contains("号令") || 低.contains("--bin")
            });
            if 撞自身 {
                return Err("命令被护栏拦截：cargo build 指定自身 bin 会重链接被占用的号令.exe 撞文件锁，请改用 cargo build --workspace --lib".to_string());
            }
        }
        return Ok(());
    }

    // 其余命令一律不得引用项目自身二进制名：运行号令.exe、探查其进程都会诱导模型误操作。
    if 小写.contains("号令") {
        return Err(
            "命令被护栏拦截：不得运行或探查项目自身二进制（号令），命令功能验证由界主执行"
                .to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod 测试 {
    use super::校验命令护栏;

    /// 边界值：超时毫秒 = 0 必须被拒（> 0 才能进入沙箱）。
    #[test]
    fn 超时毫秒为0应被拒绝() {
        let 错 = 校验命令护栏("cargo", &["build"], Some(0)).expect_err("超时毫秒为 0 应被护栏拒绝");
        assert!(
            错.contains("超时毫秒为 0"),
            "错误信息应明示「超时毫秒为 0」：{错}"
        );
    }
}
