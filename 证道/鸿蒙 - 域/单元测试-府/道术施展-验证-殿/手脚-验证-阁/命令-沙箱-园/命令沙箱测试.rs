//! 命令-沙箱-园 · 命令沙箱测试：验证隔离视图、越界检测回滚、清理、超时强杀。
//! 仅经 daoshu_fu 根 pub 符号（沙箱视图/运行/清理/沙箱结果）与可观察目录布局测试。

#[cfg(test)]
mod 测试 {
    use daoshu_fu::沙箱视图;
    use std::fs;

    /// 临时工作区：造若干源码文件，测试结束清理。
    fn 临时工作区(名: &str) -> std::path::PathBuf {
        let 根 = std::env::temp_dir().join(format!("证道_命令沙箱_{名}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&根);
        fs::create_dir_all(&根.join("甲")).unwrap();
        fs::write(根.join("Cargo.toml"), "[package]\nname = \"沙箱\"\n").unwrap();
        fs::write(根.join("甲").join("丙.rs"), "pub fn 丙() {}\n").unwrap();
        根
    }

    /// 视图根（沙箱目录布局）：.上下文/命令沙箱/{任务id}/视图。
    fn 视图路径(根: &std::path::Path, 任务id: &str) -> std::path::PathBuf {
        根.join(".上下文").join("命令沙箱").join(任务id).join("视图")
    }

    #[test]
    fn 沙箱内运行命令返回结果() {
        let 根 = 临时工作区("运行");
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        let 回执 = 沙箱.运行("cargo", &["--version"], None, None).unwrap();
        assert_eq!(回执.结果.退出码, Some(0));
        assert!(回执.结果.标准输出.contains("cargo"));
        assert_eq!(回执.越界数, 0);
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 越界修改穿透真实后双侧恢复() {
        let 根 = 临时工作区("穿透");
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        // 视图源码文件与真实同 inode（硬链接）：命令改写视图文件即穿透真实。
        let 视图丙 = 视图路径(&根, "任务T").join("甲").join("丙.rs");
        let 命令 = format!("Set-Content -LiteralPath '{}' -Value '改'", 视图丙.to_string_lossy());
        let 回执 = 沙箱.运行("powershell.exe", &["-NoProfile", "-Command", &命令], None, None).unwrap();
        assert!(回执.越界数 >= 1, "穿透改写应判越界：{}", 回执.越界详情);
        assert_eq!(
            fs::read_to_string(根.join("甲").join("丙.rs")).unwrap(),
            "pub fn 丙() {}\n",
            "真实文件应恢复写前内容"
        );
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 越界新增与删除均回滚() {
        let 根 = 临时工作区("增删");
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        let 视图 = 视图路径(&根, "任务T");
        let 命令 = format!(
            "New-Item -ItemType File -Path '{}'; Remove-Item -LiteralPath '{}'",
            视图.join("越界.rs").to_string_lossy(),
            视图.join("甲").join("丙.rs").to_string_lossy()
        );
        let 回执 = 沙箱.运行("powershell.exe", &["-NoProfile", "-Command", &命令], None, None).unwrap();
        assert!(回执.越界数 >= 2, "新增与删除应各计一处：{}", 回执.越界详情);
        assert!(根.join("甲").join("丙.rs").exists(), "真实文件不受视图删除影响");
        assert!(!根.join("越界.rs").exists(), "越界新增不应落到真实盘面");
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 真实区绝对路径写入被拦截() {
        let 根 = 临时工作区("绝对");
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        let 越界文件 = 根.join("真实越界.txt");
        let 命令 = format!("Set-Content -LiteralPath '{}' -Value 'x'", 越界文件.to_string_lossy());
        let 回执 = 沙箱.运行("powershell.exe", &["-NoProfile", "-Command", &命令], None, None).unwrap();
        assert!(回执.越界数 >= 1, "真实区写入应判越界：{}", 回执.越界详情);
        assert!(!越界文件.exists(), "真实区越界新增应被删除");
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 清理删除整箱() {
        let 根 = 临时工作区("清理");
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        沙箱.运行("cargo", &["--version"], None, None).unwrap();
        沙箱.清理().unwrap();
        assert!(
            !根.join(".上下文").join("命令沙箱").join("任务T").exists(),
            "整箱应被删除"
        );
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 物化增量复用_不重写视图() {
        let 根 = 临时工作区("增量");
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        沙箱.运行("cargo", &["--version"], None, None).unwrap();
        let 视图文件 = 视图路径(&根, "任务T").join("甲").join("丙.rs");
        let 指纹 = |路径: &std::path::Path| {
            let 元 = fs::metadata(路径).unwrap();
            (
                元.len(),
                元.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
            )
        };
        let 前 = 指纹(&视图文件);
        std::thread::sleep(std::time::Duration::from_millis(20));
        沙箱.运行("cargo", &["--version"], None, None).unwrap();
        let 后 = 指纹(&视图文件);
        assert_eq!(前, 后, "增量物化不应重写视图文件");
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 构建物不进视图() {
        let 根 = 临时工作区("排除");
        fs::create_dir_all(根.join("道果树")).unwrap();
        fs::write(根.join("道果树").join("构建.exe"), "产物").unwrap();
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        沙箱.运行("cargo", &["--version"], None, None).unwrap();
        assert!(
            !视图路径(&根, "任务T").join("道果树").exists(),
            "构建物不应镜像进视图"
        );
        let _ = fs::remove_dir_all(&根);
    }

    #[test]
    fn 沙箱超时_长命令被强杀回错() {
        // 1 秒超时，命令真等 30 秒：必触发沙箱内超时强杀，越界数仍应为 0（powershell 未改源码）。
        let 根 = 临时工作区("沙箱超时");
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        let 错 = 沙箱
            .运行(
                "powershell.exe",
                &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
                None,
                Some(1_000),
            )
            .unwrap_err();
        assert!(错.contains("超时"), "应返回超时错误：{错}");
        assert!(错.contains("强杀"), "应说明子进程已被强杀：{错}");
        let _ = fs::remove_dir_all(&根);
    }

    /// 构建自证走沙箱后，真实区不被 cargo 副作用污染（2026-08-17 直播实锤修复：
    /// 原在真实区跑 cargo build，模型改 Cargo.toml 后 cargo 连锁更新真实区 Cargo.lock，
    /// 不经沙箱越界检测、不在回滚垫，打回撤销不恢复——根级文件被污染）。
    /// 机制验证：cargo 在视图内生成 Cargo.lock → 沙箱判越界删除，真实区保持无锁文件。
    #[test]
    fn 沙箱内cargo构建不污染真实区锁文件() {
        let 根 = std::env::temp_dir().join(format!("证道_命令沙箱_锁文件_{}", std::process::id()));
        let _ = fs::remove_dir_all(&根);
        fs::create_dir_all(根.join("src")).unwrap();
        fs::write(
            根.join("Cargo.toml"),
            "[package]\nname = \"锁文件沙箱\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(根.join("src").join("lib.rs"), "pub fn 甲() {}\n").unwrap();
        let 沙箱 = 沙箱视图::打开(&根, "任务T");
        let 回执 = 沙箱.运行("cargo", &["build"], None, None).unwrap();
        assert!(
            !根.join("Cargo.lock").exists(),
            "真实区不应出现 Cargo.lock（视图内生成的应被越界清理）：{}",
            回执.越界详情
        );
        let _ = fs::remove_dir_all(&根);
    }
}