//! A buddies allocator.
//!
//! This can be used to allocate blocks of different sizes from a single
//! fixed-width block, and is useful in bare-metal physical memory allocation
//! (which is how Linux does things).
use core::mem::MaybeUninit;
use bitvec::prelude::*;
use crate::common::list::{ListLink, ListNode};

/// [`RawBuddies`]: A slightly unsafe buddy allocator.
///
/// A small size and no standard library dependency is traded for an unsafe
/// structure. A safe shell can be constructed around this for built-in
/// allocation of resources as well as a safe allocation result.
pub struct RawBuddies<T: ListNode<ListLink>> {
    /// The number of buddies. Cannot be greater than 64.
    num: usize,
    /// A pointer to the first data element (size 2^num).
    data: *mut T,
    /// A pointer to the first bitspace byte.
    bits: *mut u8,
    /// A list of buddies in each order.
    free_list: [ListLink; 64],
}

impl<T: ListNode<ListLink>> RawBuddies<T> {
    /// Creates a new [`RawBuddies`].
    ///
    /// ### Conditions
    /// `data` and `bits` are not dropped as long as the instantiation lives.
    /// `data` is at least of length `2^num`. It may be uninitialized.
    /// `bits` is at least of length `2^num/8` (i.e it holds `2^num` bits). It
    /// must only contain `0`s (i.e `false`s).
    pub fn uninit(num: usize, data: *mut T, bits: *mut u8) -> Self {
        let mut free_list: [MaybeUninit<ListLink>; 64] = MaybeUninit::uninit_array();
        for i in 0..64 {
            free_list[i] = MaybeUninit::new(ListLink::uninit());
        }
        unsafe {
            let delta = bits.add((1 << num) / 8).offset_from(data as *mut u8);
            assert!(delta <= 0);
        }
        Self {
            num,
            data,
            bits,
            free_list: unsafe { core::mem::transmute(free_list) },
        }
    }

    pub fn init(&mut self) {
        for free_head in self.free_list.iter_mut() {
            free_head.init();
        }
        self.clear_bits();
        // Insert the whole block into the free list's highest order.
        self.free_list[self.num - 1].insert_at_first(self.data);
    }

    fn clear_bits(&mut self) {
        for i in 0..self.num {
            self.buddymap_mut(i).fill(false);
        }
    }

    /// Allocates a block of size `2^n` `T`s.
    ///
    /// Note for safe shells: You want to convert the pointer to a slice such
    /// that multiple (mutable) slices can be held simultaneously.
    ///
    /// Returns the reference as well as the block index (for freeing later).
    pub fn allocate(&mut self, n: usize) -> Option<(*mut T, usize)> {
        if n >= self.num {
            return None;
        }
        if self.free_list[n].is_single() {
            // We have no free blocks of this size.
            // Try to split a larger block.
            if !self.spilt_buddy(n + 1) {
                return None;
            }
        }
        let block: *mut T = self.free_list[n].next_ptr::<T>().unwrap();
        unsafe { (*block).link().detach() };
        let pos = self.pos(n, block);
        self.set_network(n, pos, true);
        // Return the block
        Some((block, pos))
    }

    /// Split a block of size `2^n` into two blocks of size `2^(n-1)`.
    fn spilt_buddy(&mut self, n: usize) -> bool {
        if n >= self.num || n == 0 {
            return false;
        }
        if self.free_list[n].is_single() {
            // We have no free blocks of this size.
            // Need to spilt a larger block.
            if !self.spilt_buddy(n + 1) {
                return false;
            }
        }
        if self.free_list[n].is_single() {
            // We haven't spilt anything, we're out of luck here.
            false
        } else {
            let buddy_to_split: *mut T = self.free_list[n].next_ptr::<T>().unwrap();
            unsafe { (*buddy_to_split).link().detach() };
            let first_buddy: *mut T = buddy_to_split;
            let second_buddy: *mut T = unsafe { buddy_to_split.add(1 << (n - 1)) };
            self.free_list[n - 1].insert_at_first(second_buddy);
            self.free_list[n - 1].insert_at_first(first_buddy);
            true
        }
    }
    pub fn pos(&self, n: usize, buddy: *mut T) -> usize {
        assert!(n < self.num);
        let pos = unsafe { buddy.offset_from(self.data) } as usize;
        pos >> n
    }
    fn block(&self, n: usize, pos: usize) -> *mut T {
        assert!(n < self.num);
        unsafe { self.data.add(pos << n) }
    }
    /// Frees a given block by index and size.
    ///
    /// ### Panics
    /// Panics if the block size is too large (`>= buddies`).
    /// Panics if the index is too large (`>= 2^(buddies-size-1)`).
    /// Panics if the block was already free (possible double-free).
    pub fn free(&mut self, n: usize, pos: usize) {
        assert!(n < self.num);
        assert!(pos < (1usize << (self.num - n - 1)));
        assert!(self.buddymap_ref(n)[pos]);
        // Free the network
        self.set_network(n, pos, false);

        let mut block: *mut T = self.block(n, pos);
        for order in n..self.num {
            self.free_list[order].insert_at_first(block);
            if order == self.num - 1 {
                break;
            }
            let buddy_pos = (pos >> (order - n)) ^ 1;
            if !self.buddymap_ref(order)[buddy_pos] {
                // The buddy is free too, so we can merge.
                unsafe {
                    (*block).link().detach();
                    (*self.block(order, buddy_pos)).link().detach();
                }
                // Turn to the next order's block.
                block = self.block(order + 1, buddy_pos >> 1);
            } else {
                break;
            }
        }
    }

    /// Retrieves a bit slice for a certain buddy immutably.
    ///
    /// Primarily defined for [`can_allocate`].
    fn buddymap_ref(&self, n: usize) -> &BitSlice<u8> {
        assert!(n < self.num);
        // Index is 2^(num-n) from end
        let bits: &BitSlice<u8> = BitSlice::from_slice(unsafe {
            core::slice::from_raw_parts(self.bits, 1usize << self.num)
        });
        &bits[
            (1usize << self.num) - (1usize << (self.num - n))
                ..(1usize << self.num) - (1usize << (self.num - n - 1))
            ]
    }

    /// Retrieves a bit slice for a certain buddy mutably.
    fn buddymap_mut(&mut self, n: usize) -> &mut BitSlice<u8> {
        assert!(n < self.num);
        // Index is 2^(num-n) from end
        let bits: &mut BitSlice<u8> = BitSlice::<u8>::from_slice_mut(unsafe {
            core::slice::from_raw_parts_mut(self.bits, 1usize << self.num)
        });
        &mut bits[
            (1usize << self.num) - (1usize << (self.num - n))
                ..(1usize << self.num) - (1usize << (self.num - n - 1))
            ]
    }

    /// Sets a 'network' of bits around a single set.
    ///
    /// Primarily defined for [`allocate`] and [`free`].
    fn set_network(&mut self, n: usize, i: usize, v: bool) {
        assert!(n < self.num);
        // Begin by setting the lower network of bits
        for b in 0..n {
            self.buddymap_mut(b)[i << (n - b)..(i + 1) << (n - b)].fill(v);
        }
        // Set the higher network of bits
        // v == true: Keep setting until we hit another true
        // v == false: Keep setting until the other is true
        if v {
            for b in n..self.num {
                let map = self.buddymap_mut(b);
                if map[i >> (b - n)] {
                    break;
                }
                map.set(i >> (b - n), true);
            }
        } else {
            for b in n..self.num {
                let map = self.buddymap_mut(b);
                map.set(i >> (b - n), false);
                if map[(i >> (b - n)) ^ 1] {
                    break;
                }
            }
        }
    }
}