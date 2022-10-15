//! [`Buddies`](crate): A buddies allocator.
//!
//! This can be used to allocate blocks of different sizes from a single
//! fixed-width block, and is useful in bare-metal physical memory allocation
//! (which is how Linux does things).
use bitvec::prelude::*;

/// [`RawBuddies`]: A slightly unsafe buddy allocator.
///
/// A small size and no standard library dependency is traded for an unsafe
/// structure. A safe shell can be constructed around this for built-in
/// allocation of resources as well as a safe allocation result.
pub struct RawBuddies<T> {
    /// The number of buddies.
    num: usize,
    /// A pointer to the first data element (size 2^num).
    data: *mut T,
    /// A pointer to the first bitspace byte.
    bits: *mut u8,
}

impl<T> RawBuddies<T> {
    /// Creates a new [`RawBuddies`].
    ///
    /// ### Conditions
    /// `data` and `bits` are not dropped as long as the instantiation lives.
    /// `data` is at least of length `2^num`. It may be uninitialized.
    /// `bits` is at least of length `2^num/8` (i.e it holds `2^num` bits). It
    /// must only contain `0`s (i.e `false`s).
    pub unsafe fn new(num: usize, data: *mut T, bits: *mut u8) -> Self {
        let mut s = Self {
            num,
            data,
            bits,
        };
        s.clear_bits();
        s
    }

    fn clear_bits(&mut self) {
        for i in 0..self.num {
            self.buddymap_mut(i).fill(false);
        }
    }
    /// Checks if a block of size `2^n` `T`s can be allocated.
    ///
    /// ### Panics
    /// Panics if the block size is too large (`>= buddies`).
    pub fn can_allocate(&self, n: usize) -> bool {
        assert!(n < self.num);
        // Check if matching buddy contains free zones
        self.buddymap_ref(n).not_all()
    }

    /// Allocates a block of size `2^n` `T`s.
    ///
    /// Note for safe shells: You want to convert the pointer to a slice such
    /// that multiple (mutable) slices can be held simultaneously.
    ///
    /// Returns the reference as well as the block index (for freeing later).
    ///
    /// ### Panics
    /// Panics if the block size is too large (`>= buddies`).
    pub fn allocate(&mut self, n: usize) -> Option<(*mut T, usize)> {
        assert!(n < self.num);
        // Find a free zone for the buddy (auto-exit on fail)
        let pos = self.buddymap_ref(n).first_zero()?;
        // Mark the network
        self.set_network(n, pos, true);
        // Return the block
        Some((unsafe { self.data.add(pos << n) }, pos))
    }

    pub fn pos(&self, n: usize, buddy: *mut T) -> usize {
        assert!(n < self.num);
        let pos = unsafe { buddy.offset_from(self.data) } as usize;
        pos >> n
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
        // Drop the data
        unsafe { core::ptr::drop_in_place(self.data.add(pos << n)); }
        // Free the network
        self.set_network(n, pos, false);
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
        let bits: &mut BitSlice::<u8> = BitSlice::<u8>::from_slice_mut(unsafe {
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
        // assert!(i < (1 << n));
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