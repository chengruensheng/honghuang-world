// 外部判据夹具：由评测方提供，不属于被测产物，直接断言行为。
// 注入位置：产物 crate 的 tests/ 目录；通过条件：本夹具全部测试通过。
// {{crate}} 占位符在注入前被替换为产物 crate 名（连字符转下划线）。
//
// 本夹具检验「读真实数据 + 健壮解析」：JSONL 逐行解析、按 status 计数、
// 缺字段/坏行/非 UTF-8/缺文件等边界的 Err 语义。
// 临时文件落在 std::env::temp_dir()，用进程 id 隔离避免并发冲突。

fn 临时路径(名: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("外部判据-{}-{}", std::process::id(), 名));
    p
}

fn 写临时(名: &str, 内容: &[u8]) -> String {
    let p = 临时路径(名);
    std::fs::write(&p, 内容).expect("写临时文件");
    p.to_string_lossy().to_string()
}

fn 清理(名: &str) {
    let _ = std::fs::remove_file(临时路径(名));
}

#[test]
fn 外部判据_正常多行按状态计数() {
    let 路径 = 写临时("正常.jsonl", concat!(
        r#"{"id":1,"title":"甲","status":"清理完成"}"#, "\n",
        r#"{"id":2,"title":"乙","status":"已取消"}"#, "\n",
        r#"{"id":3,"title":"丙","status":"清理完成"}"#, "\n",
    ).as_bytes());
    let 表 = {{crate}}::统计状态(&路径).expect("正常文件应成功");
    assert_eq!(表.get("清理完成"), Some(&2));
    assert_eq!(表.get("已取消"), Some(&1));
    assert_eq!(表.len(), 2);
    assert_eq!({{crate}}::总任务数(&路径).expect("正常文件应成功"), 3);
    清理("正常.jsonl");
}

#[test]
fn 外部判据_空文件得空表与零() {
    let 路径 = 写临时("空.jsonl", b"");
    let 表 = {{crate}}::统计状态(&路径).expect("空文件应成功");
    assert_eq!(表.len(), 0);
    assert_eq!({{crate}}::总任务数(&路径).expect("空文件应成功"), 0);
    清理("空.jsonl");
}

#[test]
fn 外部判据_缺status行计入未知() {
    let 路径 = 写临时("缺字段.jsonl", concat!(
        r#"{"id":1,"title":"甲","status":"清理完成"}"#, "\n",
        r#"{"id":2,"title":"乙"}"#, "\n",
    ).as_bytes());
    let 表 = {{crate}}::统计状态(&路径).expect("缺字段行不应整体失败");
    assert_eq!(表.get("清理完成"), Some(&1));
    assert_eq!(表.get("未知"), Some(&1));
    assert_eq!({{crate}}::总任务数(&路径).expect("缺字段行仍算解析成功"), 2);
    清理("缺字段.jsonl");
}

#[test]
fn 外部判据_坏行返回错误() {
    let 路径 = 写临时("坏行.jsonl", concat!(
        r#"{"id":1,"status":"清理完成"}"#, "\n",
        "这不是JSON\n",
    ).as_bytes());
    assert!({{crate}}::统计状态(&路径).is_err(), "坏行应返回 Err");
    assert!({{crate}}::总任务数(&路径).is_err(), "坏行应返回 Err");
    清理("坏行.jsonl");
}

#[test]
fn 外部判据_非UTF8返回错误() {
    let 路径 = 写临时("乱码.jsonl", &[0xFF, 0xFE, 0x7B, 0x7D]);
    assert!({{crate}}::统计状态(&路径).is_err(), "非 UTF-8 应返回 Err");
    清理("乱码.jsonl");
}

#[test]
fn 外部判据_文件不存在返回错误() {
    let p = 临时路径("不存在.jsonl");
    let _ = std::fs::remove_file(&p);
    let 路径 = p.to_string_lossy().to_string();
    assert!({{crate}}::统计状态(&路径).is_err(), "文件不存在应返回 Err");
    assert!({{crate}}::总任务数(&路径).is_err(), "文件不存在应返回 Err");
}
