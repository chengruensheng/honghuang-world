#[cfg(test)]
mod 测试 {
    use crate::契约::后进先出栈;
    use crate::实现::栈;

    #[test]
    fn 空栈出栈返回空() {
        let mut 栈实例 = 栈::新建();
        assert_eq!(栈实例.出栈(), None);
    }

    #[test]
    fn 空栈查看栈顶返回空() {
        let 栈实例 = 栈::新建();
        assert_eq!(栈实例.查看栈顶(), None);
    }

    #[test]
    fn 后进先出顺序正确() {
        let mut 栈实例 = 栈::新建();
        栈实例.入栈(1);
        栈实例.入栈(2);
        栈实例.入栈(3);
        assert_eq!(栈实例.出栈(), Some(3));
        assert_eq!(栈实例.出栈(), Some(2));
        assert_eq!(栈实例.出栈(), Some(1));
        assert_eq!(栈实例.出栈(), None);
    }

    #[test]
    fn 查看栈顶不移除元素() {
        let mut 栈实例 = 栈::新建();
        栈实例.入栈(10);
        栈实例.入栈(20);
        assert_eq!(栈实例.查看栈顶(), Some(20));
        assert_eq!(栈实例.查看栈顶(), Some(20));
        assert_eq!(栈实例.出栈(), Some(20));
        assert_eq!(栈实例.查看栈顶(), Some(10));
    }

    #[test]
    fn 入栈出栈交替仍保持后进先出() {
        let mut 栈实例 = 栈::新建();
        栈实例.入栈(5);
        栈实例.入栈(6);
        assert_eq!(栈实例.出栈(), Some(6));
        栈实例.入栈(7);
        assert_eq!(栈实例.出栈(), Some(7));
        assert_eq!(栈实例.出栈(), Some(5));
        assert_eq!(栈实例.出栈(), None);
    }

    #[test]
    fn 支持负数与零() {
        let mut 栈实例 = 栈::新建();
        栈实例.入栈(-1);
        栈实例.入栈(0);
        栈实例.入栈(i64::MIN);
        栈实例.入栈(i64::MAX);
        assert_eq!(栈实例.出栈(), Some(i64::MAX));
        assert_eq!(栈实例.出栈(), Some(i64::MIN));
        assert_eq!(栈实例.出栈(), Some(0));
        assert_eq!(栈实例.出栈(), Some(-1));
    }
}
