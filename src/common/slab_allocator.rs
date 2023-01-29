use core::{alloc::{Allocator, AllocError, Layout}, ptr::{NonNull, self}, slice};

/// Error cases when using a [`SlabAllocator`]
#[derive(Clone, Copy, Debug)]
pub enum SlabAllocatorError {
    /// Size is not divisible by 8
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
        self.size / self.layout.size()
    }

    /// Returns the size of the bitmap in bytes
    fn bitmap_size(&self) -> usize {
        const NUM_BITS: usize = 8;
        let full_bytes = self.bitmap_num_bits() / NUM_BITS;
        if self.bitmap_num_bits() % NUM_BITS != 0 {
            full_bytes + 1
        } else {
            full_bytes
        }
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
        self.size - self.bitmap_size()
    }

    /// Returns a pointer to the byte after the last byte in this allocator's storage
    unsafe fn end(&self) -> *const u8 {
        self.storage.add(self.size)
    }

    /// Initializes a new slab allocator for objects with this `layout`
    ///
    /// Returns [`SlabAllocatorError::InvalidSize`] if `size` is not divisible by `layout.size()` or if `storage` cannot
    /// store two or more objects (one for the bitmap)
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
    for i in 0..8 {
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
    assert!(free_bit < 8);
    1 << free_bit
}

unsafe impl Allocator for SlabAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Return error if layouts do not match
        if self.layout != layout {
            return Err(AllocError);
        }

        // Loop through each byte in bitmap to check for zeroed bits
        let bitmap = unsafe {
            self.bitmap()
        };
        for (i, bitmap_part) in bitmap.iter_mut().enumerate() {
            // Check if byte contains any zeroed bits
            if *bitmap_part < u8::MAX {
                // Get index of first free bit
                let free_bit = first_free_bit(*bitmap_part);
                // Get index for free memory location
                let free_index = i * 8 + free_bit;

                // Set bitmap to indicate that the memory location is now used
                *bitmap_part |= bit_mask(free_bit as u8);

                // Return a slice of the memory location
                unsafe {
                    let ptr = self.buffer_ptr().add(free_index * self.layout.size());
                    return Ok(NonNull::new(slice::from_raw_parts_mut(ptr as *mut u8, self.layout.size())).unwrap());
                }
            }
        }

        // No memory is available
        return Err(AllocError);
    }

    unsafe fn deallocate(&self, alloc_ptr: NonNull<u8>, layout: Layout) {
        // Assert that layouts match
        assert_eq!(self.layout, layout);

        // Assert pointer is in range
        let alloc_ptr = alloc_ptr.as_ptr() as *const u8;
        assert!(alloc_ptr >= self.buffer_ptr());
        assert!(alloc_ptr < self.end());

        // Calculate index of byte and bit in bitmap
        let offset = alloc_ptr.sub_ptr(self.buffer_ptr());
        let byte_idx = offset / self.layout.size();
        let free_bit = offset % self.layout.size();

        // Assert that the index is valid
        let bitmap = self.bitmap();
        assert!(byte_idx < bitmap.len());

        // Zero out bit in bitmap
        bitmap[byte_idx] &= !bit_mask(free_bit as u8);

        // Zero out freed memory
        ptr::write_bytes(alloc_ptr as *mut u8, 0, self.layout.size());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{vec, vec::Vec};

    /// Ensures that a single `u64` can be successfully allocated and deallocated
    #[test]
    fn main_test() {
        // Init allocator
        let storage: Vec<u8> = vec![0; 8 * 8];
        let layout = Layout::new::<u64>();
        let slab_allocator = unsafe {
            SlabAllocator::new(storage.as_ptr(), storage.len(), layout)
                .expect("Failed to create allocator")
        };

        // Allocate
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

        // Deallocate
        unsafe { slab_allocator.deallocate(allocated.cast::<u8>(), layout) };
        assert_eq!(*data, 0);
    }
}
