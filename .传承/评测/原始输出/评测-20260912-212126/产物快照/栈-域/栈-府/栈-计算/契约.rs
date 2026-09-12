/// 后进先出栈契约
pub trait 后进先出栈 {
    /// 将整数元素压入栈顶，后进先出
    fn 入栈(&mut self, 元素: i64);

    /// 弹出并返回栈顶元素，空栈返回 None
    fn 出栈(&mut self) -> Option<i64>;

    /// 返回栈顶元素但不移除，空栈返回 None
    fn 查看栈顶(&self) -> Option<i64>;
}
