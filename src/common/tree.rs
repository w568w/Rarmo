use core::ptr;
use crate::common::list::ListNode;
use crate::common::tree::safe_link::{color, is_black, is_left_child, is_right_child, left_of, left_rotate_if_possible, parent_of, right_of, right_rotate_if_possible, set_color};

#[derive(Clone, Copy)]
pub enum RbTreeColor {
    Red,
    Black,
}

// A implementation of red-black tree.
// This implementation is based on this article: https://blog.csdn.net/u014454538/article/details/120120216
// It is not thread-safe, so use it with a lock.
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

// Provide some helper functions for `RbTreeLink` which can be safely invoked on a null pointer.
mod safe_link {
    use core::ptr;
    use crate::common::list::ListNode;
    use crate::common::tree::{RbTree, RbTreeColor, RbTreeLink};

    fn map_not_null<F>(link: *mut RbTreeLink, f: F) -> *mut RbTreeLink
        where F: FnOnce(&mut RbTreeLink) -> *mut RbTreeLink {
        if link != ptr::null_mut() {
            f(unsafe { &mut *link })
        } else {
            ptr::null_mut()
        }
    }

    fn do_if_not_null<F>(link: *mut RbTreeLink, f: F)
        where F: FnOnce(&mut RbTreeLink) {
        if link != ptr::null_mut() {
            f(unsafe { &mut *link })
        }
    }

    pub(super) fn left_rotate_if_possible<T: ListNode<RbTreeLink>>(parent: *mut RbTreeLink, tree: &mut RbTree<T>) {
        do_if_not_null(parent, |parent|
            do_if_not_null(right_of(parent), |_| unsafe { parent.left_rotate(tree) }));
    }

    pub(super) fn right_rotate_if_possible<T: ListNode<RbTreeLink>>(parent: *mut RbTreeLink, tree: &mut RbTree<T>) {
        do_if_not_null(parent, |parent|
            do_if_not_null(left_of(parent), |_| unsafe { parent.right_rotate(tree) }));
    }

    pub(super) fn parent_of(link: *mut RbTreeLink) -> *mut RbTreeLink {
        map_not_null(link, |link| link.parent)
    }

    pub(super) fn left_of(link: *mut RbTreeLink) -> *mut RbTreeLink {
        map_not_null(link, |link| link.left)
    }

    pub(super) fn right_of(link: *mut RbTreeLink) -> *mut RbTreeLink {
        map_not_null(link, |link| link.right)
    }

    pub(super) fn is_left_child(child: *mut RbTreeLink, parent: *mut RbTreeLink) -> bool {
        if child == ptr::null_mut() || parent == ptr::null_mut() {
            false
        } else {
            left_of(parent) == child
        }
    }

    pub(super) fn is_right_child(child: *mut RbTreeLink, parent: *mut RbTreeLink) -> bool {
        if child == ptr::null_mut() || parent == ptr::null_mut() {
            false
        } else {
            right_of(parent) == child
        }
    }

    pub(super) fn set_color(link: *mut RbTreeLink, color: RbTreeColor) {
        do_if_not_null(link, |link| link.color = color)
    }

    pub(super) fn color(link: *mut RbTreeLink) -> RbTreeColor {
        if link == ptr::null_mut() {
            RbTreeColor::Black
        } else {
            unsafe { (*link).color }
        }
    }

    pub(super) fn is_black(link: *mut RbTreeLink) -> bool {
        link.is_null() || unsafe { matches!((*link).color,RbTreeColor::Black) }
    }
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
            let parent_left = left_of(self.parent) as *const RbTreeLink;
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

impl<T: ListNode<RbTreeLink> + 'static> core::fmt::Display for RbTree<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Print the whole tree as a bracketed expression.
        // (root (left) (right))
        // (root (left))
        // (root () (right))
        // (root)
        // ()
        fn print_node(node: *mut RbTreeLink, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            if node.is_null() {
                write!(f, "()")?;
            } else {
                let node = unsafe { &mut *node };
                write!(f, "{}(", match node.color {
                    RbTreeColor::Red => "R",
                    RbTreeColor::Black => "B",
                })?;
                print_node(node.left, f)?;
                write!(f, " ")?;
                print_node(node.right, f)?;
                write!(f, ")")?;
            }
            Ok(())
        }
        print_node(self.root, f)
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


    fn fix_insert(&mut self, mut node_to_fix: *mut RbTreeLink) {
        set_color(node_to_fix, RbTreeColor::Red);

        while !node_to_fix.is_null() && self.root != node_to_fix && !is_black(parent_of(node_to_fix)) {
            let mut parent = parent_of(node_to_fix);
            let mut grandparent = parent_of(parent);
            if is_left_child(parent, grandparent) {
                // If parent is the left child
                let uncle = right_of(grandparent);
                if !is_black(uncle) {
                    // If uncle is red, recolor parent and uncle to black, grandparent to red,
                    // and fix at grandparent`
                    set_color(parent, RbTreeColor::Black);
                    set_color(uncle, RbTreeColor::Black);
                    set_color(grandparent, RbTreeColor::Red);
                    node_to_fix = grandparent;
                } else {
                    // If uncle is black and node is the left child, we need to recolor parent to black, grandparent to red,
                    // and right rotate at grandparent
                    if is_right_child(node_to_fix, parent) {
                        // Special case: if node is the right child, left rotate at parent first!
                        // And then node will be set to its parent. `parent` will be set to original `node_to_fix`.
                        node_to_fix = parent;
                        left_rotate_if_possible(node_to_fix, self);
                        parent = parent_of(node_to_fix);
                        grandparent = parent_of(parent);
                    }
                    set_color(parent, RbTreeColor::Black);
                    set_color(grandparent, RbTreeColor::Red);
                    right_rotate_if_possible(grandparent, self);
                }
            } else {
                let uncle = left_of(grandparent);
                if !is_black(uncle) {
                    set_color(parent, RbTreeColor::Black);
                    set_color(uncle, RbTreeColor::Black);
                    set_color(grandparent, RbTreeColor::Red);
                    node_to_fix = grandparent;
                } else {
                    if is_left_child(node_to_fix, parent) {
                        node_to_fix = parent;
                        right_rotate_if_possible(node_to_fix, self);
                        parent = parent_of(node_to_fix);
                        grandparent = parent_of(parent);
                    }
                    set_color(parent, RbTreeColor::Black);
                    set_color(grandparent, RbTreeColor::Red);
                    left_rotate_if_possible(grandparent, self);
                }
            }
        }

        // Rule: root is always black!
        set_color(self.root, RbTreeColor::Black);
    }

    fn just_insert(&mut self, link: *mut RbTreeLink) -> bool {
        if self.root.is_null() {
            self.root = link;
            set_color(link, RbTreeColor::Black);
            false
        } else {
            let mut parent = self.root;
            loop {
                if self.inner_cmp(link, parent) {
                    if left_of(parent).is_null() {
                        unsafe { (*parent).set_left_child(link) };
                        return !is_black(parent);
                    } else {
                        parent = left_of(parent);
                    }
                } else {
                    if right_of(parent).is_null() {
                        unsafe { (*parent).set_right_child(link) };
                        return !is_black(parent);
                    } else {
                        parent = right_of(parent);
                    }
                }
            }
        }
    }

    pub fn insert(&mut self, node: *mut T) {
        let link = unsafe { (*node).link_ptr() };
        if self.just_insert(link) {
            self.fix_insert(link);
        }
        self.size += 1;
    }

    unsafe fn just_delete(&mut self, link: *mut RbTreeLink) -> Option<*mut RbTreeLink> {
        let parent = parent_of(link);
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
        } else if is_left_child(link, parent) {
            (*parent).set_left_child(child);
        } else {
            (*parent).set_right_child(child);
        }
        if child.is_null() {
            // If `child` is null (i.e. `link` has no child), we can just delete `link` (we have already done it) and return.
            None
        } else {
            // We need to fix the tree if `link` is black.
            (*child).parent = parent;
            set_color(child, color(link));

            if is_black(child) { Some(child) } else { None }
        }
    }

    fn fix_delete(&mut self, mut node_to_fix: *mut RbTreeLink) {
        while !node_to_fix.is_null() && self.root != node_to_fix && is_black(node_to_fix) {
            let mut parent = parent_of(node_to_fix);
            if is_left_child(node_to_fix, parent) {
                // If node is the left child
                let mut sibling = right_of(parent);
                if !is_black(sibling) {
                    // If sibling is red, recolor sibling to black, parent to red, and left rotate at parent
                    set_color(sibling, RbTreeColor::Black);
                    set_color(parent, RbTreeColor::Red);
                    left_rotate_if_possible(parent, self);
                    parent = parent_of(node_to_fix);
                    sibling = right_of(parent);
                }
                if is_black(left_of(sibling)) && is_black(right_of(sibling)) {
                    // If sibling's children are both black, recolor sibling to red and fix at parent
                    set_color(sibling, RbTreeColor::Red);
                    node_to_fix = parent;
                } else {
                    if is_black(right_of(sibling)) {
                        // If sibling's right child is black, recolor sibling to red, left child to black, and right rotate at sibling
                        set_color(sibling, RbTreeColor::Red);
                        set_color(left_of(sibling), RbTreeColor::Black);
                        right_rotate_if_possible(sibling, self);
                        parent = parent_of(node_to_fix);
                        sibling = right_of(parent);
                    }
                    // If sibling's right child is red, recolor sibling to parent's color, parent to black, right child to black, and left rotate at parent
                    set_color(sibling, color(parent));
                    set_color(parent, RbTreeColor::Black);
                    set_color(right_of(sibling), RbTreeColor::Black);
                    left_rotate_if_possible(parent, self);
                    node_to_fix = self.root;
                }
            } else {
                // If node is the right child, symmetric to the above
                let mut sibling = left_of(parent);
                if !is_black(sibling) {
                    set_color(sibling, RbTreeColor::Black);
                    set_color(parent, RbTreeColor::Red);
                    right_rotate_if_possible(parent, self);
                    parent = parent_of(node_to_fix);
                    sibling = left_of(parent);
                }
                if is_black(left_of(sibling)) && is_black(right_of(sibling)) {
                    set_color(sibling, RbTreeColor::Red);
                    node_to_fix = parent;
                } else {
                    if is_black(left_of(sibling)) {
                        set_color(sibling, RbTreeColor::Red);
                        set_color(right_of(sibling), RbTreeColor::Black);
                        left_rotate_if_possible(sibling, self);
                        parent = parent_of(node_to_fix);
                        sibling = left_of(parent);
                    }
                    set_color(sibling, color(parent));
                    set_color(parent, RbTreeColor::Black);
                    set_color(left_of(sibling), RbTreeColor::Black);
                    right_rotate_if_possible(parent, self);
                    node_to_fix = self.root;
                }
            }
        }
        // If node is red (i.e. it is the root), recolor it to black!
        set_color(node_to_fix, RbTreeColor::Black);
    }

    // Return whether `link` is in the tree.
    //
    // Note: it only checks the root of `link`, and does not guarantee that `link` is really IN the tree.
    // If you need to check that, use `contains` instead.
    pub fn is_attached(&self, mut link: *mut RbTreeLink) -> bool {
        while !link.is_null() {
            if self.root == link {
                return true;
            }
            link = parent_of(link);
        }
        false
    }

    pub fn delete(&mut self, node: &mut T) {
        let link = unsafe { (*node).link_ptr() };
        if !self.is_attached(link) {
            return;
        }
        unsafe {
            if let Some(node_to_fix) = self.just_delete(link) {
                self.fix_delete(node_to_fix);
            }
            (*link).parent = ptr::null_mut();
            (*link).left = ptr::null_mut();
            (*link).right = ptr::null_mut();
            (*link).color = RbTreeColor::Red;
        }

        self.size -= 1;
    }

    // Return the biggest node in the tree.
    pub fn head(&mut self) -> Option<&mut T> {
        if self.size == 0 {
            None
        } else {
            let mut node = self.root;
            while !right_of(node).is_null() {
                node = right_of(node);
            }
            let ret = T::container(node);
            Some(ret)
        }
    }
}