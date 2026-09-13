// 外部判据夹具：由评测方提供，不属于被测产物，直接断言行为。
// 注入位置：产物 crate 的 tests/ 目录；通过条件：本夹具全部测试通过。
// {{crate}} 占位符在注入前被替换为产物 crate 名（连字符转下划线）。

#[test]
fn 外部判据_加减乘除优先级() {
    assert_eq!({{crate}}::求值("1+2*3"), Some(7));
}

#[test]
fn 外部判据_括号改变优先级() {
    assert_eq!({{crate}}::求值("(1+2)*3"), Some(9));
}

#[test]
fn 外部判据_一元负号() {
    assert_eq!({{crate}}::求值("-4+10"), Some(6));
}

#[test]
fn 外部判据_空格容错() {
    assert_eq!({{crate}}::求值(" 1 + 2 * ( 3 - 4 ) "), Some(-1));
}

#[test]
fn 外部判据_除法与截断() {
    assert_eq!({{crate}}::求值("7/2"), Some(3));
}

#[test]
fn 外部判据_除零返回空() {
    assert_eq!({{crate}}::求值("10/0"), None);
}

#[test]
fn 外部判据_非法表达式返回空() {
    assert_eq!({{crate}}::求值("1+"), None);
    assert_eq!({{crate}}::求值(""), None);
    assert_eq!({{crate}}::求值("(1+2"), None);
}
