use alloc::boxed::Box;
use field_offset::offset_of;
use crate::common::list::{ListLink, ListNode};
use crate::CONSOLE;
use core::fmt::Write;

#[repr(C)]
struct Data {
    val: u64,
    link: ListLink,
}

impl Data {
    fn new(val: u64) -> Self {
        let mut data = Self {
            val,
            link: ListLink::new(),
        };
        data.link.init();
        data
    }
}

impl ListNode for Data {
    fn get_link_offset() -> usize {
        offset_of!(Data => link).get_byte_offset()
    }
}

pub fn test_list() {
    let mut list_head = ListLink::new();
    list_head.init();
    let mut data1 = Box::new(Data::new(1));
    let mut data2 = Box::new(Data::new(2));
    let mut data3 = Box::new(Data::new(3));
    list_head.insert_after(data1.as_mut());
    list_head.insert_after(data2.as_mut());
    list_head.insert_after(data3.as_mut());
    let mut data = list_head.next::<Data>();
    for item in list_head.iter::<Data>() {
        let mut binding = CONSOLE.write();
        let writer = binding.as_mut().unwrap();
        write!(writer, "{}\n", unsafe { (*item).val });
    }
}