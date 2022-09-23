#[derive(Clone, Copy)]
pub struct ListNode<T> {
    pub prev: Option<*mut T>,
    pub next: Option<*mut T>,
}