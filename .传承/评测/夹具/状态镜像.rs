// 外部判据夹具：由评测方提供，不属于被测产物，直接断言行为。
// 注入位置：产物 crate 的 tests/ 目录；通过条件：本夹具全部测试通过。
// {{crate}} 占位符在注入前被替换为产物 crate 名（连字符转下划线）。
//
// 本夹具专门检验「带依赖」能力：夹具自身使用 serde_json（完全限定路径），
// 并要求产物类型 derive serde::Serialize / serde::Deserialize——
// 因此产物 Cargo.toml 若缺 serde_json 或缺 serde(derive) 依赖，本夹具将**编译失败**，
// 从而把「声明了依赖但未正确使用」与「根本没声明依赖」都判为不通过。

#[test]
fn 外部判据_状态镜像_序列化为变体名() {
    let v = {{crate}}::状态镜像::清理完成;
    let s = serde_json::to_string(&v).expect("需 serde::Serialize 派生 + serde_json 依赖");
    assert_eq!(s, "\"清理完成\"");
}

#[test]
fn 外部判据_状态镜像_反序列化() {
    let s = "\"待圣人设计\"";
    let d: {{crate}}::状态镜像 = serde_json::from_str(s).expect("需 serde::Deserialize 派生 + serde_json 依赖");
    assert_eq!(d, {{crate}}::状态镜像::待圣人设计);
}

#[test]
fn 外部判据_状态镜像_五变体往返一致() {
    let 变体们 = [
        {{crate}}::状态镜像::待圣人设计,
        {{crate}}::状态镜像::待大罗金仙实现,
        {{crate}}::状态镜像::待准圣验收,
        {{crate}}::状态镜像::待道祖终审,
        {{crate}}::状态镜像::清理完成,
    ];
    for v in 变体们 {
        let s = serde_json::to_string(&v).unwrap();
        let d: {{crate}}::状态镜像 = serde_json::from_str(&s).unwrap();
        assert_eq!(d, v);
    }
}
