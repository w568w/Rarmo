use core::ptr;
use crate::common::list::ListNode;

#[derive(Clone, Copy)]
pub enum RbTreeColor {
    Red,
    Black,
}

pub struct RbTree<T: ListNode<RbTreeLink> + 'static> {
    root: *mut RbTreeLink,
    size: usize,
    // a > b: cmp(a, b) == false,
    // a < b: cmp(a, b) == true
    cmp: fn(&mut T, &mut T) -> bool,
}

pub struct RbTreeLink {
    pub parent: *mut RbTreeLink,
    pub left: *mut RbTreeLink,
    pub right: *mut RbTreeLink,
    pub color: RbTreeColor,
}

impl RbTreeLink {
    pub const fn new() -> Self {
        Self {
            parent: ptr::null_mut(),
            left: ptr::null_mut(),
            right: ptr::null_mut(),
            color: RbTreeColor::Red,
        }
    }

    pub fn left_child(&mut self) -> &mut RbTreeLink {
        unsafe { &mut *self.left }
    }
    pub fn right_child(&mut self) -> &mut RbTreeLink {
        unsafe { &mut *self.right }
    }

    fn set_left_child(&mut self, child: *mut RbTreeLink) {
        self.left = child;
        if !child.is_null() {
            unsafe { (*child).parent = self };
        }
    }

    fn set_right_child(&mut self, child: *mut RbTreeLink) {
        self.right = child;
        if !child.is_null() {
            unsafe { (*child).parent = self };
        }
    }

    fn is_left_child(&self) -> bool {
        if self.parent.is_null() {
            false
        } else {
            let parent_left = unsafe { (*self.parent).left } as *const RbTreeLink;
            parent_left == self
        }
    }

    unsafe fn left_rotate<T: ListNode<RbTreeLink>>(&mut self, tree: &mut RbTree<T>) {
        // Set the right child's left child as the new right child
        let right_child = self.right;
        self.set_right_child((*right_child).left);

        // Set the parent's new child to the right child
        (*right_child).parent = self.parent;
        if self.parent.is_null() {
            // self is root, so set the right child as the new root
            tree.root = right_child;
        } else if self.is_left_child() {
            (*self.parent).set_left_child(right_child);
        } else {
            (*self.parent).set_right_child(right_child);
        }

        // Set self's parent to the right child
        (*right_child).set_left_child(self);
    }

    unsafe fn right_rotate<T: ListNode<RbTreeLink>>(&mut self, tree: &mut RbTree<T>) {
        // Set the left child's right child as the new left child
        let left_child = self.left;
        self.set_left_child((*left_child).right);

        // Set the parent's new child to the left child
        (*left_child).parent = self.parent;
        if self.parent.is_null() {
            // self is root, so set the left child as the new root
            tree.root = left_child;
        } else if self.is_left_child() {
            (*self.parent).set_left_child(left_child);
        } else {
            (*self.parent).set_right_child(left_child);
        }

        // Set self's parent to the left child
        (*left_child).set_right_child(self);
    }
}

impl<T: ListNode<RbTreeLink> + 'static> RbTree<T> {
    pub fn new(cmp: fn(&mut T, &mut T) -> bool) -> Self {
        Self {
            root: ptr::null_mut(),
            size: 0,
            cmp,
        }
    }
    fn inner_cmp(&self, a: *mut RbTreeLink, b: *mut RbTreeLink) -> bool {
        (self.cmp)(T::container(a), T::container(b))
    }
    fn is_black(link: *mut RbTreeLink) -> bool {
        link.is_null() || unsafe { matches!((*link).color,RbTreeColor::Black) }
    }
    unsafe fn fix_insert(&mut self, mut node_to_fix: *mut RbTreeLink) {
        (*node_to_fix).color = RbTreeColor::Red;

        while !node_to_fix.is_null() && self.root != node_to_fix && !Self::is_black((*node_to_fix).parent) {
            let mut parent = (*node_to_fix).parent;
            let grandparent = (*parent).parent;
            if parent == (*grandparent).left {
                // If parent is the left child
                let uncle = (*grandparent).right;
                if !Self::is_black(uncle) {
                    // If uncle is red, recolor parent and uncle to black, grandparent to red,
                    // and fix at grandparent
                    (*parent).color = RbTreeColor::Black;
                    (*uncle).color = RbTreeColor::Black;
                    (*grandparent).color = RbTreeColor::Red;
                    node_to_fix = grandparent;
                } else {
                    // If uncle is black and node is the left child, we need to recolor parent to black, grandparent to red,
                    // and right rotate at grandparent
                    if node_to_fix == (*parent).right {
                        // Special case: if node is the right child, left rotate at parent first!
                        // And then node will be set to its parent. `parent` will be set to original `node_to_fix`.
                        node_to_fix = parent;
                        (*node_to_fix).left_rotate(self);
                        parent = (*node_to_fix).parent;
                    }
                    (*parent).color = RbTreeColor::Black;
                    (*grandparent).color = RbTreeColor::Red;
                    (*grandparent).right_rotate(self);
                }
            } else {
                let uncle = (*grandparent).left;
                if !Self::is_black(uncle) {
                    (*parent).color = RbTreeColor::Black;
                    (*uncle).color = RbTreeColor::Black;
                    (*grandparent).color = RbTreeColor::Red;
                    node_to_fix = grandparent;
                } else {
                    if node_to_fix == (*parent).left {
                        node_to_fix = parent;
                        (*node_to_fix).right_rotate(self);
                        parent = (*node_to_fix).parent;
                    }
                    (*parent).color = RbTreeColor::Black;
                    (*grandparent).color = RbTreeColor::Red;
                    (*grandparent).left_rotate(self);
                }
            }
        }

        // Rule: root is always black!
        (*self.root).color = RbTreeColor::Black;
    }
    unsafe fn just_insert(&mut self, link: *mut RbTreeLink) -> bool {
        if self.root.is_null() {
            self.root = link;
            (*link).color = RbTreeColor::Black;
            false
        } else {
            let mut parent = self.root;
            loop {
                if self.inner_cmp(link, parent) {
                    if (*parent).left.is_null() {
                        (*parent).set_left_child(link);
                        return matches!((*parent).color, RbTreeColor::Red);
                    } else {
                        parent = (*parent).left;
                    }
                } else {
                    if (*parent).right.is_null() {
                        (*parent).set_right_child(link);
                        return matches!((*parent).color, RbTreeColor::Red);
                    } else {
                        parent = (*parent).right;
                    }
                }
            }
        }
    }
    pub fn insert(&mut self, node: *mut T) {
        let link = unsafe { (*node).link_ptr() };
        unsafe {
            if self.just_insert(link) {
                self.fix_insert(link);
            }
        }
        self.size += 1;
    }
    unsafe fn just_delete(&mut self, link: *mut RbTreeLink) -> (bool, *mut RbTreeLink) {
        let parent = (*link).parent;
        // Find a candidate to replace the deleted node `link`.
        let mut child = if (*link).left.is_null() {
            // If `link` has no left child, we can just replace it with its right child (can be null).
            (*link).right
        } else if (*link).right.is_null() {
            // If `link` has no right child, we can just replace it with its left child.
            (*link).left
        } else {
            // If `link` has both left and right child, we need to find the successor of `link` to replace it.
            let mut successor = (*link).right;
            while !(*successor).left.is_null() {
                successor = (*successor).left;
            }

            let successor_parent = (*successor).parent;
            let successor_right = (*successor).right;
            if successor_parent != link {
                // If the successor is not the direct right child of `link`, we need to replace the successor with its right child.
                (*successor_parent).set_left_child(successor_right);
                // And link should give its right subtree to the successor.
                (*successor).set_right_child((*link).right);
            }
            // `link` should also give its left subtree to the successor.
            (*successor).set_left_child((*link).left);
            successor
        };
        // Tell the parent of `link` that its new child is `child` now.
        if parent.is_null() {
            self.root = child;
        } else if link == (*parent).left {
            (*parent).set_left_child(child);
        } else {
            (*parent).set_right_child(child);
        }
        if child.is_null() {
            // If `child` is null (i.e. `link` has no child), we can just delete `link` (we have already done it) and return.
            (false, child)
        } else {
            // We need to fix the tree if `link` is black.
            (*child).parent = parent;
            (*child).color = (*link).color;
            (Self::is_black(child), child)
        }
    }
    unsafe fn fix_delete(&mut self, mut node_to_fix: *mut RbTreeLink) {
        while !node_to_fix.is_null() && self.root != node_to_fix && Self::is_black(node_to_fix) {
            let mut parent = (*node_to_fix).parent;
            if node_to_fix == (*parent).left {
                // If node is the left child
                let mut sibling = (*parent).right;
                if !Self::is_black(sibling) {
                    // If sibling is red, recolor sibling to black, parent to red, and left rotate at parent
                    (*sibling).color = RbTreeColor::Black;
                    (*parent).color = RbTreeColor::Red;
                    (*parent).left_rotate(self);
                    parent = (*node_to_fix).parent;
                    sibling = (*parent).right;
                }
                if !sibling.is_null() && Self::is_black((*sibling).left) && Self::is_black((*sibling).right) {
                    // If sibling's children are both black, recolor sibling to red and fix at parent
                    (*sibling).color = RbTreeColor::Red;
                    node_to_fix = parent;
                } else {
                    if !sibling.is_null() && Self::is_black((*sibling).right) {
                        // If sibling's right child is black, recolor sibling to red, left child to black, and right rotate at sibling
                        if !(*sibling).left.is_null() {
                            (*(*sibling).left).color = RbTreeColor::Black;
                        }
                        (*sibling).color = RbTreeColor::Red;
                        (*sibling).right_rotate(self);
                        parent = (*node_to_fix).parent;
                        sibling = (*parent).right;
                    }
                    // If sibling's right child is red, recolor sibling to parent's color, parent to black, right child to black, and left rotate at parent
                    (*sibling).color = (*parent).color;
                    (*parent).color = RbTreeColor::Black;
                    if !(*sibling).right.is_null() {
                        (*(*sibling).right).color = RbTreeColor::Black;
                    }
                    (*parent).left_rotate(self);
                    parent = (*node_to_fix).parent;
                    node_to_fix = self.root;
                }
            } else {
                // If node is the right child
                let mut sibling = (*parent).left;
                if !Self::is_black(sibling) {
                    // If sibling is red, recolor sibling to black, parent to red, and right rotate at parent
                    (*sibling).color = RbTreeColor::Black;
                    (*parent).color = RbTreeColor::Red;
                    (*parent).right_rotate(self);
                    parent = (*node_to_fix).parent;
                    sibling = (*parent).left;
                }
                if !sibling.is_null() && Self::is_black((*sibling).left) && Self::is_black((*sibling).right) {
                    // If sibling's children are both black, recolor sibling to red and fix at parent
                    (*sibling).color = RbTreeColor::Red;
                    node_to_fix = parent;
                } else {
                    if !sibling.is_null() && Self::is_black((*sibling).left) {
                        // If sibling's left child is black, recolor sibling to red, right child to black, and left rotate at sibling
                        if !(*sibling).right.is_null() {
                            (*(*sibling).right).color = RbTreeColor::Black;
                        }
                        (*sibling).color = RbTreeColor::Red;
                        (*sibling).left_rotate(self);
                        parent = (*node_to_fix).parent;
                        sibling = (*parent).left;
                    }
                    // If sibling's left child is red, recolor sibling to parent's color, parent to black, left child to black, and right rotate at parent
                    (*sibling).color = (*parent).color;
                    (*parent).color = RbTreeColor::Black;
                    if !(*sibling).left.is_null() {
                        (*(*sibling).left).color = RbTreeColor::Black;
                    }
                    (*parent).right_rotate(self);
                    parent = (*node_to_fix).parent;
                    node_to_fix = self.root;
                }
            }
        }
        // If node is red (i.e. it is the root), recolor it to black!
        (*node_to_fix).color = RbTreeColor::Black;
    }
    pub fn is_attached(&self, mut link: *mut RbTreeLink) -> bool {
        while !link.is_null() {
            if self.root == link {
                return true;
            }
            link = unsafe { (*link).parent };
        }
        false
    }
    pub fn delete(&mut self, node: &mut T) {
        let link = unsafe { (*node).link_ptr() };
        if !self.is_attached(link) {
            return;
        }
        unsafe {
            let (need_fix, node_to_fix) = self.just_delete(link);
            if need_fix {
                self.fix_delete(node_to_fix);
            }
            (*link).parent = ptr::null_mut();
            (*link).left = ptr::null_mut();
            (*link).right = ptr::null_mut();
        }

        self.size -= 1;
    }

    pub fn head(&mut self) -> Option<&mut T> {
        if self.size == 0 {
            None
        } else {
            let mut node = self.root;
            while !unsafe { (*node).right.is_null() } {
                node = unsafe { &mut *(*node).right };
            }
            let ret = T::container(node);
            Some(ret)
        }
    }
}