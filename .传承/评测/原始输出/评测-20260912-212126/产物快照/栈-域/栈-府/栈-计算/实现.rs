use crate::契约::后进先出栈;

/// 基于 Vec 实现的后进先出栈
pub struct 栈 {
    元素: Vec<i64>,
}

impl 栈 {
    /// 创建空栈
    pub fn 新建() -> Self {
        栈 { 元素: Vec::new() }
    }
}

impl 后进先出栈 for 栈 {
    fn 入栈(&mut self, 元素: i64) {
        self.元素.push(元素);
    }

    fn 出栈(&mut self) -> Option<i64> {
        self.元素.pop()
    }

    fn 查看栈顶(&self) -> Option<i64> {
        self.元素.last().copied()
    }
}
