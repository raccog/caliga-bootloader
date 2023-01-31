use core::{alloc::{Allocator, AllocError, Layout}, fmt::Debug, ptr::{NonNull, self}, slice};
#[cfg(not(test))]
use log::debug;
#[cfg(test)]
use std::println as debug;

/// Error cases when using a [`SlabAllocator`]
#[derive(Clone, Copy, Debug)]
pub enum SlabAllocatorError {
    InvalidSize,
}

/// An allocator that can allocate evenly distributed chunks of the same size
///
/// Each chunk of memory in this allocator has the same [`Layout`];
#[derive(Clone, Copy, Debug)]
pub struct SlabAllocator {
    storage: *const u8,
    size: usize,
    slab_layout: Layout,
}

impl SlabAllocator {
    /// Returns the bitmap
    unsafe fn bitmap(&self) -> &mut [u8] {
        slice::from_raw_parts_mut(self.bitmap_ptr() as *mut u8, self.bitmap_size())
    }

    /// Returns the number of usable bits in the bitmap
    ///
    /// Each usable bit corresponds to a single slab in the buffer. Unusable bits do not have any
    /// corresponding slab and cannot be used for allocation.
    ///
    /// NOTE: All bits after the last usable bit should be marked with a `1`; signifying
    /// that the corresponding slab is unusable.
    fn bitmap_bits(&self) -> usize {
        self.buffer_size() / self.slab_layout.size()
    }

    /// Returns a pointer to the bitmap
    unsafe fn bitmap_ptr(&self) -> *const u8 {
        self.storage.add(self.buffer_size())
    }

    /// Returns the size of the bitmap in bytes
    fn bitmap_size(&self) -> usize {
        // This calculation includes the unusable slabs taken up by the bitmap
        let slab_count = self.size / self.slab_layout.size();

        const BITS: usize = u8::BITS as usize;
        let bitmap_size = slab_count / BITS;

        if slab_count % BITS != 0 {
            bitmap_size + 1
        } else {
            bitmap_size
        }
    }

    /// Returns the buffer used for slab allocation
    unsafe fn buffer(&self) -> &mut [u8] {
        slice::from_raw_parts_mut(self.buffer_ptr() as *mut u8, self.buffer_size())
    }

    /// Returns a pointer to the buffer used for slab allocation
    unsafe fn buffer_ptr(&self) -> *const u8 {
        self.storage
    }

    /// Returns the size of the buffer (used for slab allocation) in bytes
    fn buffer_size(&self) -> usize {
        self.size - self.bitmap_size()
    }

    /// Returns the total capacity of slabs controlled by this allocator
    fn capacity(&self) -> usize {
        self.buffer_size() / self.slab_layout.size()
    }

    /// Initializes a new slab allocator with each slab having the same `slab_layout`
    ///
    /// # Errors
    ///
    /// Returns [`SlabAllocatorError::InvalidSize`] if:
    ///
    /// * `size` is not divisible by `slab_layout.size()`
    /// * `size` is not large enough to store two slabs of size `slab_layout.size()`
    /// * `storage` is null
    /// * `storage` is not aligned to `slab_layout.align()`
    pub unsafe fn new(storage: *const u8, size: usize, slab_layout: Layout) -> Result<SlabAllocator, SlabAllocatorError> {
        let layout_size = slab_layout.size();
        if size % layout_size != 0 || size < layout_size * 2 || storage.is_null() || !storage.is_aligned_to(slab_layout.align()) {
            return Err(SlabAllocatorError::InvalidSize);
        }

        let slab_allocator = SlabAllocator {
            storage, size, slab_layout
        };

        slab_allocator.bitmap().fill(0);
        slab_allocator.buffer().fill(0);

        // Mask bits for memory that is unavailable
        // These masked bits are marked with `1`, showing the allocator that their corresponding
        // slabs are unavailable for allocation.
        let slab_count = slab_allocator.capacity();
        let unmasked_bits_count = slab_allocator.bitmap_bits() % u8::BITS as usize;
        let masked_bytes_start = slab_count / u8::BITS as usize;
        const U8_MAX: u8 = u8::MAX;
        let bitmap = slab_allocator.bitmap();

        if unmasked_bits_count != 0 {
            // Mask the first partially-unusable byte of the bitmap
            // Part of this byte might still have usable bits, so `u8::MAX` needs
            // to be shifted to unset those usable bits.
            *&mut bitmap[masked_bytes_start] = U8_MAX << unmasked_bits_count;
        }

        // Mask any further unusable bits
        if masked_bytes_start < bitmap.len() - 1 {
            for bitmap_part in bitmap[masked_bytes_start + 1..].iter_mut() {
                *bitmap_part = U8_MAX;
            }
        }

        debug!("{:#?} bitmap: {:#?} {:#?} buffer: {:#?} bitmap_bits: {:#?} buffer_size: {:#?}", slab_allocator, slab_allocator.bitmap_ptr(), slab_allocator.bitmap(), slab_allocator.buffer_ptr(), slab_allocator.bitmap_bits(),
                slab_allocator.buffer_size());

        Ok(slab_allocator)
    }
}

// Returns the index of the first free bit in a byte (0-7)
//
// Index 0 is the least significant bit, while 7 is the most significant.
//
// # Safety
//
// Panics if `byte == u8::MAX`
fn first_free_bit(mut byte: u8) -> usize {
    for i in 0..u8::BITS as usize {
        // Return index if the associated bit is zero
        if byte & 0x1 == 0 {
            return i;
        }

        // Check the next bit
        byte >>= 1;
    }

    unimplemented!();
}

unsafe impl Allocator for SlabAllocator {
    // Returns `AllocError` if:
    //
    // * `layout.align()` does not match this slab allocator's alignment
    // * There is no memory block large enough to allocate `layout.size()` sequential bytes
    //
    // NOTE: This currently will suffer from some memory fragmentation unless all allocations are the same size
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Return error if `layout` does not match `self.slab_layout`
        if self.slab_layout != layout {
            return Err(AllocError);
        }

        let bitmap = unsafe {
            self.bitmap()
        };
        // Search each byte of the bitmap to find a free slab
        // NOTE: Free slabs are denoted by a `0` in the bitmap.
        for (i, bitmap_part) in bitmap.iter_mut().enumerate() {
            if *bitmap_part < u8::MAX {
                let slab_bit = first_free_bit(*bitmap_part);
                let slab_index = i * u8::BITS as usize + slab_bit;

                // Set bitmap to indicate that the memory location is now used
                *bitmap_part |= 1 << slab_bit;

                unsafe {
                    let ptr = self.buffer_ptr().add(slab_index * self.slab_layout.size());
                    debug!("Alloc {:#?}", ptr);
                    return Ok(NonNull::new(slice::from_raw_parts_mut(ptr as *mut u8, self.slab_layout.size())).unwrap());
                }
            }
        }

        // No memory is available
        return Err(AllocError);
    }

    unsafe fn deallocate(&self, alloc_ptr: NonNull<u8>, layout: Layout) {
        debug!("Dealloc {:#?}", alloc_ptr);

        // Ensure deallocation is valid
        // TODO: Determine if something other than assertions should be used
        let alloc_ptr = alloc_ptr.as_ptr() as *const u8;
        assert!(alloc_ptr >= self.buffer_ptr());
        assert!(alloc_ptr < self.bitmap_ptr());
        assert_eq!(self.slab_layout, layout);

        // Calculate indices for the bit that corresponds to this memory location
        let offset = alloc_ptr.sub_ptr(self.buffer_ptr());
        let slab_index = offset / self.slab_layout.size();
        let byte_idx = slab_index / u8::BITS as usize;
        let bit_idx = slab_index % u8::BITS as usize;

        // Ensure the index is valid
        let bitmap = self.bitmap();
        assert!(byte_idx < bitmap.len());

        // Zero out bit in bitmap
        bitmap[byte_idx] &= !(1 << bit_idx);

        // Zero out freed memory
        ptr::write_bytes(alloc_ptr as *mut u8, 0, self.slab_layout.size());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{boxed::Box, mem, vec, vec::Vec};

    /// A `SlabAllocator` that uses a `Vec` to store its allocations
    #[allow(dead_code)]
    struct VecSlabAlloc {
        pub slab_allocator: SlabAllocator,
        pub layout: Layout,
        pub storage: Vec<u8>,
    }

    /// Initialize a slab allocator with memory from a `Vec`
    ///
    /// Used in other tests
    fn init_slab_alloc<T>(size: usize) -> VecSlabAlloc {
        let storage: Vec<u8> = vec![0; size];
        let layout = Layout::new::<T>();
        let slab_allocator = unsafe {
            SlabAllocator::new(storage.as_ptr(), storage.len(), layout)
                .expect("Failed to create allocator")
        };

        VecSlabAlloc {
            slab_allocator,
            layout,
            storage
        }
    }

    /// Ensures that an allocator of the smallest possible size (1 slab) can be used
    ///
    /// Ensures that only a single allocation is available for an allocator of this capacity
    #[test]
    fn smallest_allocation() {
        fn smallest_allocation_assert(data: u8, slab_allocator: &SlabAllocator) {
            let allocated = Box::try_new_in(data, slab_allocator)
                .expect("Failed to allocate");
            assert_eq!(*allocated, data);

            Box::try_new_in(!data, slab_allocator)
                .expect_err("Should have failed to allocate");
        }

        let alloc = init_slab_alloc::<u8>(2 * mem::size_of::<u8>());
        let slab_allocator = &alloc.slab_allocator;

        // A single allocation should be available
        const DATA: u8 = 0xda;
        smallest_allocation_assert(DATA, slab_allocator);

        // Since the previous allocation was freed, a new one
        // should be available
        // A single allocation should be available
        smallest_allocation_assert(DATA, slab_allocator);
    }

    /// Ensures that a single `u64` can be manually allocated and deallocated
    #[test]
    fn single_manual_allocation() {
        let alloc = init_slab_alloc::<u64>(8 * mem::size_of::<u64>());
        let slab_allocator = alloc.slab_allocator;
        let layout = alloc.layout;

        // Manual allocation
        let allocated = slab_allocator
            .allocate(layout)
            .expect("Failed to allocate");
        let data = unsafe {
            allocated
                .cast::<u64>()
                .as_mut()
        };

        // Ensure it's initialized as 0
        const ZERO: u64 = 0;
        assert_eq!(*data, ZERO);

        // Ensure it gets set correctly
        const DATA: u64 = 0xdeadbeef;
        *data = DATA;
        assert_eq!(*data, DATA);

        // Manual deallocation
        unsafe { slab_allocator.deallocate(allocated.cast::<u8>(), layout) };

        // Ensure it is set to 0 when deallocated
        assert_eq!(*data, 0);
    }

    /// Ensures that a single `f32` can be automatically allocated and deallocated using a `Box`
    #[test]
    fn single_auto_allocation() {
        let alloc = init_slab_alloc::<f32>(8 * mem::size_of::<f32>());
        let slab_allocator = alloc.slab_allocator;

        // Ensure float allocation works
        const DATA: f32 = 3.14159;
        let data = Box::try_new_in(DATA, slab_allocator)
            .expect("Failed to allocate");

        // Ensure it gets set correctly
        assert_eq!(*data, DATA);
    }

    /// Ensures that the entire section of memory owned by the allocator can be used for allocations
    ///
    /// Tests this by giving an allocator enough memory to allocate 7 `u16`s and ensures that they can all be allocated.
    ///
    /// Also tests that an 8th allocation will fail because there is not enough memory.
    #[test]
    fn multiple_auto_allocation() {
        // Init allocator with space for 7 `u16`s and 1 byte for the bitmap
        const NUM_ALLOC: usize = 7;
        let alloc = init_slab_alloc::<u16>((NUM_ALLOC + 1) * mem::size_of::<u16>());
        let slab_allocator = alloc.slab_allocator;

        // This is inside a block so that the slab_allocator is not deallocated before its own allocations!
        {
            // Save allocations in a `Vec` so they are all deallocated at once
            let mut saved_allocations: Vec<Box<u16, SlabAllocator>> = vec![];

            // Fill allocator
            for i in 0..NUM_ALLOC {
                let alloc = Box::try_new_in(i as u16, slab_allocator)
                    .expect("Failed to allocate");
                saved_allocations.push(alloc);
            }

            // Ensure allocations are set correctly
            for i in 0..NUM_ALLOC {
                assert_eq!(i as u16, *saved_allocations[i]);
            }

            // This allocation is expected to fail because there should be no more room for allocations
            Box::try_new_in(9 as u16, slab_allocator)
                .expect_err("Should have failed to allocate");
        }
    }
}
