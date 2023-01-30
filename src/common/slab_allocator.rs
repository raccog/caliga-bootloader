use core::{alloc::{Allocator, AllocError, Layout}, ptr::{NonNull, self}, slice};
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
    layout: Layout,
}

impl SlabAllocator {
    /// Returns the bitmap used for keeping track of used memory
    unsafe fn bitmap(&self) -> &mut [u8] {
        slice::from_raw_parts_mut(self.bitmap_ptr() as *mut u8, self.bitmap_size())
    }

    /// Returns the bitmap used for keeping track of used memory
    unsafe fn bitmap_ptr(&self) -> *const u8 {
        self.storage
    }

    /// Returns the size of the bitmap in bits
    fn bitmap_num_bits(&self) -> usize {
        // Calculate how many bytes are taken up by the bitmap and its padding
        let num_bits = self.size / self.layout.size();
        let mut bitmap_size = num_bits / u8::BITS as usize;

        // Ensure that all bits are included
        if num_bits % u8::BITS as usize != 0 {
            bitmap_size += 1
        }

        // Ensure that padding is included
        let alignment = bitmap_size % self.layout.align();
        if alignment != 0 {
            bitmap_size += self.layout.align() - alignment;
        }

        // Final calculation using the real size of the bitmap
        (self.size - bitmap_size) / self.layout.size()
    }

    /// Returns the size of the bitmap in bytes
    fn bitmap_size(&self) -> usize {
        // Use previous bitmap size to get the real bitmap size
        let num_bits = self.bitmap_num_bits();
        let mut bitmap_size = num_bits / u8::BITS as usize;

        // Ensure all bits are included
        if num_bits % u8::BITS as usize != 0 {
            bitmap_size += 1
        }

        bitmap_size
    }

    /// Returns the buffer used for allocating objects
    unsafe fn buffer(&self) -> &mut [u8] {
        slice::from_raw_parts_mut(self.buffer_ptr() as *mut u8, self.buffer_size())
    }

    /// Returns the buffer used for allocating objects
    unsafe fn buffer_ptr(&self) -> *const u8 {
        let bitmap_end = self.storage.add(self.bitmap_size());
        let offset = bitmap_end.align_offset(self.layout.align());
        bitmap_end.add(offset)
    }

    /// Returns the size of the slab allocation buffer in bytes
    fn buffer_size(&self) -> usize {
        // Ensure that padding is included in the bitmap's size
        let mut bitmap_size = self.bitmap_size();
        let alignment = bitmap_size % self.layout.align();
        if alignment != 0 {
            bitmap_size += self.layout.align() - alignment;
        }

        self.size - bitmap_size
    }

    /// Returns a pointer to the byte after the last byte in this allocator's storage
    unsafe fn end(&self) -> *const u8 {
        self.storage.add(self.size)
    }

    /// Initializes a new slab allocator for objects with this `layout`
    ///
    /// # Errors
    ///
    /// Returns [`SlabAllocatorError::InvalidSize`] if `size` is not divisible by `layout.size()` or if `storage` cannot
    /// store two or more objects (at least one for the bitmap)
    pub unsafe fn new(storage: *const u8, size: usize, layout: Layout) -> Result<SlabAllocator, SlabAllocatorError> {
        // Return error if size is invalid
        let layout_size = layout.size();
        if size % layout_size != 0 || size < layout_size * 2 {
            return Err(SlabAllocatorError::InvalidSize);
        }

        // Init allocator fields
        let slab_allocator = SlabAllocator {
            storage, size, layout
        };

        // Zero all memory
        slab_allocator.bitmap().fill(0);
        slab_allocator.buffer().fill(0);

        // Mask bits for memory that is unavailable
        let available_bits = slab_allocator.bitmap_num_bits() % u8::BITS as usize;
        if available_bits != 0 {
            *slab_allocator.bitmap().last_mut().unwrap() = u8::MAX << available_bits;
        }

        debug!("{:#?} bitmap: {:#?} {:#?} buffer: {:#?} bitmap_bits: {:#?} buffer_size: {:#?}", slab_allocator, slab_allocator.bitmap_ptr(), slab_allocator.bitmap(), slab_allocator.buffer_ptr(), slab_allocator.bitmap_num_bits(),
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
// Panics if `byte` has no zeroed bits.
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

// Returns a mask for the `free_bit` index
//
// # Safety
//
// Panics if `free_bit` is greater than 7.
fn bit_mask(free_bit: u8) -> u8 {
    assert!(free_bit < u8::BITS as u8);
    1 << free_bit
}

unsafe impl Allocator for SlabAllocator {
    // Returns `AllocError` if:
    //
    // * `layout.align()` does not match this slab allocator's alignment
    // * There is no memory block large enough to allocate `layout.size()` sequential bytes
    //
    // NOTE: This currently will suffer from some memory fragmentation unless all allocations are the same size
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Return error if layouts do not match
        if self.layout != layout {
            return Err(AllocError);
        }

        // Find a large enough free block of memory using the bitmap
        let bitmap = unsafe {
            self.bitmap()
        };
        for (i, bitmap_part) in bitmap.iter_mut().enumerate() {
            // Check if this part of the bitmap contains any free memory
            if *bitmap_part < u8::MAX {
                // Get index of first free bit
                let free_bit = first_free_bit(*bitmap_part);
                // Get index for free memory location
                let free_index = i * u8::BITS as usize + free_bit;

                // Set bitmap to indicate that the memory location is now used
                *bitmap_part |= bit_mask(free_bit as u8);

                // Return a slice of the memory location
                unsafe {
                    let ptr = self.buffer_ptr().add(free_index * self.layout.size());
                    debug!("Alloc {:#?}", ptr);
                    return Ok(NonNull::new(slice::from_raw_parts_mut(ptr as *mut u8, self.layout.size())).unwrap());
                }
            }
        }

        // No memory is available
        return Err(AllocError);
    }

    unsafe fn deallocate(&self, alloc_ptr: NonNull<u8>, layout: Layout) {
        debug!("Dealloc {:#?}", alloc_ptr);
        // Ensure deallocation is valid
        let alloc_ptr = alloc_ptr.as_ptr() as *const u8;
        assert!(alloc_ptr >= self.buffer_ptr());
        assert!(alloc_ptr < self.end());
        assert_eq!(self.layout, layout);

        // Calculate index of byte and bit in bitmap
        let offset = alloc_ptr.sub_ptr(self.buffer_ptr());
        let index = offset / self.layout.size();
        let byte_idx = index / u8::BITS as usize;
        let bit_idx = index % u8::BITS as usize;

        // Ensure the index is invalid
        let bitmap = self.bitmap();
        assert!(byte_idx < bitmap.len());

        // Zero out bit in bitmap
        bitmap[byte_idx] &= !bit_mask(bit_idx as u8);

        // Zero out freed memory
        ptr::write_bytes(alloc_ptr as *mut u8, 0, self.layout.size());
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
