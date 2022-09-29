use core::marker::PhantomData;
use core::ptr;

pub struct ListLink {
    pub prev: *mut ListLink,
    pub next: *mut ListLink,
}

pub trait ListNode {
    fn link_ptr(&mut self) -> *mut ListLink {
        let ptr = self as *mut Self as *mut ListLink;
        unsafe { ptr.byte_add(Self::get_link_offset()) }
    }
    fn link(&mut self) -> &mut ListLink {
        unsafe { &mut *self.link_ptr() }
    }
    fn get_link_offset() -> usize;
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
    pub fn insert_at_first<T: ListNode>(&mut self, node: *mut T) {
        let node_link = unsafe { (*node).link_ptr() };
        unsafe {
            (*node_link).prev = self;
            (*node_link).next = self.next;
            (*(self.next)).prev = node_link;
        }
        self.next = node_link;
    }

    pub fn merge(&mut self, other: &mut ListLink) {
        let other_next = other.next;
        other.next = self.next;
        self.next = other_next;
        unsafe {
            (*(other.next)).prev = other;
            (*(self.next)).prev = self;
        }
    }

    pub fn is_single(&self) -> bool {
        let self_addr = self as *const Self as usize;
        let next_addr = self.next as usize;
        next_addr == self_addr
    }
    pub fn container_ptr<T: ListNode>(&mut self) -> *mut T {
        let ptr = self as *mut Self as *mut T;
        unsafe { ptr.byte_sub(T::get_link_offset()) }
    }

    pub fn container<T: ListNode>(&mut self) -> &mut T {
        unsafe { &mut *self.container_ptr::<T>() }
    }
    pub fn prev<T: ListNode>(&self) -> Option<&mut T> {
        if self.is_single() {
            None
        } else {
            unsafe { (*(self.prev)).container_ptr::<T>().as_mut() }
        }
    }
    pub fn next<T: ListNode>(&self) -> Option<&mut T> {
        if self.is_single() {
            None
        } else {
            unsafe { (*(self.next)).container_ptr::<T>().as_mut() }
        }
    }

    pub fn next_ptr<T: ListNode>(&self) -> Option<*mut T> {
        if self.is_single() {
            None
        } else {
            Some(unsafe { (*(self.next)).container_ptr::<T>() })
        }
    }

    pub fn detach(&mut self) {
        unsafe {
            (*(self.prev)).next = self.next;
            (*(self.next)).prev = self.prev;
        }
        self.prev = self;
        self.next = self;
    }

    pub fn iter<T>(&mut self, skip_self: bool) -> IterationInfo<T> {
        IterationInfo {
            head: self as *mut Self,
            cur: self as *mut Self,
            typ: PhantomData,
            skip_head: skip_self,
            has_walked_head: false,
        }
    }
}


pub struct IterationInfo<T> {
    head: *mut ListLink,
    cur: *mut ListLink,
    typ: PhantomData<T>,
    skip_head: bool,
    has_walked_head: bool,
}

impl<T: ListNode + 'static> Iterator for IterationInfo<T> {
    type Item = &'static mut T;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have walked over `head` again, we should stop
        if self.head == self.cur && self.has_walked_head {
            return None;
        }
        // If `head` is the only element and we skip it, we should stop
        if unsafe { (*(self.head)).is_single() } && self.skip_head {
            return None;
        }

        // Now we have walked over `head` at least once, so set `has_walked_head` to true
        self.has_walked_head = true;
        if self.skip_head {
            self.skip_head = false;
            self.cur = unsafe { (*(self.cur)).next };
        }

        let ret = unsafe { (*(self.cur)).container::<T>() };
        self.cur = unsafe { (*(self.cur)).next };
        Some(ret)
    }
}

pub trait InplaceFilter {
    type Item;
    fn filter_inplace(self, f: fn(Self::Item) -> bool, after_detach: Option<fn(Self::Item)>);
}

impl<T: ListNode + 'static> InplaceFilter for IterationInfo<T> {
    type Item = &'static mut T;

    fn filter_inplace(mut self, f: fn(Self::Item) -> bool, after_detach: Option<fn(Self::Item)>) {
        loop {
            // If we have walked over `head` again, we should stop
            if self.head == self.cur && self.has_walked_head {
                break;
            }
            // If `head` is the only element and we skip it, we should stop
            if unsafe { (*(self.head)).is_single() } && self.skip_head {
                break;
            }
            self.has_walked_head = true;

            if self.skip_head {
                self.skip_head = false;
                self.cur = unsafe { (*(self.cur)).next };
            }

            let cur_container = unsafe { (*(self.cur)).container::<T>() };
            let next = unsafe { (*(self.cur)).next };
            if !f(cur_container) {
                if unsafe { (*(self.cur)).is_single() } {
                    panic!("Trying to remove the last element of the list!");
                }
                unsafe { (*(self.cur)).detach() };
                if let Some(after_detach) = after_detach {
                    after_detach(unsafe { (*(self.cur)).container::<T>() });
                }
            }
            self.cur = next;
        }
    }
}