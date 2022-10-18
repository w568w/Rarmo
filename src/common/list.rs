use core::marker::PhantomData;
use core::ptr;
use crate::common::list::safe_link::{detach, is_single, link_of, next_of, set_next, set_prev};

pub struct ListLink {
    pub prev: *mut ListLink,
    pub next: *mut ListLink,
}

pub fn map_not_null<T, F>(link: *mut T, f: F) -> *mut T
    where F: FnOnce(&mut T) -> *mut T {
    if link != ptr::null_mut() {
        f(unsafe { &mut *link })
    } else {
        ptr::null_mut()
    }
}

pub fn do_if_not_null<T, F>(link: *mut T, f: F)
    where F: FnOnce(&mut T) {
    if link != ptr::null_mut() {
        f(unsafe { &mut *link })
    }
}

pub trait ListNode<LinkType> {
    fn link_ptr(&mut self) -> *mut LinkType {
        let ptr = self as *mut Self as *mut LinkType;
        unsafe { ptr.byte_add(Self::get_link_offset()) }
    }
    fn link(&mut self) -> &mut LinkType {
        unsafe { &mut *self.link_ptr() }
    }

    fn container_ptr<ContainerType: ListNode<LinkType>>(link: *mut LinkType) -> *mut ContainerType {
        let ptr = link as *mut ContainerType;
        unsafe { ptr.byte_sub(ContainerType::get_link_offset()) }
    }

    fn container<ContainerType: ListNode<LinkType>>(link: *mut LinkType) -> &'static mut ContainerType {
        unsafe { &mut *ContainerType::container_ptr(link) }
    }

    fn get_link_offset() -> usize;
}

mod safe_link {
    use crate::common::list::{do_if_not_null, map_not_null};
    use super::*;

    pub(super) fn set_prev(link: *mut ListLink, prev: *mut ListLink) {
        do_if_not_null(link, |link| link.prev = prev);
    }

    pub(super) fn set_next(link: *mut ListLink, next: *mut ListLink) {
        do_if_not_null(link, |link| link.next = next);
    }

    pub(super) fn prev_of(link: *mut ListLink) -> *mut ListLink {
        map_not_null(link, |link| link.prev)
    }

    pub(super) fn next_of(link: *mut ListLink) -> *mut ListLink {
        map_not_null(link, |link| link.next)
    }

    pub(super) fn link_of<T: ListNode<ListLink>>(node: *mut T) -> *mut ListLink {
        if node.is_null() {
            ptr::null_mut()
        } else {
            unsafe { (*node).link_ptr() }
        }
    }

    pub(super) fn is_single(link: *mut ListLink) -> bool {
        !link.is_null() && link == next_of(link)
    }

    pub(super) fn detach(link: *mut ListLink) {
        do_if_not_null(link, |link| link.detach());
    }
}

impl ListLink {
    pub const fn uninit() -> Self {
        Self {
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }
    pub fn init(&mut self) {
        self.next = self;
        self.prev = self;
    }
    pub fn insert_at_first<T: ListNode<ListLink>>(&mut self, node: *mut T) {
        let node_link = link_of(node);
        set_prev(node_link, self);
        set_next(node_link, self.next);
        set_prev(self.next, node_link);
        set_next(self, node_link);
    }

    pub fn insert_at_last<T: ListNode<ListLink>>(&mut self, node: *mut T) {
        do_if_not_null(self.prev, |prev| prev.insert_at_first(node));
    }

    pub fn merge(&mut self, other: &mut ListLink) {
        let other_next = other.next;
        other.next = self.next;
        self.next = other_next;
        set_prev(next_of(other), other);
        set_prev(self.next, self);
    }

    pub fn is_single(&self) -> bool {
        let self_addr = self as *const Self as usize;
        let next_addr = self.next as usize;
        next_addr == self_addr
    }

    pub fn prev<T: ListNode<ListLink> + 'static>(&self) -> Option<&mut T> {
        if self.is_single() {
            None
        } else {
            Some(T::container(self.prev))
        }
    }
    pub fn next<T: ListNode<ListLink> + 'static>(&self) -> Option<&mut T> {
        if self.is_single() {
            None
        } else {
            Some(T::container(self.next))
        }
    }

    pub fn next_ptr<T: ListNode<ListLink>>(&self) -> Option<*mut T> {
        if self.is_single() {
            None
        } else {
            Some(T::container_ptr(self.next))
        }
    }

    pub fn detach(&mut self) {
        set_next(self.prev, self.next);
        set_prev(self.next, self.prev);
        self.prev = self;
        self.next = self;
    }

    pub fn iter<T>(&mut self, skip_self: bool) -> LinkedListIterationInfo<T> {
        LinkedListIterationInfo {
            head: self as *mut Self,
            cur: self as *mut Self,
            typ: PhantomData,
            skip_head: skip_self,
            has_walked_head: false,
        }
    }
}


pub struct LinkedListIterationInfo<T> {
    head: *mut ListLink,
    cur: *mut ListLink,
    typ: PhantomData<T>,
    skip_head: bool,
    has_walked_head: bool,
}

impl<Container: ListNode<ListLink> + 'static> Iterator for LinkedListIterationInfo<Container> {
    type Item = &'static mut Container;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have walked over `head` again, we should stop
        if self.head == self.cur && self.has_walked_head {
            return None;
        }
        // If `head` is the only element and we skip it, we should stop
        if is_single(self.head) && self.skip_head {
            return None;
        }

        // Now we have walked over `head` at least once, so set `has_walked_head` to true
        self.has_walked_head = true;
        if self.skip_head {
            self.skip_head = false;
            self.cur = next_of(self.cur);
        }

        let ret = Container::container(self.cur);
        self.cur = next_of(self.cur);
        Some(ret)
    }
}

pub trait InplaceFilter {
    type Item;
    fn filter_inplace<F1, F2>(self, f: F1, after_detach: F2)
        where F1: FnMut(Self::Item) -> bool, F2: FnMut(Self::Item) -> bool;
}

impl<T: ListNode<ListLink> + 'static> InplaceFilter for LinkedListIterationInfo<T> {
    type Item = &'static mut T;

    fn filter_inplace<F1, F2>(mut self, mut f: F1, mut after_detach: F2)
        where F1: FnMut(Self::Item) -> bool, F2: FnMut(Self::Item) -> bool {
        loop {
            // If we have walked over `head` again, we should stop
            if self.head == self.cur && self.has_walked_head {
                break;
            }
            // If `head` is the only element and we skip it, we should stop
            if is_single(self.head) && self.skip_head {
                break;
            }
            self.has_walked_head = true;

            if self.skip_head {
                self.skip_head = false;
                self.cur = next_of(self.cur);
            }

            let cur_container = T::container(self.cur);
            let next = next_of(self.cur);
            if !f(cur_container) {
                if is_single(self.cur) {
                    panic!("Trying to remove the last element of the list!");
                }
                detach(self.cur);
                if after_detach(T::container(self.cur)) {
                    return;
                }
            }
            self.cur = next;
        }
    }
}