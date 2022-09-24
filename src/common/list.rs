use core::marker::PhantomData;
use core::ptr;


pub struct ListLink {
    pub prev: *mut ListLink,
    pub next: *mut ListLink,
}

pub trait ListNode {
    fn get_link(&mut self) -> *mut ListLink {
        let ptr = self as *mut Self as *mut ListLink;
        unsafe { ptr.byte_add(Self::get_link_offset()) }
    }
    fn link(&mut self) -> &mut ListLink {
        unsafe { &mut *self.get_link() }
    }
    fn get_link_offset() -> usize;
}

impl ListLink {
    pub fn new() -> Self {
        Self {
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }
    pub fn init(&mut self) {
        self.next = self;
        self.prev = self;
    }
    pub fn insert_after<T: ListNode>(&mut self, node: *mut T) {
        let node_link = unsafe { (*node).get_link() };
        unsafe {
            (*node_link).prev = self;
            (*node_link).next = self.next;
            (*(self.next)).prev = node_link;
        }
        self.next = node_link;
    }
    pub fn no_next(&self) -> bool {
        let self_addr = self as *const Self as usize;
        let next_addr = self.next as usize;
        next_addr == self_addr
    }
    pub fn container<T: ListNode>(&mut self) -> *mut T {
        let ptr = self as *mut Self as *mut T;
        unsafe { ptr.byte_sub(T::get_link_offset()) }
    }

    pub fn last<T: ListNode>(&self) -> Option<&mut T> {
        if self.no_next() {
            None
        } else {
            unsafe { (*(self.prev)).container::<T>().as_mut() }
        }
    }
    pub fn next<T: ListNode>(&self) -> Option<&mut T> {
        let self_addr = self as *const Self as usize;
        let next_addr = self.next as usize;
        if self.no_next() {
            None
        } else {
            unsafe { (*(self.next)).container::<T>().as_mut() }
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

    pub fn iter<T>(&mut self) -> IterationInfo<T> {
        IterationInfo {
            head: self as *mut Self,
            cur: self as *mut Self,
            typ: PhantomData,
        }
    }
}


pub struct IterationInfo<T> {
    head: *mut ListLink,
    cur: *mut ListLink,
    typ: PhantomData<T>,
}

impl<T: ListNode> Iterator for IterationInfo<T> {
    type Item = *mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.head == unsafe { (*(self.cur)).next } {
            None
        } else {
            self.cur = unsafe { (*(self.cur)).next };
            Some(unsafe { (*(self.cur)).container::<T>() })
        }
    }
}