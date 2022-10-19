use alloc::boxed::Box;
use core::cmp::min;
use core::mem::MaybeUninit;
use core::ptr;
use field_offset::offset_of;
use spin::Mutex;
use crate::aarch64::mmu::PAGE_SIZE;
use crate::common::list::{InplaceFilter, ListLink, ListNode};
use crate::define_early_init;
use crate::kernel::mem::{kalloc_page, kfree_page};
use crate::kernel::proc::Process;
use crate::kernel::proc::ProcessState::Sleeping;
use crate::kernel::sched::{acquire_sched_lock, activate, sched, thisproc};

const SEQ_MULTIPLIER: i32 = 16;
const MAX_MSGNUM: i32 = 256;

pub const IPC_RMID: i32 = 0;
pub const IPC_PRIVATE: i32 = 0;
pub const IPC_CREATE: i32 = 2;
pub const IPC_EXCL: i32 = 1;
pub const IPC_NOWAIT: i32 = 1;

const ENOMEM: i32 = -1;
const ENOSEQ: i32 = -2;
const ENOENT: i32 = -3;
const EEXIST: i32 = -4;
const EINVAL: i32 = -5;
const EAGAIN: i32 = -6;
const EIDRM: i32 = -7;
const E2BIG: i32 = -8;
const ENOMSG: i32 = -9;

const MSG_SIZE: usize = PAGE_SIZE - core::mem::size_of::<Message>();
const MSG_SEG_SIZE: usize = PAGE_SIZE - core::mem::size_of::<MessageSegment>();

struct MessageQueue {
    key: i32,
    seq: i32,
    max_msg: i32,
    sum_msg: i32,
    q_message: ListLink,
    q_sender: ListLink,
    q_receiver: ListLink,
}

struct IPCIds {
    size: i32,
    in_use: i32,
    seq: u8,
    lock: Mutex<()>,
    entries: [*mut MessageQueue; SEQ_MULTIPLIER as usize],
}

trait MessageHeader<T> {
    fn get_data(&self) -> *mut u8 {
        unsafe { (self as *const _ as *mut u8).add(core::mem::size_of::<T>()) }
    }
    fn get_data_slice(&self, len: usize) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.get_data(), len) }
    }
}

pub trait AsMessageBuffer<T> {
    fn as_message_buffer(&mut self) -> &mut MessageBuffer {
        unsafe { &mut *(self as *mut _ as *mut MessageBuffer) }
    }

    fn message_buffer_size() -> usize {
        core::mem::size_of::<T>() - core::mem::size_of::<MessageBuffer>()
    }
}

#[repr(C)]
pub struct MessageBuffer {
    mtype: i32,
}

impl MessageHeader<MessageBuffer> for MessageBuffer {}

#[repr(C)]
struct Message {
    mtype: i32,
    size: usize,
    link: ListLink,
    seg: MessageSegment,
}

impl ListNode<ListLink> for Message {
    fn get_link_offset() -> usize { offset_of!(Message => link).get_byte_offset() }
}

struct MessageSegment {
    next_seg: *mut MessageSegment,
}

impl MessageHeader<MessageSegment> for MessageSegment {}

#[repr(C)]
struct MessageSender {
    proc: *mut Process,
    link: ListLink,
}

impl MessageSender {
    fn new(proc: *mut Process) -> Self {
        Self {
            proc,
            link: ListLink::uninit(),
        }
    }
}

impl ListNode<ListLink> for MessageSender {
    fn get_link_offset() -> usize { offset_of!(MessageSender => link).get_byte_offset() }
}

#[repr(C)]
struct MessageReceiver {
    proc: *mut Process,
    link: ListLink,
    mtype: i32,
    size: usize,
    r_msg: *mut Message,
}

impl MessageReceiver {
    pub fn new(proc: *mut Process, mtype: i32, size: usize) -> Self {
        Self {
            proc,
            link: ListLink::uninit(),
            mtype,
            size,
            r_msg: ptr::null_mut(),
        }
    }
}

impl ListNode<ListLink> for MessageReceiver {
    fn get_link_offset() -> usize { offset_of!(MessageReceiver => link).get_byte_offset() }
}

static mut MSG_IDS: MaybeUninit<IPCIds> = MaybeUninit::uninit();

fn msg_ids() -> &'static mut IPCIds {
    unsafe { MSG_IDS.assume_init_mut() }
}

extern "C" fn init_ipc() {
    unsafe {
        MSG_IDS = MaybeUninit::new(IPCIds {
            size: SEQ_MULTIPLIER,
            in_use: 0,
            seq: 0,
            lock: Mutex::new(()),
            entries: [ptr::null_mut(); SEQ_MULTIPLIER as usize],
        });
    }
}
define_early_init!(init_ipc);

/// Add a message `queue` to the IPC ids.
///
/// It will set the `seq` field of `queue` to the next available sequence number.
fn ipc_add_id(queue: &mut MessageQueue) -> Option<i32> {
    let msg_ids = msg_ids();
    msg_ids.entries.iter_mut().enumerate()
        .find(|(_, e)| e.is_null())
        .map(|(i, e)| {
            msg_ids.in_use += 1;
            queue.seq = msg_ids.seq as i32;
            msg_ids.seq += 1;
            *e = queue;
            i as i32
        })
}

const fn ipc_buildin(id: i32, seq: i32) -> i32 {
    seq * SEQ_MULTIPLIER + id
}

/// Create a message queue with the given `key` and add it to the IPC ids.
///
/// Returns the allocated id for the message queue.
fn new_queue(key: i32) -> Result<i32, i32> {
    let mut queue = Box::new(MessageQueue {
        key,
        seq: 0,
        max_msg: MAX_MSGNUM,
        sum_msg: 0,
        q_message: ListLink::uninit(),
        q_sender: ListLink::uninit(),
        q_receiver: ListLink::uninit(),
    });
    queue.q_message.init();
    queue.q_sender.init();
    queue.q_receiver.init();

    let id = ipc_add_id(queue.as_mut()).ok_or(ENOSEQ)?;

    let ret = ipc_buildin(id, queue.seq);
    let _ = Box::into_raw(queue);

    Ok(ret)
}

/// Get the message queue's id with the given `key`.
fn ipc_findkey(key: i32) -> Option<i32> {
    msg_ids().entries.iter_mut()
        .enumerate()
        .find(|(_, e)| !e.is_null() && unsafe { e.read().key } == key)
        .map(|(i, _)| i as i32)
}

/// Get the message queue with the given `key`.
///
/// Returns the message queue's id.
pub fn sys_msgget(key: i32, msgflg: i32) -> Result<i32, i32> {
    let ipc_ids = msg_ids();
    let _lock = ipc_ids.lock.lock();

    if key == IPC_PRIVATE {
        return new_queue(key);
    }


    if let Some(id) = ipc_findkey(key) {
        // If a message queue with the given `key` exists
        return if msgflg & IPC_EXCL != 0 {
            Err(EEXIST)
        } else {
            let queue = unsafe { &mut *ipc_ids.entries[id as usize] };
            Ok(ipc_buildin(id, queue.seq))
        };
    }

    // Or, if a message queue with the given `key` does not exist
    return if msgflg & IPC_CREATE != 0 {
        new_queue(key)
    } else {
        Err(ENOENT)
    };
}

/// Free the `msg` and its segments.
///
/// After this function, the `msg` will be invalid.
fn drop_msg(msg: *mut Message) {
    let mut m_seg = unsafe { (*msg).seg.next_seg };
    while !m_seg.is_null() {
        let next_msg = unsafe { (*m_seg).next_seg };
        kfree_page(m_seg as *mut u8, 1);
        m_seg = next_msg;
    }
    kfree_page(msg as *mut u8, 1);
}

/// Create a new message with the given `src`.
///
/// Return a pointer to the new message. The message has NOT initialized its `link` field.
fn load_msg(mut src: &[u8]) -> *mut Message {
    let mut len = src.len();
    let msg = kalloc_page(1) as *mut Message;
    // todo: kalloc_page may fail if necessary
    let msg = unsafe { &mut *msg };
    let copied_len = min(len, MSG_SIZE);
    unsafe { msg.seg.get_data().copy_from_nonoverlapping(src.as_ptr(), copied_len); }
    len -= copied_len;
    src = &src[copied_len..];
    msg.seg.next_seg = ptr::null_mut();

    // If the message is too long, we need to allocate more pages
    let mut tail = &mut msg.seg.next_seg;
    while len > 0 {
        let m_seg = kalloc_page(1) as *mut MessageSegment;
        let m_seg = unsafe { &mut *m_seg };
        let copied_len = min(len, MSG_SEG_SIZE);
        unsafe { m_seg.get_data().copy_from_nonoverlapping(src.as_ptr(), copied_len); }
        *tail = m_seg;
        m_seg.next_seg = ptr::null_mut();
        tail = &mut m_seg.next_seg;

        len -= copied_len;
        src = &src[copied_len..];
    }
    msg as *mut Message
}

/// Get the corresponding message queue with the given `msg_id`.
///
/// Note: you should hold the lock of `MSG_IDS` before calling this function.
fn get_msg_queue(msg_id: i32) -> Option<&'static mut MessageQueue> {
    let ipc_ids = msg_ids();
    let id = msg_id % SEQ_MULTIPLIER;
    if id < 0 || id >= ipc_ids.size || ipc_ids.entries[id as usize].is_null() {
        return None;
    }
    let queue = ipc_ids.entries[id as usize];
    let queue = unsafe { &mut *queue };
    if queue.seq != msg_id / SEQ_MULTIPLIER {
        return None;
    }
    Some(queue)
}

/// Determine whether the receiver can receive the message type.
fn test_msg(receive_type: i32, msg_type: i32) -> bool {
    if receive_type == 0 {
        true
    } else if receive_type > 0 {
        receive_type == msg_type
    } else {
        receive_type <= -msg_type
    }
}

/// Send `msg` to `queue`.
///
/// Return true if it finds the first receiver which can receive the message.
/// Otherwise, return false.
///
/// It will wake up all potential receivers before the first possible receiver.
fn pipeline_send(queue: &mut MessageQueue, msg: &mut Message) -> bool {
    let mut ret = false;
    let mtype = msg.mtype;
    queue.q_receiver.iter::<MessageReceiver>(true).filter_inplace(|recv: &mut MessageReceiver| {
        // Only keep the receivers that can not receive this message
        !test_msg(recv.mtype, mtype)
    }, |recv: &mut MessageReceiver| {
        let proc = unsafe { &mut *recv.proc };
        if msg.size > recv.size {
            // The receiver's buffer is too small to receive this message!
            recv.r_msg = ptr::null_mut();
            activate(proc);
            false
        } else {
            // Give the message to the receiver
            recv.r_msg = msg;
            activate(proc);
            ret = true;
            // Break the iteration
            true
        }
    });
    ret
}

/// Create a new message with the given buffer `msgp` and `msg_size`.
/// Then, send the message to the message queue with the given `msg_id`.
///
/// Return 0 on success, or a negative error code on failure.
pub fn sys_msgsend(msg_id: i32, msgp: &mut MessageBuffer, msg_size: usize, msgflg: i32) -> Result<i32, i32> {
    if msgp.mtype < 1 {
        return Err(EINVAL);
    }
    // Create a new message in new pages
    let msg = load_msg(msgp.get_data_slice(msg_size));
    if msg.is_null() {
        return Err(ENOMEM);
    }
    let msg = unsafe { &mut *msg };
    msg.mtype = msgp.mtype;
    msg.size = msg_size;

    loop {
        let lock = msg_ids().lock.lock();
        let queue = get_msg_queue(msg_id);
        if queue.is_none() {
            drop_msg(msg);
            return Err(EIDRM);
        }
        let queue = queue.unwrap();
        if queue.sum_msg + 1 > queue.max_msg {
            // The queue is full, we need to wait!
            if msgflg & IPC_NOWAIT != 0 {
                // But we cannot wait, so we return EAGAIN
                drop_msg(msg);
                return Err(EAGAIN);
            } else {
                // Or we can wait, so we push the current process into the waiting queue
                let mut sender = MessageSender::new(thisproc());
                sender.link.init();
                queue.q_sender.insert_at_last(&mut sender);
                let sched_lock = acquire_sched_lock();
                drop(lock);
                sched(sched_lock, Sleeping);
            }
        } else {
            // The queue is not full, we can send the message
            if !pipeline_send(queue, msg) {
                // If no receiver is waiting, we push the message into the queue
                msg.link.init();
                queue.q_message.insert_at_last(msg);
                queue.sum_msg += 1;
            }
            return Ok(0);
        }
    }
}

/// Pop up all the senders in the queue and wake them up.
///
/// Note: after calling this function, the queue will be empty.
fn wakeup_senders(head: &mut ListLink) {
    head.iter::<MessageSender>(true).filter_inplace(|_| false, |sender: &mut MessageSender| {
        activate(unsafe { &mut *sender.proc });
        false
    });
}

/// Pop up all the receivers in the queue and wake them up.
///
/// Note: after calling this function, the queue will be empty.
fn wakeup_receivers(head: &mut ListLink) {
    head.iter::<MessageReceiver>(true).filter_inplace(|_| false, |receiver: &mut MessageReceiver| {
        receiver.r_msg = ptr::null_mut();
        activate(unsafe { &mut *receiver.proc });
        false
    });
}

/// Store the message into `dst`. This function is a reverse operation of `load_msg`.
///
/// The size of `dst` must be greater than or equal to `msg_size`.
fn store_msg(dst: *mut u8, msg: &mut Message, msg_size: usize) {
    let mut len = msg_size;
    let mut dst = dst;
    let mut m_seg = &mut msg.seg;

    let copied_len = min(len, MSG_SIZE);
    unsafe { dst.copy_from_nonoverlapping(m_seg.get_data(), copied_len); }
    len -= copied_len;
    dst = unsafe { dst.byte_add(copied_len) };
    m_seg = unsafe { &mut *m_seg.next_seg };

    while len > 0 {
        let copied_len = min(len, MSG_SEG_SIZE);
        unsafe { dst.copy_from_nonoverlapping(m_seg.get_data(), copied_len); }
        len -= copied_len;
        dst = unsafe { dst.byte_add(copied_len) };
        m_seg = unsafe { &mut *m_seg.next_seg };
    }
}

/// Receive a message from the message queue with the given `msg_id`.
///
/// Return the message size on success, or a negative error code on failure.
pub fn sys_msgrcv(msg_id: i32, msgp: &mut MessageBuffer, mut msg_size: usize, mut mtype: i32, msgflg: i32) -> Result<i32, i32> {
    let lock = msg_ids().lock.lock();

    let queue = get_msg_queue(msg_id).ok_or(EIDRM)?;

    let mut found_msg: *mut Message = ptr::null_mut();

    // Check the queue for the first possible message (or, the `-mtype`-th message if mtype < 0)
    for msg in queue.q_message.iter::<Message>(true) {
        if test_msg(mtype, msg.mtype) {
            found_msg = msg;
            if mtype < -1 {
                mtype += 1;
            } else {
                break;
            }
        }
    }
    if !found_msg.is_null() {
        // If we find a message...
        let found_msg = unsafe { &mut *found_msg };
        if found_msg.size > msg_size {
            // If the buffer is too small, we return E2BIG
            return Err(E2BIG);
        }
        // This message is the one we want, so we detach it from the queue
        found_msg.link.detach();
        queue.sum_msg -= 1;
        // The queue has space now, so we wake up all the senders
        wakeup_senders(&mut queue.q_sender);
        drop(lock);
    } else {
        // If we cannot find a message...
        if msgflg & IPC_NOWAIT != 0 {
            // If we cannot wait, we return ENOMSG
            return Err(ENOMSG);
        } else {
            // If we can wait, we push the current process into the waiting queue
            let mut receiver = MessageReceiver::new(thisproc(), mtype, msg_size);
            receiver.link.init();
            queue.q_receiver.insert_at_last(&mut receiver);
            let sched_lock = acquire_sched_lock();
            drop(lock);
            sched(sched_lock, Sleeping);

            // After waking up, we check the message again
            found_msg = receiver.r_msg;
            if found_msg.is_null() {
                // In `ss_wakeup`, if the message is too large, we set `r_msg` to null
                return Err(E2BIG);
            }
        }
    }

    // Now we have got a message, so we copy it to the buffer
    let found_msg = unsafe { &mut *found_msg };
    msg_size = min(msg_size, found_msg.size);
    // Store the message into the buffer
    store_msg(msgp.get_data(), found_msg, msg_size);
    msgp.mtype = found_msg.mtype;
    // Drop the message
    drop_msg(found_msg);
    // Return the size of the message
    Ok(msg_size as i32)
}

/// Empty out the message `queue`.
///
/// This function will wake up all the receivers and tell them that "the message is too large".
fn expunge_all(queue: &mut MessageQueue) {
    queue.q_receiver.iter::<MessageReceiver>(true).filter_inplace(|_| false, |receiver: &mut MessageReceiver| {
        receiver.r_msg = ptr::null_mut();
        activate(unsafe { &mut *receiver.proc });
        false
    });
}

/// Drop the message queue with the given `msg_id`.
fn drop_queue(msg_id: i32) {
    let msg_ids = msg_ids();
    let _lock = msg_ids.lock.lock();
    let queue_ptr = get_msg_queue(msg_id);
    if let Some(queue) = queue_ptr {
        // Remove the queue from `MSG_IDS`
        msg_ids.entries[(msg_id % SEQ_MULTIPLIER) as usize] = ptr::null_mut();
        // Wake up all the senders and receivers
        wakeup_receivers(&mut queue.q_receiver);
        expunge_all(queue);
        // Drop all the messages in the queue
        queue.q_message.iter::<Message>(true).filter_inplace(|_| false, |msg: &mut Message| {
            drop_msg(msg);
            false
        });
        msg_ids.in_use -= 1;
        // Drop the queue
        unsafe { let _ = Box::from_raw(queue); }
    }
}

/// Control the message queue with the given `msg_id`.
///
/// Return 0 on success, or a negative error code on failure.
pub fn sys_msgctl(msg_id: i32, cmd: i32) -> Result<i32, i32> {
    match cmd {
        IPC_RMID => {
            drop_queue(msg_id);
            Ok(0)
        }
        _ => Err(EINVAL)
    }
}