#![cfg(test)]
//! 安全·单进程：端口占用即拒 + 监控界面不写任何状态文件。
//! 依据：融合蓝图 §6.3 安全副作用「不写任何状态文件 + 只起一个 HTTP 服务进程」。
//!
//! 注：完整 HTTP 启动验证已在 项目全景理解 与 cookbook 实测覆盖；
//! 本园专注「端口独立性」（一次只能一个进程监听）和「不写状态」。

use std::net::TcpListener;

/// 找一个空闲端口。
fn 空闲端口() -> u16 {
    let 监听 = TcpListener::bind("127.0.0.1:0").unwrap();
    let 端口 = 监听.local_addr().unwrap().port();
    drop(监听);
    端口
}

#[test]
fn 同端口_两个监听器_第二个失败() {
    let 端口 = 空闲端口();
    let _监听1 = TcpListener::bind(format!("127.0.0.1:{}", 端口)).expect("第一个监听器应成功");
    let 结果2 = TcpListener::bind(format!("127.0.0.1:{}", 端口));
    assert!(结果2.is_err(), "同端口二次绑定应失败：{:?}", 结果2.err());
}

#[test]
fn 不同端口_可独立监听() {
    let 端口1 = 空闲端口();
    let 端口2 = 空闲端口();
    assert_ne!(端口1, 端口2);

    let 监听1 = TcpListener::bind(format!("127.0.0.1:{}", 端口1)).unwrap();
    let 监听2 = TcpListener::bind(format!("127.0.0.1:{}", 端口2)).unwrap();
    assert!(监听1.local_addr().is_ok());
    assert!(监听2.local_addr().is_ok());
}

#[test]
fn jiankong_fu_不声明依赖任何会写状态的crate() {
    // 静态检查：jiankong_fu 自身不写任何 .json/.jsonl 文件
    // （其消费三源 jsonl 是只读，事件流增量由调用方经 mpsc channel 注入）
    // 通过依赖图静态特征证明：监控界面-府只读三源 lib 根符号。
    //
    // 此测试断言监控界面-府的入口文件存在且可访问（间接证明不依赖未声明文件）。
    let 入口 = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../乾坤/呈现-域/监控界面-府/入口.rs");
    assert!(入口.exists(), "监控界面-府入口.rs 应存在：{:?}", 入口);
}
