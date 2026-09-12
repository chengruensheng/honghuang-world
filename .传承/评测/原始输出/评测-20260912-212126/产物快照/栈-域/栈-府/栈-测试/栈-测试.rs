use zhan_fu::契约::后进先出栈;
use zhan_fu::实现::栈;

#[test]
fn 集成测试空栈返回空() {
    let mut 栈实例 = 栈::新建();
    assert_eq!(栈实例.出栈(), None);
    assert_eq!(栈实例.查看栈顶(), None);
}

#[test]
fn 集成测试后进先出() {
    let mut 栈实例 = 栈::新建();
    栈实例.入栈(100);
    栈实例.入栈(200);
    栈实例.入栈(300);
    assert_eq!(栈实例.出栈(), Some(300));
    assert_eq!(栈实例.出栈(), Some(200));
    assert_eq!(栈实例.出栈(), Some(100));
}

#[test]
fn 集成测试负数与交替操作() {
    let mut 栈实例 = 栈::新建();
    栈实例.入栈(-1);
    栈实例.入栈(0);
    assert_eq!(栈实例.出栈(), Some(0));
    栈实例.入栈(1);
    assert_eq!(栈实例.查看栈顶(), Some(1));
    assert_eq!(栈实例.出栈(), Some(1));
    assert_eq!(栈实例.出栈(), Some(-1));
}
